use std::{error::Error, io::Cursor, path::Path};

use ddsfile::Dds;
use xc3_lib::{
    mibl::{ImageFormat, Mibl, ViewDimension},
    msrd::Msrd,
    mxmd::Mxmd,
    xbc1::Xbc1,
};

// TODO: Store a texture name as well?
/// A non swizzled version of an [Mibl] texture.
#[derive(Debug, Clone)]

pub struct ImageTexture {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub view_dimension: ViewDimension,
    pub image_format: ImageFormat,
    pub mipmap_count: u32,
    pub image_data: Vec<u8>,
}

impl TryFrom<&Mibl> for ImageTexture {
    type Error = tegra_swizzle::SwizzleError;

    fn try_from(mibl: &Mibl) -> Result<Self, Self::Error> {
        Ok(Self {
            width: mibl.footer.width,
            height: mibl.footer.height,
            depth: mibl.footer.depth,
            view_dimension: mibl.footer.view_dimension,
            image_format: mibl.footer.image_format,
            mipmap_count: mibl.footer.mipmap_count,
            image_data: mibl.deswizzled_image_data()?,
        })
    }
}

impl TryFrom<Mibl> for ImageTexture {
    type Error = tegra_swizzle::SwizzleError;

    fn try_from(mibl: Mibl) -> Result<Self, Self::Error> {
        Self::try_from(&mibl)
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
        let packed_textures = match &textures.inner {
            xc3_lib::mxmd::TexturesInner::Unk0(t) => &t.textures1.textures,
            xc3_lib::mxmd::TexturesInner::Unk1(t) => &t.textures.as_ref().unwrap().textures,
        };

        let packed_texture_data = msrd.unwrap().extract_low_texture_data();
        let middle_textures = msrd.unwrap().extract_middle_textures();

        // Assume the packed and non packed textures have the same ordering.
        // Xenoblade 3 has some textures in the chr/tex folder.
        packed_textures
            .iter()
            .zip(msrd.unwrap().textures.as_ref().unwrap().textures.iter())
            .enumerate()
            .map(|(i, (item, packed_item))| {
                load_wismt_texture(m_tex_folder, h_tex_folder, &item.name)
                    .or_else(|| middle_textures.get(i).map(|t| t.try_into().unwrap()))
                    .unwrap_or_else(|| {
                        // Some textures only appear in the packed textures and have no high res version.
                        load_packed_texture(&packed_texture_data, packed_item)
                    })
            })
            .collect()
    } else if let Some(packed_textures) = &mxmd.packed_textures {
        packed_textures
            .textures
            .iter()
            .map(|t| {
                Mibl::read(&mut Cursor::new(&t.mibl_data))
                    .unwrap()
                    .try_into()
                    .unwrap()
            })
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
    Mibl::read(&mut Cursor::new(&data))
        .unwrap()
        .try_into()
        .unwrap()
}

fn load_wismt_texture(
    m_texture_folder: &Path,
    h_texture_folder: &Path,
    texture_name: &str,
) -> Option<ImageTexture> {
    // TODO: Create a helper function in xc3_lib for this?
    let xbc1 = Xbc1::from_file(m_texture_folder.join(texture_name).with_extension("wismt")).ok()?;
    let mut reader = Cursor::new(xbc1.decompress().unwrap());

    let mibl_m = Mibl::read(&mut reader).unwrap();

    let base_mip_level =
        Xbc1::from_file(&h_texture_folder.join(texture_name).with_extension("wismt"))
            .unwrap()
            .decompress()
            .unwrap();

    Some(merge_mibl(base_mip_level, mibl_m))
}

pub fn merge_mibl(base_mip_level: Vec<u8>, mibl_m: Mibl) -> ImageTexture {
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
