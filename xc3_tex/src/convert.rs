use std::{io::Cursor, path::Path};

use anyhow::Context;
use binrw::BinRead;
use image_dds::{ddsfile::Dds, image::RgbaImage, ImageFormat, Mipmaps, Quality, Surface};
use xc3_lib::{
    bmn::Bmn,
    dds::DdsExt,
    dhal::Dhal,
    lagp::Lagp,
    mibl::Mibl,
    msrd::{
        streaming::{chr_tex_nx_folder, HighTexture},
        Msrd,
    },
    mtxt::Mtxt,
    mxmd::Mxmd,
    xbc1::{CompressionType, MaybeXbc1, Xbc1},
};

// TODO: Support apmd?
pub enum File {
    Mibl(Mibl),
    Mtxt(Mtxt),
    Dds(Dds),
    Image(RgbaImage),
    Wilay(MaybeXbc1<Wilay>),
    Wimdo(Box<Mxmd>),
    Bmn(Bmn),
}

// TODO: Move this to xc3_lib?
#[derive(BinRead)]
pub enum Wilay {
    Dhal(Dhal),
    Lagp(Lagp),
}

impl File {
    pub fn to_dds(
        &self,
        format: Option<ImageFormat>,
        quality: Option<Quality>,
        mipmaps: bool,
    ) -> anyhow::Result<Dds> {
        match self {
            File::Mibl(mibl) => mibl
                .to_dds()
                .with_context(|| "failed to convert Mibl to DDS"),
            File::Mtxt(mtxt) => mtxt
                .to_dds()
                .with_context(|| "failed to convert Mtxt to DDS"),
            File::Dds(dds) => {
                // Handle changes in image format while preserving layers and mipmaps.
                // TODO: dds doesn't implement clone?
                match format {
                    Some(format) => Surface::from_dds(dds)?
                        .decode_rgba8()?
                        .encode(
                            format,
                            quality.unwrap_or(Quality::Normal),
                            if mipmaps {
                                Mipmaps::GeneratedAutomatic
                            } else {
                                Mipmaps::Disabled
                            },
                        )?
                        .to_dds()
                        .with_context(|| "failed to convert surface to DDS"),
                    None => Ok(Dds {
                        header: dds.header.clone(),
                        header10: dds.header10.clone(),
                        data: dds.data.clone(),
                    }),
                }
            }
            File::Image(image) => image_dds::dds_from_image(
                image,
                format.ok_or(anyhow::anyhow!("missing required image output format"))?,
                quality.unwrap_or(Quality::Normal),
                if mipmaps {
                    Mipmaps::GeneratedAutomatic
                } else {
                    Mipmaps::Disabled
                },
            )
            .with_context(|| "failed to encode image to DDS"),
            File::Wilay(_) => Err(anyhow::anyhow!(
                "wilay textures must be saved to an output folder instead of a single image"
            )),
            File::Wimdo(_) => Err(anyhow::anyhow!(
                "wimdo textures must be saved to an output folder instead of a single image"
            )),
            File::Bmn(_) => Err(anyhow::anyhow!(
                "bmn textures must be saved to an output folder instead of a single image"
            )),
        }
    }

    pub fn to_mibl(
        &self,
        format: Option<ImageFormat>,
        quality: Option<Quality>,
        mipmaps: bool,
    ) -> anyhow::Result<Mibl> {
        // TODO: decode and encode again if needed.
        match self {
            File::Mibl(mibl) => Ok(mibl.clone()),
            File::Mtxt(mtxt) => Mibl::from_surface(mtxt.to_surface())
                .with_context(|| "failed to convert Mtxt to Mibl"),
            File::Dds(dds) => Mibl::from_dds(dds).with_context(|| "failed to create Mibl from DDS"),
            File::Image(image) => {
                let dds = image_dds::dds_from_image(
                    image,
                    format.ok_or(anyhow::anyhow!("missing required image output format"))?,
                    quality.unwrap_or(Quality::Normal),
                    if mipmaps {
                        Mipmaps::GeneratedAutomatic
                    } else {
                        Mipmaps::Disabled
                    },
                )
                .with_context(|| "failed to create encode image to DDS")?;

                Mibl::from_dds(&dds)
                    .with_context(|| "failed to create Mibl from image encoded to DDS")
            }
            File::Wilay(_) => Err(anyhow::anyhow!(
                "wilay textures must be saved to an output folder instead of a single image"
            )),
            File::Wimdo(_) => Err(anyhow::anyhow!(
                "wimdo textures must be saved to an output folder instead of a single image"
            )),
            File::Bmn(_) => Err(anyhow::anyhow!(
                "bmn textures must be saved to an output folder instead of a single image"
            )),
        }
    }

    pub fn to_image(&self) -> anyhow::Result<RgbaImage> {
        match self {
            File::Mibl(mibl) => image_dds::image_from_dds(&mibl.to_dds()?, 0)
                .with_context(|| "failed to decode Mibl image"),
            File::Mtxt(mtxt) => image_dds::image_from_dds(&mtxt.to_dds()?, 0)
                .with_context(|| "failed to decode Mtxt image"),
            File::Dds(dds) => {
                image_dds::image_from_dds(dds, 0).with_context(|| "failed to decode DDS")
            }
            File::Image(image) => Ok(image.clone()),
            File::Wilay(_) => Err(anyhow::anyhow!(
                "wilay textures must be saved to an output folder instead of a single image"
            )),
            File::Wimdo(_) => Err(anyhow::anyhow!(
                "wimdo textures must be saved to an output folder instead of a single image"
            )),
            File::Bmn(_) => Err(anyhow::anyhow!(
                "bmn textures must be saved to an output folder instead of a single image"
            )),
        }
    }
}

pub fn update_wilay_from_folder(
    input: &str,
    input_folder: &str,
    output: &str,
) -> anyhow::Result<usize> {
    // Replace existing images in a .wilay file.
    // TODO: Error if indices are out of range?
    let mut wilay = MaybeXbc1::<Wilay>::from_file(input)?;
    let mut count = 0;
    match &mut wilay {
        MaybeXbc1::Uncompressed(wilay) => match wilay {
            Wilay::Dhal(dhal) => {
                replace_dhal_textures(dhal, &mut count, input, input_folder)?;
                dhal.save(output)?;
            }
            Wilay::Lagp(lagp) => {
                replace_lagp_textures(lagp, &mut count, input, input_folder)?;
                lagp.save(output)?;
            }
        },
        MaybeXbc1::Xbc1(xbc1) => {
            let mut wilay: Wilay = xbc1.extract()?;
            match &mut wilay {
                Wilay::Dhal(dhal) => {
                    replace_dhal_textures(dhal, &mut count, input, input_folder)?;
                    let xbc1 = Xbc1::new(xbc1.name.clone(), dhal, CompressionType::Zlib)?;
                    xbc1.save(output)?;
                }
                Wilay::Lagp(lagp) => {
                    replace_lagp_textures(lagp, &mut count, input, input_folder)?;
                    let xbc1 = Xbc1::new(xbc1.name.clone(), lagp, CompressionType::Zlib)?;
                    xbc1.save(output)?;
                }
            }
        }
    }

    Ok(count)
}

fn replace_lagp_textures(
    lagp: &mut Lagp,
    count: &mut usize,
    input: &str,
    input_folder: &str,
) -> anyhow::Result<()> {
    if let Some(textures) = &mut lagp.textures {
        *count += replace_wilay_mibl(textures, input, input_folder)?;
    }
    Ok(())
}

fn replace_dhal_textures(
    dhal: &mut Dhal,
    count: &mut usize,
    input: &str,
    input_folder: &str,
) -> anyhow::Result<()> {
    if let Some(textures) = &mut dhal.textures {
        *count += replace_wilay_mibl(textures, input, input_folder)?;
    }
    if let Some(textures) = &mut dhal.uncompressed_textures {
        *count += replace_wilay_jpeg(textures, input, input_folder)?;
    }
    Ok(())
}

fn replace_wilay_mibl(
    textures: &mut xc3_lib::dhal::Textures,
    input: &str,
    input_folder: &str,
) -> anyhow::Result<usize> {
    let mut count = 0;

    for entry in std::fs::read_dir(input_folder)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("dds") {
            if let Some(i) = image_index(&path, input) {
                // TODO: Add a to_bytes helper?
                let dds = Dds::from_file(&path)
                    .with_context(|| format!("{path:?} is not a valid DDS file"))?;
                let mibl = Mibl::from_dds(&dds).with_context(|| "failed to convert DDS to Mibl")?;
                let mut writer = Cursor::new(Vec::new());
                mibl.write(&mut writer)?;

                textures.textures[i].mibl_data = writer.into_inner();

                count += 1;
            }
        }
    }

    Ok(count)
}

fn replace_wilay_jpeg(
    textures: &mut xc3_lib::dhal::UncompressedTextures,
    input: &str,
    input_folder: &str,
) -> anyhow::Result<usize> {
    let mut count = 0;

    for entry in std::fs::read_dir(input_folder)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jpeg") {
            if let Some(i) = image_index(&path, input) {
                textures.textures[i].jpeg_data = std::fs::read(&path)
                    .with_context(|| format!("{path:?} is not a valid JPEG file"))?;
                count += 1;
            }
        }
    }

    Ok(count)
}

pub fn update_wimdo_from_folder(
    input: &str,
    input_folder: &str,
    output: &str,
    chr_tex_nx: Option<String>,
) -> anyhow::Result<usize> {
    let input_path = Path::new(input);
    let output_path = Path::new(output);

    // TODO: Error if indices are out of range?
    // TODO: avoid duplicating logic with xc3_model?
    let mut mxmd =
        Mxmd::from_file(input).with_context(|| format!("{input:?} is not a valid wimdo file"))?;

    let uses_chr = has_chr_textures(&mxmd);

    let chr_tex_nx_input = chr_tex_nx_folder(input_path).or(chr_tex_nx.map(Into::into));
    if uses_chr && chr_tex_nx_input.is_none() {
        panic!(
            "chr/tex/nx folder required by wimdo and wismt but cannot be inferred from input path"
        );
    }

    // We need to repack the entire wismt even though we only modify textures.
    let msrd = Msrd::from_file(input_path.with_extension("wismt"))?;
    let (vertex, spch, mut textures) = msrd.extract_files(chr_tex_nx_input.as_deref())?;

    let mut count = 0;

    for entry in std::fs::read_dir(input_folder)? {
        let path = entry?.path();
        if let Some(i) = image_index(&path, input) {
            if let Ok(dds) = Dds::from_file(path) {
                let new_mibl = Mibl::from_dds(&dds)?;
                if let Some(high) = &mut textures[i].high {
                    let (mid, base_mip) = new_mibl.split_base_mip();
                    *high = HighTexture {
                        mid,
                        base_mip: Some(base_mip),
                    };
                } else {
                    textures[i].low = new_mibl;
                }
                count += 1;
            }
        }
    }

    // Save files to disk.
    let new_msrd = Msrd::from_extracted_files(&vertex, &spch, &textures, uses_chr)?;

    mxmd.streaming = Some(new_msrd.streaming.clone());
    mxmd.save(output_path)?;
    new_msrd.save(output_path.with_extension("wismt"))?;

    Ok(count)
}

fn has_chr_textures(mxmd: &Mxmd) -> bool {
    // Some Xenoblade 3 models still require empty chr/tex/nx data even if disabled by flags.
    // Check the offset instead of flags to be safe.
    // TODO: Why does this not return true for all xc3 files?
    if let Some(streaming) = &mxmd.streaming {
        streaming.inner.has_chr_textures()
    } else {
        false
    }
}

fn image_index(path: &Path, input: &str) -> Option<usize> {
    // Match the input file name in case the folder contains multiple wilay.
    // Allow optional chars after the index like a texture name.
    // "mnu417_cont01.88.dds" -> 88
    // "model.7.BL000101_BODY_NRM.dds" -> 7
    let path = path.with_extension("");
    let file_name = path.file_name()?.to_str()?;
    let (file_name, rhs) = file_name.split_once('.')?;
    let (index, _) = rhs.split_once('.').unwrap_or((rhs, ""));

    let input_file_name = Path::new(input).with_extension("");
    let input_file_name = input_file_name.file_name()?.to_str()?;
    if file_name == input_file_name {
        index.parse().ok()
    } else {
        None
    }
}

pub fn extract_wilay_to_folder(
    wilay: MaybeXbc1<Wilay>,
    input: &Path,
    output_folder: &Path,
) -> anyhow::Result<usize> {
    let file_name = input.file_name().unwrap();
    let mut count = 0;
    match wilay {
        MaybeXbc1::Uncompressed(wilay) => match wilay {
            Wilay::Dhal(dhal) => extract_dhal(dhal, output_folder, file_name, &mut count)?,
            Wilay::Lagp(lagp) => extract_lagp(lagp, output_folder, file_name, &mut count)?,
        },
        MaybeXbc1::Xbc1(xbc1) => {
            let wilay: Wilay = xbc1.extract()?;
            match wilay {
                Wilay::Dhal(dhal) => extract_dhal(dhal, output_folder, file_name, &mut count)?,
                Wilay::Lagp(lagp) => extract_lagp(lagp, output_folder, file_name, &mut count)?,
            }
        }
    }

    Ok(count)
}

fn extract_lagp(
    lagp: Lagp,
    output_folder: &Path,
    file_name: &std::ffi::OsStr,
    count: &mut usize,
) -> anyhow::Result<()> {
    if let Some(textures) = lagp.textures {
        for (i, texture) in textures.textures.iter().enumerate() {
            let dds = Mibl::from_bytes(&texture.mibl_data)?.to_dds()?;
            let path = output_folder
                .join(file_name)
                .with_extension(format!("{i}.dds"));
            dds.save(path)?;
        }

        *count += textures.textures.len();
    }
    Ok(())
}

fn extract_dhal(
    dhal: Dhal,
    output_folder: &Path,
    file_name: &std::ffi::OsStr,
    count: &mut usize,
) -> anyhow::Result<()> {
    if let Some(textures) = dhal.textures {
        for (i, texture) in textures.textures.iter().enumerate() {
            let dds = Mibl::from_bytes(&texture.mibl_data)?.to_dds()?;
            let path = output_folder
                .join(file_name)
                .with_extension(format!("{i}.dds"));
            dds.save(path)?;
        }
        *count += textures.textures.len();
    }
    if let Some(textures) = dhal.uncompressed_textures {
        for (i, texture) in textures.textures.iter().enumerate() {
            let path = output_folder
                .join(file_name)
                .with_extension(format!("{i}.jpeg"));
            std::fs::write(path, &texture.jpeg_data)?;
        }

        *count += textures.textures.len();
    }
    Ok(())
}

pub fn extract_wimdo_to_folder(
    mxmd: Mxmd,
    input: &Path,
    output_folder: &Path,
) -> anyhow::Result<usize> {
    let file_name = input.file_name().unwrap();

    // TODO: packed mxmd textures.
    // TODO: chr/tex/nx folder as parameter?
    let chr_tex_nx = chr_tex_nx_folder(input);
    if has_chr_textures(&mxmd) && chr_tex_nx.is_none() {
        panic!(
            "chr/tex/nx folder required by wimdo and wismt but cannot be inferred from input path"
        );
    }

    let msrd = Msrd::from_file(input.with_extension("wismt"))?;
    let (_, _, textures) = msrd.extract_files(chr_tex_nx.as_deref())?;

    for (i, texture) in textures.iter().enumerate() {
        let dds = texture.mibl_final().to_dds()?;
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.{}.dds", texture.name));
        dds.save(path)?;
    }

    Ok(textures.len())
}

pub fn extract_bmn_to_folder(
    bmn: Bmn,
    input: &Path,
    output_folder: &Path,
) -> anyhow::Result<usize> {
    let file_name = input.file_name().unwrap();

    let mut count = 0;
    if let Some(unk16) = bmn.unk16 {
        for (i, texture) in unk16.textures.iter().enumerate() {
            if !texture.mtxt_data.is_empty() {
                let dds = Mtxt::from_bytes(&texture.mtxt_data)?.to_dds()?;
                let path = output_folder
                    .join(file_name)
                    .with_extension(format!("{i}.dds"));
                dds.save(path)?;
                count += 1;
            }
        }
    }

    Ok(count)
}

// TODO: Move this to xc3_lib?
pub fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> anyhow::Result<Mibl> {
    Xbc1::from_file(path)?.extract().map_err(Into::into)
}

pub fn create_wismt_single_tex(mibl: &Mibl) -> anyhow::Result<Xbc1> {
    // TODO: Set the name properly.
    Xbc1::new("middle.witx".to_string(), mibl, CompressionType::Zlib).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_index_paths() {
        assert_eq!(
            Some(0),
            image_index(&Path::new("a/b/file.0.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            Some(7),
            image_index(&Path::new("file.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(Some(7), image_index(&Path::new("file.7.dds"), "file.wilay"));
        assert_eq!(
            Some(7),
            image_index(&Path::new("file.7.optional_name.dds"), "file.wilay")
        );
        assert_eq!(
            None,
            image_index(&Path::new("file2.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            None,
            image_index(&Path::new("a/b/file.0.dds"), "b/c/file2.wilay")
        );
    }
}
