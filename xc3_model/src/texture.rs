use std::{error::Error, path::Path};

use ddsfile::Dds;
use xc3_lib::{
    mibl::Mibl,
    msrd::Msrd,
    mxmd::{Mxmd, PackedTexture},
    xbc1::Xbc1,
};

pub use xc3_lib::mibl::{ImageFormat, ViewDimension};

/// A non swizzled version of an [Mibl] texture.
#[derive(Debug, Clone)]
pub struct ImageTexture {
    pub name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub view_dimension: ViewDimension,
    pub image_format: ImageFormat,
    pub mipmap_count: u32,
    pub image_data: Vec<u8>,
}

impl ImageTexture {
    /// Deswizzle the data from `mibl`.
    /// The `name` is not required but creates more descriptive file names and debug information.
    pub fn from_mibl(
        mibl: &Mibl,
        name: Option<String>,
    ) -> Result<Self, tegra_swizzle::SwizzleError> {
        Ok(Self {
            name,
            width: mibl.footer.width,
            height: mibl.footer.height,
            depth: mibl.footer.depth,
            view_dimension: mibl.footer.view_dimension,
            image_format: mibl.footer.image_format,
            mipmap_count: mibl.footer.mipmap_count,
            image_data: mibl.deswizzled_image_data()?,
        })
    }

    pub fn from_packed_texture(texture: &PackedTexture) -> Self {
        let mibl = Mibl::from_bytes(&texture.mibl_data);
        Self::from_mibl(&mibl, Some(texture.name.clone())).unwrap()
    }
}

// TODO: Indicate that this is for non maps?
// TODO: Create unit tests for this?
pub fn load_textures(
    mxmd: &Mxmd,
    msrd: Option<&Msrd>,
    m_tex_folder: &Path,
    h_tex_folder: &Path,
) -> Vec<ImageTexture> {
    // TODO: packed mxmd, external mxmd, low res msrd, msrd,
    // TODO: Is this the correct way to handle this?
    // TODO: Is it possible to have both packed and external mxmd textures?
    if let Some(textures) = &mxmd.textures {
        let mxmd_textures = match &textures.inner {
            xc3_lib::mxmd::TexturesInner::Unk0(t) => Some(&t.textures1.textures),
            xc3_lib::mxmd::TexturesInner::Unk1(t) => t.textures.as_ref().map(|t| &t.textures),
        };

        let packed_texture_data = msrd.unwrap().extract_low_texture_data();
        // TODO: These textures aren't in the same order?
        let middle_textures = msrd.unwrap().extract_middle_textures();

        // TODO: Same as msrd?
        let texture_ids = &msrd.as_ref().unwrap().texture_ids;

        // Assume the packed and non packed textures have the same ordering.
        // Xenoblade 3 has some textures in the chr/tex folder.
        // TODO: Are the mxmd and msrd packed texture lists always identical?
        mxmd_textures
            .map(|packed_textures| {
                packed_textures
                    .iter()
                    .enumerate()
                    .map(|(i, texture)| {
                        load_wismt_texture(m_tex_folder, h_tex_folder, &texture.name)
                            .or_else(|| {
                                // TODO: Assign in a second pass to avoid O(N) find.
                                texture_ids
                                    .iter()
                                    .position(|id| *id as usize == i)
                                    .and_then(|index| {
                                        middle_textures.get(index).map(|mibl| {
                                            ImageTexture::from_mibl(
                                                mibl,
                                                Some(texture.name.clone()),
                                            )
                                            .unwrap()
                                        })
                                    })
                            })
                            .unwrap_or_else(|| {
                                // Some textures only appear in the packed textures and have no high res version.
                                load_packed_texture(&packed_texture_data, texture)
                            })
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else if let Some(packed_textures) = &mxmd.packed_textures {
        packed_textures
            .textures
            .iter()
            .map(ImageTexture::from_packed_texture)
            .collect()
    } else {
        // TODO: How to handle this case?
        Vec::new()
    }
}

fn load_packed_texture(
    packed_texture_data: &[u8],
    item: &xc3_lib::mxmd::PackedExternalTexture,
) -> ImageTexture {
    let data = &packed_texture_data
        [item.mibl_offset as usize..item.mibl_offset as usize + item.mibl_length as usize];

    let mibl = Mibl::from_bytes(data);
    ImageTexture::from_mibl(&mibl, Some(item.name.clone())).unwrap()
}

fn load_wismt_texture(
    m_texture_folder: &Path,
    h_texture_folder: &Path,
    texture_name: &str,
) -> Option<ImageTexture> {
    // TODO: Create a helper function in xc3_lib for this?
    let xbc1 = Xbc1::from_file(m_texture_folder.join(texture_name).with_extension("wismt")).ok()?;
    let mibl_m = Mibl::from_bytes(&xbc1.decompress().unwrap());

    let base_mip_level =
        Xbc1::from_file(&h_texture_folder.join(texture_name).with_extension("wismt"))
            .unwrap()
            .decompress()
            .unwrap();

    Some(merge_mibl(
        base_mip_level,
        mibl_m,
        Some(texture_name.to_string()),
    ))
}

pub fn merge_mibl(base_mip_level: Vec<u8>, mibl_m: Mibl, name: Option<String>) -> ImageTexture {
    let width = mibl_m.footer.width * 2;
    let height = mibl_m.footer.height * 2;
    // TODO: double depth?
    let depth = mibl_m.footer.depth;

    // The high resolution texture is only the base level.
    let mipmap_count = 1;

    // TODO: move to xc3_lib?
    let mut image_data = tegra_swizzle::surface::deswizzle_surface(
        width as usize,
        height as usize,
        depth as usize,
        &base_mip_level,
        mibl_m.footer.image_format.block_dim(),
        None,
        mibl_m.footer.image_format.bytes_per_pixel(),
        mipmap_count,
        if mibl_m.footer.view_dimension == ViewDimension::Cube {
            6
        } else {
            1
        },
    )
    .unwrap();

    // Non swizzled data has no alignment requirements.
    // We can just combine the two surfaces.
    image_data.extend_from_slice(&mibl_m.deswizzled_image_data().unwrap());

    ImageTexture {
        name,
        width,
        height,
        depth,
        view_dimension: mibl_m.footer.view_dimension,
        image_format: mibl_m.footer.image_format,
        mipmap_count: mibl_m.footer.mipmap_count + 1,
        image_data,
    }
}

// TODO: add conversions to and from dds for surface to image_dds?
impl ImageTexture {
    pub fn to_image(&self) -> Result<image::RgbaImage, Box<dyn Error>> {
        let dds = self.to_dds()?;
        image_dds::image_from_dds(&dds, 0).map_err(Into::into)
    }

    pub fn to_dds(&self) -> Result<Dds, Box<dyn Error>> {
        let mut dds = Dds::new_dxgi(ddsfile::NewDxgiParams {
            height: self.height,
            width: self.width,
            depth: if self.depth > 1 {
                Some(self.depth)
            } else {
                None
            },
            format: self.image_format.into(),
            mipmap_levels: if self.mipmap_count > 1 {
                Some(self.mipmap_count)
            } else {
                None
            },
            array_layers: if self.view_dimension == ViewDimension::Cube {
                Some(6)
            } else {
                None
            },
            caps2: None,
            is_cubemap: false,
            resource_dimension: if self.depth > 1 {
                ddsfile::D3D10ResourceDimension::Texture3D
            } else {
                ddsfile::D3D10ResourceDimension::Texture2D
            },
            alpha_mode: ddsfile::AlphaMode::Straight, // TODO: Does this matter?
        })?;

        dds.data = self.image_data.clone();

        Ok(dds)
    }
}
