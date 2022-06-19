use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;
use syn::Meta;

use crate::field::{set_bool, set_option, tag_attr, word_attr, Label};

#[derive(Clone)]
pub struct Field {
    pub label: Label,
    pub tag: u32,
    pub lazypb: Option<syn::Path>,
}

impl Field {
    pub fn new(attrs: &[Meta], inferred_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let mut message = false;
        let mut label = None;
        let mut tag = None;
        let mut boxed = false;
        #[allow(unused_mut)]
        let mut lazypb = None;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("lazypb") {
                #[cfg(feature = "lazypb")]
                {
                    let t = match *attr {
                        Meta::NameValue(syn::MetaNameValue {
                            lit: syn::Lit::Str(ref lit),
                            ..
                        }) => syn::parse_str::<syn::Path>(&lit.value())?,
                        Meta::List(ref list) if list.nested.len() == 1 => {
                            if let syn::NestedMeta::Meta(Meta::Path(ref path)) = list.nested[0] {
                                if let Some(ident) = path.get_ident() {
                                    syn::Path::from(ident.clone())
                                } else {
                                    bail!("invalid lazypb inner type: item must be an identifier");
                                }
                            } else {
                                bail!("invalid lazypb inner type: item must be an identifier");
                            }
                        }
                        _ => bail!("invalid lazypb inner type: {:?}", attr),
                    };
                    set_option(&mut lazypb, t, "duplicate lazypb attribute")?;
                }
            } else if word_attr("message", attr) {
                set_bool(&mut message, "duplicate message attribute")?;
            } else if word_attr("boxed", attr) {
                set_bool(&mut boxed, "duplicate boxed attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !message {
            return Ok(None);
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!(
                "unknown attribute for message field: {:?}",
                unknown_attrs[0]
            ),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        let tag = match tag.or(inferred_tag) {
            Some(tag) => tag,
            None => bail!("message field is missing a tag attribute"),
        };

        Ok(Some(Field {
            label: label.unwrap_or(Label::Optional),
            tag,
            lazypb,
        }))
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(attrs, None)? {
            if let Some(attr) = attrs.iter().find(|attr| Label::from_attr(attr).is_some()) {
                bail!(
                    "invalid attribute for oneof field: {}",
                    attr.path().into_token_stream()
                );
            }
            field.label = Label::Required;
            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                if let Some(ref msg) = #ident {
                    ::prost::encoding::message::encode(#tag, msg, buf);
                }
            },
            Label::Required => quote! {
                ::prost::encoding::message::encode(#tag, &#ident, buf);
            },
            Label::Repeated => quote! {
                for msg in &#ident {
                    ::prost::encoding::message::encode(#tag, msg, buf);
                }
            },
        }
    }

    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => {
                #[cfg(feature = "lazypb")]
                if self.lazypb.is_some() {
                    quote! {
                        ::prost::lazypb::message::merge(wire_type,
                                                         #ident.get_or_insert_with(::core::default::Default::default),
                                                         buf,
                                                         ctx)
                    }
                } else {
                    quote! {
                        ::prost::encoding::message::merge(wire_type,
                                                         #ident.get_or_insert_with(::core::default::Default::default),
                                                         buf,
                                                         ctx)
                    }
                }
                #[cfg(not(feature = "lazypb"))]
                quote! {
                    ::prost::encoding::message::merge(wire_type,
                                                     #ident.get_or_insert_with(::core::default::Default::default),
                                                     buf,
                                                     ctx)
                }
            }
            Label::Required => quote! {
                ::prost::encoding::message::merge(wire_type, #ident, buf, ctx)
            },
            Label::Repeated => quote! {
                ::prost::encoding::message::merge_repeated(wire_type, #ident, buf, ctx)
            },
        }
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                #ident.as_ref().map_or(0, |msg| ::prost::encoding::message::encoded_len(#tag, msg))
            },
            Label::Required => quote! {
                ::prost::encoding::message::encoded_len(#tag, &#ident)
            },
            Label::Repeated => quote! {
                ::prost::encoding::message::encoded_len_repeated(#tag, &#ident)
            },
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => quote!(#ident = ::core::option::Option::None),
            Label::Required => quote!(#ident.clear()),
            Label::Repeated => quote!(#ident.clear()),
        }
    }

    #[cfg(not(feature = "lazypb"))]
    pub fn methods(&self, _ident: &Ident) -> Option<TokenStream> {
        None
    }

    #[cfg(feature = "lazypb")]
    pub fn methods(&self, ident: &Ident) -> Option<TokenStream> {
        match self.lazypb {
            Some(ref ty) => {
                use proc_macro2::Span;
                let mut ident_str = ident.to_string();
                if ident_str.starts_with("r#") {
                    ident_str = ident_str[2..].to_owned();
                }

                let get_doc = format!(
                    "Returns the value of lazy `{0}`. The pending bytes will be decoded when needed.",
                    ident_str,
                );
                let get_method = Ident::new(&format!("get_{}", ident_str), Span::call_site());
                if self.label == Label::Optional {
                    Some(quote! {
                                    #[doc=#get_doc]
                                    pub fn #get_method(&self) -> ::core::result::Result<::core::option::Option<::core::cell::Ref<'_, #ty>>, ::prost::DecodeError> {
                    match self.#ident {
                        Some(ref val) => {
                            let val_ref = val.borrow();
                            match *val_ref {
                                ::lazypb::Lazy::Ready(_) => Ok(Some(::core::cell::Ref::map(val_ref, |x| match x {
                                    ::lazypb::Lazy::Ready(x1) => x1,
                                    _ => unreachable!(),
                                }))),
                                ::lazypb::Lazy::Pending(_) => {
                                    // drop val_ref to prevent BorrowMutError
                                    drop(val_ref);
                                    let mut v_mut = val.borrow_mut();

                                    match *v_mut {
                                        ::lazypb::Lazy::Pending(ref mut b) => {
                                            #[allow(unused_imports)]
                                            use ::prost::Message;
                                            *v_mut = ::lazypb::Lazy::Ready(#ty::decode(b)?);
                                        }
                                        _ => unreachable!(),
                                    }
                                    drop(v_mut);

                                    Ok(Some(::core::cell::Ref::map(val.borrow(), |x| match x {
                                        ::lazypb::Lazy::Ready(x1) => x1,
                                        _ => unreachable!(),
                                    })))
                                }
                                ::lazypb::Lazy::Init => Ok(None),
                            }
                        }
                        _ => Ok(None),
                    }
                                    }
                                })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
