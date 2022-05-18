use bytes::Bytes;

#[derive(Debug)]
pub enum Lazy<T> {
    Pending(Bytes),
    Ready(T),
}
