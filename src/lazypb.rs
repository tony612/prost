use core::cell::RefCell;

use crate::Message;
use bytes::Bytes;
use lazypb::Lazy;

impl<M> Message for RefCell<Lazy<M>>
where
    M: Message,
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
        wire_type: crate::encoding::WireType,
        buf: &mut B,
        ctx: crate::encoding::DecodeContext,
    ) -> Result<(), crate::DecodeError>
    where
        B: bytes::Buf,
        Self: Sized,
    {
        let mut r = self.borrow_mut();
        match &mut *r {
            Lazy::Init => {
                let mut b = Bytes::new();
                crate::encoding::bytes::merge(wire_type, &mut b, buf, ctx)?;
                *r = Lazy::Pending(b);
                Ok(())
            }
            Lazy::Pending(b) => {
                crate::encoding::bytes::merge(wire_type, b, buf, ctx)?;
                Ok(())
            }
            Lazy::Ready(m) => {
                m.merge(buf)?;
                Ok(())
            }
        }

        // let len = decode_varint(buf)?;
        // let remaining = buf.remaining();
        // if len > remaining as u64 {
        //     return Err(DecodeError::new("buffer underflow"));
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
