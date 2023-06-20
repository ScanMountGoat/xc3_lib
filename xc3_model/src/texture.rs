use std::{io::Cursor, path::Path};

use xc3_lib::{
    mibl::{Mibl, MiblFooter},
    msrd::Msrd,
    mxmd::Mxmd,
    xbc1::Xbc1,
};

// TODO: Indicate that this is for non maps?
pub fn load_textures(
    msrd: &Msrd,
    mxmd: &Mxmd,
    m_tex_folder: &Path,
    h_tex_folder: &Path,
) -> Vec<Mibl> {
    let cached_texture_data = msrd.extract_texture_data();

    // Assume the cached and non cached textures have the same ordering.
    mxmd.textures
        .items
        .as_ref()
        .unwrap()
        .textures
        .iter()
        .zip(msrd.texture_name_table.as_ref().unwrap().textures.iter())
        .map(|(item, cached_item)| {
            load_wismt_texture(m_tex_folder, h_tex_folder, &item.name).unwrap_or_else(|| {
                // Some textures only appear in the cache and have no high res version.
                load_cached_texture(&cached_texture_data, cached_item)
            })
        })
        .collect()
}

fn load_cached_texture(
    cached_texture_data: &[u8],
    cached_item: &xc3_lib::msrd::TextureInfo,
) -> Mibl {
    let data = &cached_texture_data
        [cached_item.offset as usize..cached_item.offset as usize + cached_item.size as usize];
    Mibl::read(&mut Cursor::new(&data)).unwrap()
}

fn load_wismt_texture(
    m_texture_folder: &Path,
    h_texture_folder: &Path,
    texture_name: &str,
) -> Option<Mibl> {
    // TODO: Create a helper function in xc3_lib for this?
    let xbc1 = Xbc1::from_file(m_texture_folder.join(texture_name).with_extension("wismt")).ok()?;
    let mut reader = Cursor::new(xbc1.decompress().unwrap());

    let mibl_m = Mibl::read(&mut reader).unwrap();

    let mut base_mip_level =
        Xbc1::from_file(&h_texture_folder.join(texture_name).with_extension("wismt"))
            .unwrap()
            .decompress()
            .unwrap();
    // TODO: Will this correctly handle alignment?
    base_mip_level.extend_from_slice(&mibl_m.image_data);

    // TODO: Is this the correct size calculation?
    let image_size = base_mip_level.len() as u32;

    // TODO: make merging mibl part of xc3_lib?
    Some(Mibl {
        image_data: base_mip_level,
        footer: MiblFooter {
            image_size,
            width: mibl_m.footer.width * 2,
            height: mibl_m.footer.height * 2,
            depth: mibl_m.footer.depth, // TODO: double depth?
            mipmap_count: mibl_m.footer.mipmap_count + 1,
            ..mibl_m.footer
        },
    })
}
