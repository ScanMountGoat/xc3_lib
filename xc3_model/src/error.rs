use thiserror::Error;
use xc3_lib::error::{DecompressStreamError, ExtractStreamFilesError, ReadFileError};

#[derive(Debug, Error)]
pub enum LoadModelError {
    #[error("error reading wimdo file")]
    Wimdo(#[source] ReadFileError),

    #[error("error extracting texture from wimdo file")]
    WimdoPackedTexture {
        #[source]
        source: binrw::Error,
    },

    #[error("error reading vertex data")]
    VertexData(binrw::Error),

    #[error("failed to find Mxmd in Apmd file")]
    MissingApmdMxmdEntry,

    #[error("expected packed wimdo vertex data but found none")]
    MissingMxmdVertexData,

    #[error("expected packed wimdo shader data but found none")]
    MissingMxmdShaderData,

    #[error("error loading image texture")]
    Image(#[from] CreateImageTextureError),

    #[error("error decompressing stream")]
    Stream(#[from] DecompressStreamError),

    #[error("error extracting stream data")]
    ExtractFiles(#[from] ExtractStreamFilesError),

    #[error("error reading legacy wismt streaming file")]
    WismtLegacy(#[source] ReadFileError),

    #[error("error reading wismt streaming file")]
    Wismt(#[source] ReadFileError),
}

#[derive(Debug, Error)]
pub enum LoadModelLegacyError {
    #[error("error reading camdo file")]
    Camdo(#[source] ReadFileError),

    #[error("error reading vertex data")]
    VertexData(binrw::Error),

    #[error("error loading image texture")]
    Image(#[from] CreateImageTextureError),

    #[error("error reading casmt streaming file")]
    Casmt(#[source] std::io::Error),
}

#[derive(Debug, Error)]
pub enum CreateModelError {
    #[error("error extracting stream data")]
    ExtractFiles(#[from] ExtractStreamFilesError),
}

#[derive(Debug, Error)]
pub enum LoadCollisionsError {
    #[error("error reading idcm streaming file")]
    Idcm(#[from] ReadFileError),
}

// TODO: Add more error variants.
#[cfg(feature = "gltf")]
#[derive(Debug, Error)]
pub enum CreateGltfError {
    #[error("error writing buffers")]
    Binrw(#[from] binrw::Error),
}

#[cfg(feature = "gltf")]
#[derive(Debug, Error)]
pub enum SaveGltfError {
    #[error("error writing files")]
    Io(#[from] std::io::Error),

    #[error("error serializing JSON file")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum LoadMapError {
    #[error("error reading data")]
    Io(#[from] std::io::Error),

    #[error("error reading wismhd file")]
    Wismhd(#[source] ReadFileError),

    #[error("error reading data")]
    Binrw(#[from] binrw::Error),

    #[error("error loading image texture")]
    Image(#[from] CreateImageTextureError),

    #[error("error decompressing stream")]
    Stream(#[from] DecompressStreamError),
}

#[derive(Debug, Error)]
pub enum LoadShaderDatabaseError {
    #[error("error reading shader file")]
    Io(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum SaveShaderDatabaseError {
    #[error("error writing shader file")]
    Io(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum CreateImageTextureError {
    #[error("error deswizzling surface")]
    SwizzleMibl(#[from] xc3_lib::mibl::SwizzleError),

    #[error("error deswizzling surface")]
    SwizzleMtxt(#[from] xc3_lib::mtxt::SwizzleError),

    #[error("error reading data")]
    Binrw(#[from] binrw::Error),

    #[error("error decompressing stream")]
    Stream(#[from] DecompressStreamError),

    #[error("error converting image surface")]
    Surface(#[from] image_dds::error::SurfaceError),

    #[error("error converting Mibl texture")]
    Mibl(#[from] xc3_lib::error::CreateMiblError),
}
