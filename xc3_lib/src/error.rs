//! Error types for read and write operations.
use std::path::PathBuf;

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

    #[error("stream index {index} out of range for length {count}")]
    MissingStream { index: usize, count: usize },
}

#[derive(Debug, Error)]
#[error("error reading {path:?}")]
pub struct ReadFileError {
    pub path: PathBuf,
    #[source]
    pub source: binrw::Error,
}

#[derive(Debug, Error)]
pub enum ExtractStreamFilesError {
    #[error("error decompressing stream")]
    Stream(#[from] DecompressStreamError),

    #[error("error reading chr/tex texture")]
    ChrTexTexture(#[from] ReadFileError),

    #[error("legacy streams do not contain all necessary data")]
    LegacyStream,
}

#[derive(Debug, Error)]
pub enum CreateXbc1Error {
    #[error("error reading or writing data")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum CreateMtxtError {
    #[error("error swizzling surface")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating surface from DDS")]
    DdsError(#[from] image_dds::error::SurfaceError),

    #[error("image format {0:?} is not supported by Mibl")]
    UnsupportedImageFormat(image_dds::ImageFormat),
}

#[derive(Debug, Error)]
pub enum CreateMiblError {
    #[error("error swizzling surface")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating surface from DDS")]
    DdsError(#[from] image_dds::error::SurfaceError),

    #[error("image format {0:?} is not supported by Mibl")]
    UnsupportedImageFormat(image_dds::ImageFormat),
}
