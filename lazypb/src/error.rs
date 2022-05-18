use std::result;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // Decoding
    #[error("EOF when reading")]
    EOF,
    #[error("field {0} is borrowed before get, so can't decode in get")]
    BorrowBeforeGet(String),
}

pub type Result<T> = result::Result<T, Error>;
