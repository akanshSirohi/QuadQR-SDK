use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuadQrError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("payload does not fit selected QuadQR version/profile")]
    Capacity,
    #[error("invalid QuadQR structure: {0}")]
    Structure(String),
    #[error("Reed-Solomon error: {0}")]
    ReedSolomon(String),
    #[error("QuadQR decode failed: {0}")]
    Decode(String),
    #[error("compression error: {0}")]
    Compression(String),
    #[error("security error: {0}")]
    Security(String),
    #[cfg(feature = "image")]
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, QuadQrError>;

pub(crate) fn invalid(message: impl Into<String>) -> QuadQrError {
    QuadQrError::InvalidArgument(message.into())
}
