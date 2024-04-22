//! Shared error types for read and write operations.
use thiserror::Error;
use zune_inflate::errors::InflateDecodeErrors;

#[derive(Debug, Error)]
pub enum DecompressStreamError {
    #[error("error decoding compressed stream")]
    ZLib(#[from] InflateDecodeErrors),

    #[error("error reading stream data")]
    Io(#[from] std::io::Error),

    #[error("error reading stream data")]
    Binrw(#[from] binrw::Error),

    #[error("checksum verification failed")]
    Checksum(Vec<u8>),
}
