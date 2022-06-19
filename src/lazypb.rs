use core::cell::RefCell;

use crate::Message;
use ::lazypb::Lazy;

pub mod message {
    use core::cell::RefCell;

    use crate::encoding;
    use crate::DecodeError;
    use crate::Message;

    use ::bytes::Buf;
    use ::lazypb::Lazy;

    pub fn merge<M, B>(
        wire_type: encoding::WireType,
        msg: &mut RefCell<Lazy<M>>,
        buf: &mut B,
        ctx: encoding::DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Message + Default,
        B: Buf,
    {
        encoding::check_wire_type(encoding::WireType::LengthDelimited, wire_type)?;
        ctx.limit_reached()?;
        let len = encoding::decode_varint(buf)?;
        let remaining = buf.remaining();
        if len > remaining as u64 {
            return Err(DecodeError::new("buffer underflow"));
        }
        // When buf is small and is not Bytes, coping and malloc will be slower than parsing.
        // So parse it directly when it's small.
        if len > 32 {
            let dst = buf.copy_to_bytes(len as usize);
            let mut r = msg.borrow_mut();
            match &mut *r {
                Lazy::Init => {
                    *r = Lazy::Pending(dst);
                }
                Lazy::Pending(b) => {
                    if b.len() == 0 {
                        *b = dst;
                    } else {
                        let origin_len = b.remaining();
                        *b = b.chain(dst).copy_to_bytes(origin_len + len as usize);
                    }
                }
                Lazy::Ready(m) => {
                    m.merge(dst)?;
                }
            }
        } else {
            let limit = remaining - len as usize;
            let ctx = ctx.enter_recursion();
            let mut r = msg.borrow_mut();

            match &mut *r {
                Lazy::Init => {
                    let mut m = M::default();
                    while buf.remaining() > limit {
                        let (tag, wire_type) = encoding::decode_key(buf)?;
                        m.merge_field(tag, wire_type, buf, ctx.clone())?;
                    }
                    *r = Lazy::Ready(m);
                }
                Lazy::Pending(b) => {
                    let mut m;
                    if b.len() == 0 {
                        m = M::default();
                    } else {
                        m = M::decode(b)?;
                    }
                    while buf.remaining() > limit {
                        let (tag, wire_type) = encoding::decode_key(buf)?;
                        m.merge_field(tag, wire_type, buf, ctx.clone())?;
                    }
                    *r = Lazy::Ready(m);
                }
                Lazy::Ready(m) => {
                    while buf.remaining() > limit {
                        let (tag, wire_type) = encoding::decode_key(buf)?;
                        m.merge_field(tag, wire_type, buf, ctx.clone())?;
                    }
                }
            }

            if buf.remaining() != limit {
                return Err(DecodeError::new("delimited length exceeded"));
            }
        }
        Ok(())
    }
}

impl<M> Message for RefCell<Lazy<M>>
where
    M: Message + Default,
{
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: bytes::BufMut,
        Self: Sized,
    {
        let r = self.borrow();
        match &*r {
            Lazy::Init => {}
            Lazy::Pending(b) => {
                buf.put(b.clone());
            }
            Lazy::Ready(m) => {
                m.encode_raw(buf);
            }
        }
    }

    fn merge_field<B>(
        &mut self,
        _tag: u32,
        _wire_type: crate::encoding::WireType,
        _buf: &mut B,
        _ctx: crate::encoding::DecodeContext,
    ) -> Result<(), crate::DecodeError>
    where
        B: bytes::Buf,
        Self: Sized,
    {
        unreachable!("RefCell<Lazy<M>> merge_field should not be called");
        // let mut r = self.borrow_mut();
        // match &mut *r {
        //     Lazy::Init => {
        //         let mut m = M::default();
        //         m.merge_field(tag, wire_type, buf, ctx)?;
        //         *r = Lazy::Ready(m);
        //         Ok(())
        //     }
        //     Lazy::Pending(_b) => {
        //         todo!("decode pending bytes then merge field OR append bytes to the pending bytes");
        //     }
        //     Lazy::Ready(m) => {
        //         m.merge_field(tag, wire_type, buf, ctx)?;
        //         Ok(())
        //     }
        // }
    }

    fn encoded_len(&self) -> usize {
        let r = self.borrow();
        match &*r {
            Lazy::Init => 0,
            Lazy::Pending(b) => b.len(),
            Lazy::Ready(m) => m.encoded_len(),
        }
    }

    fn clear(&mut self) {
        self.replace(Lazy::Init);
    }
}
