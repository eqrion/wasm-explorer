use std::fmt;

/// Errors returned by waside operations.
#[derive(Debug)]
pub enum Error {
    /// An error from the underlying wasmparser binary reader.
    BinaryReader(wasmparser::BinaryReaderError),
    /// An error from the wasm-encoder re-encoding pass.
    Encoding(wasm_encoder::reencode::Error),
    /// A generic error with a message.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BinaryReader(e) => write!(f, "{e}"),
            Error::Encoding(e) => write!(f, "{e}"),
            Error::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::BinaryReader(e) => Some(e),
            Error::Encoding(e) => Some(e),
            Error::Other(_) => None,
        }
    }
}

impl From<wasmparser::BinaryReaderError> for Error {
    fn from(e: wasmparser::BinaryReaderError) -> Self {
        Error::BinaryReader(e)
    }
}

impl From<wasm_encoder::reencode::Error> for Error {
    fn from(e: wasm_encoder::reencode::Error) -> Self {
        Error::Encoding(e)
    }
}

/// A [`std::result::Result`] type with [`Error`] as the error variant.
pub type Result<T> = std::result::Result<T, Error>;
