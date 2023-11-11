use thiserror::Error;
use zune_inflate::errors::InflateDecodeErrors;

#[derive(Debug, Error)]
pub enum DecompressStreamError {
    #[error("error decoding compressed stream: {0}")]
    ZLib(#[from] InflateDecodeErrors),

    #[error("error reading data: {0}")]
    Io(#[from] std::io::Error),

    #[error("error reading data: {0}")]
    Binrw(#[from] binrw::Error),
}
