use bytes::Bytes;

#[derive(Debug, PartialEq, Clone)]
pub enum Lazy<M> {
    Init,
    Pending(Bytes),
    Ready(M),
}

impl<M> Default for Lazy<M> {
    fn default() -> Self {
        return Lazy::Init;
    }
}
