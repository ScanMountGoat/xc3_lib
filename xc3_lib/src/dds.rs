//! Utilities for [Dds] image files.
use std::{io::Cursor, path::Path};

use image_dds::ddsfile::Dds;
use thiserror::Error;

pub trait DdsExt: Sized {
    type Error;

    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Self::Error>;
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Self::Error>;
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Self::Error>;
}

impl DdsExt for Dds {
    type Error = image_dds::ddsfile::Error;

    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Self::Error> {
        Self::read(&mut Cursor::new(bytes))
    }

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Self::Error> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        Dds::read(&mut reader)
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Self::Error> {
        let mut writer = Cursor::new(Vec::new());
        self.write(&mut writer)?;
        std::fs::write(path, writer.into_inner()).map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum CreateDdsError {
    #[error("error deswizzling surface")]
    SwizzleError(#[from] tegra_swizzle::SwizzleError),

    #[error("error creating DDS")]
    DdsError(#[from] image_dds::CreateDdsError),
}
