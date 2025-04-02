use std::path::Path;

use anyhow::{anyhow, Context};
use binrw::BinRead;
use image_dds::{
    ddsfile::Dds,
    image::{DynamicImage, RgbaImage},
    ImageFormat, Mipmaps, Quality, Surface,
};
use rayon::prelude::*;
use xc3_lib::{
    bmn::Bmn,
    dds::DdsExt,
    dhal::Dhal,
    fnt::Fnt,
    laft::Laft,
    lagp::Lagp,
    laps::Laps,
    mibl::Mibl,
    msrd::{
        streaming::{chr_folder, ExtractedTexture},
        Msrd,
    },
    mtxt::Mtxt,
    mxmd::{legacy::MxmdLegacy, Mxmd},
    xbc1::{CompressionType, MaybeXbc1, Xbc1},
};

use crate::load_input_file;

// TODO: Support apmd?
pub enum File {
    Mibl(Mibl),
    Mtxt(Mtxt),
    Dds(Dds),
    Image(RgbaImage),
    Wilay(Box<MaybeXbc1<Wilay>>),
    Wimdo(Box<Mxmd>),
    Camdo(Box<MxmdLegacy>),
    Bmn(Bmn),
    Wifnt(MaybeXbc1<Laft>),
    XcxFnt(Fnt),
    Caavp(Vec<Mtxt>),
}

// TODO: Move this to xc3_lib?
#[derive(Debug, BinRead, Clone)]
pub enum Wilay {
    Dhal(Dhal),
    Lagp(Lagp),
    // LAPS wilay have no images but shouldn't produce read errors.
    #[allow(dead_code)]
    Laps(Laps),
}

impl File {
    pub fn to_dds(
        &self,
        format: Option<ImageFormat>,
        quality: Option<Quality>,
        mipmaps: bool,
        cube: bool,
        depth: bool,
    ) -> anyhow::Result<Dds> {
        match self {
            File::Mibl(mibl) => mibl
                .to_dds()
                .with_context(|| "failed to convert Mibl to DDS"),
            File::Mtxt(mtxt) => mtxt
                .to_dds()
                .with_context(|| "failed to convert Mtxt to DDS"),
            File::Wifnt(laft) => laft_mibl(laft)?
                .to_dds()
                .with_context(|| "failed to convert Laft to DDS"),
            File::XcxFnt(fnt) => fnt
                .texture
                .to_dds()
                .with_context(|| "failed to convert Fnt to DDS"),
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
                    None => Ok(clone_dds(dds)),
                }
            }
            File::Image(image) => {
                let format =
                    format.ok_or(anyhow::anyhow!("missing required image output format"))?;
                let quality = quality.unwrap_or(Quality::Normal);
                let mipmaps = if mipmaps {
                    Mipmaps::GeneratedAutomatic
                } else {
                    Mipmaps::Disabled
                };

                if cube {
                    // Assume a square image.
                    image_dds::SurfaceRgba8::from_image_layers(
                        image,
                        image.height() / image.width(),
                    )
                    .encode(format, quality, mipmaps)
                    .with_context(|| "failed to encode image to DDS")?
                    .to_dds()
                    .with_context(|| "failed to create DDS")
                } else if depth {
                    // Assume a square image.
                    image_dds::SurfaceRgba8::from_image_depth(image, image.height() / image.width())
                        .encode(format, quality, mipmaps)
                        .with_context(|| "failed to encode image to DDS")?
                        .to_dds()
                        .with_context(|| "failed to create DDS")
                } else {
                    image_dds::dds_from_image(image, format, quality, mipmaps)
                        .with_context(|| "failed to encode image to DDS")
                }
            }
            File::Wilay(_) => Err(anyhow::anyhow!(
                "wilay textures must be saved to an output folder instead of a single image"
            )),
            File::Wimdo(_) => Err(anyhow::anyhow!(
                "wimdo textures must be saved to an output folder instead of a single image"
            )),
            File::Camdo(_) => Err(anyhow::anyhow!(
                "camdo textures must be saved to an output folder instead of a single image"
            )),
            File::Bmn(_) => Err(anyhow::anyhow!(
                "bmn textures must be saved to an output folder instead of a single image"
            )),
            File::Caavp(_) => Err(anyhow::anyhow!(
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
            File::Mtxt(mtxt) => Mibl::from_surface(mtxt.to_surface()?)
                .with_context(|| "failed to convert Mtxt to Mibl"),
            File::Wifnt(laft) => laft_mibl(laft),
            File::XcxFnt(fnt) => Mibl::from_surface(fnt.texture.to_surface()?)
                .with_context(|| "failed to convert Fnt to Mibl"),
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
            File::Camdo(_) => Err(anyhow::anyhow!(
                "camdo textures must be saved to an output folder instead of a single image"
            )),
            File::Bmn(_) => Err(anyhow::anyhow!(
                "bmn textures must be saved to an output folder instead of a single image"
            )),
            File::Caavp(_) => Err(anyhow::anyhow!(
                "caavp textures must be saved to an output folder instead of a single image"
            )),
        }
    }

    pub fn to_image(&self) -> anyhow::Result<RgbaImage> {
        match self {
            File::Mibl(mibl) => image_dds::image_from_dds(&mibl.to_dds()?, 0)
                .with_context(|| "failed to decode Mibl image"),
            File::Mtxt(mtxt) => image_dds::image_from_dds(&mtxt.to_dds()?, 0)
                .with_context(|| "failed to decode Mtxt image"),
            File::Wifnt(laft) => image_dds::image_from_dds(&laft_mibl(laft)?.to_dds()?, 0)
                .with_context(|| "failed to decode Laft image"),
            File::XcxFnt(fnt) => image_dds::image_from_dds(&fnt.texture.to_dds()?, 0)
                .with_context(|| "failed to decode Fnt image"),
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
            File::Camdo(_) => Err(anyhow::anyhow!(
                "camdo textures must be saved to an output folder instead of a single image"
            )),
            File::Bmn(_) => Err(anyhow::anyhow!(
                "bmn textures must be saved to an output folder instead of a single image"
            )),
            File::Caavp(_) => Err(anyhow::anyhow!(
                "caavp textures must be saved to an output folder instead of a single image"
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
    // LAPS files have no images to replace.
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
            Wilay::Laps(_) => (),
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
                Wilay::Laps(_) => (),
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
                let dds = Dds::from_file(&path)
                    .with_context(|| format!("{path:?} is not a valid DDS file"))?;
                let mibl = Mibl::from_dds(&dds).with_context(|| "failed to convert DDS to Mibl")?;
                textures.textures[i].mibl_data = mibl.to_bytes()?;

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
    chr: Option<String>,
) -> anyhow::Result<usize> {
    let input_path = Path::new(input);
    let output_path = Path::new(output);

    // TODO: Error if indices are out of range?
    // TODO: avoid duplicating logic with xc3_model?
    let mut mxmd =
        Mxmd::from_file(input).with_context(|| format!("{input:?} is not a valid wimdo file"))?;

    let uses_chr = has_chr_textures(&mxmd);

    let chr_folder = chr_folder(input_path).or(chr.map(Into::into));
    if uses_chr && chr_folder.is_none() {
        panic!(
            "chr/tex/nx folder required by wimdo and wismt but cannot be inferred from input path"
        );
    }

    // Replace all textures to support adding or deleting textures.
    let mut mibls: Vec<_> = std::fs::read_dir(input_folder)?
        .filter_map(|e| {
            let path = e.unwrap().path();
            image_index(&path, input).and_then(|i| {
                let dds = Dds::from_file(path).ok()?;
                let mibl = Mibl::from_dds(&dds).unwrap();
                Some((i, mibl))
            })
        })
        .collect();
    mibls.sort_by_key(|(i, _)| *i);

    // Check if all indices in 0..N are used.
    for (i, (index, _)) in mibls.iter().enumerate() {
        if i != *index {
            return Err(anyhow!("Found image index {index} but expected {i}"));
        }
    }

    let count = mibls.len();

    // We need to repack the entire wismt even though we only modify textures.
    // This ensures the streaming header accounts for potential stream data changes.
    match &mut mxmd.inner {
        xc3_lib::mxmd::MxmdInner::V111(inner) => {
            anyhow::bail!("Editing version 10111 legacy wimdo models is not supported")
        }
        xc3_lib::mxmd::MxmdInner::V112(inner) => {
            let msrd = Msrd::from_file(input_path.with_extension("wismt"))?;
            let (vertex, spch, mut textures) = msrd.extract_files(chr_folder.as_deref())?;
            replace_textures(mibls, &mut textures);

            let new_msrd = Msrd::from_extracted_files(&vertex, &spch, &textures, uses_chr)?;
            inner.streaming = Some(new_msrd.streaming.clone());

            mxmd.save(output_path)?;
            new_msrd.save(output_path.with_extension("wismt"))?;
        }
        xc3_lib::mxmd::MxmdInner::V40(inner) => {
            let msrd = Msrd::from_file(input_path.with_extension("wismt"))?;
            let (vertex, spch, mut textures) = msrd.extract_files_legacy(chr_folder.as_deref())?;
            replace_textures(mibls, &mut textures);

            let new_msrd = Msrd::from_extracted_files_legacy(&vertex, &spch, &textures, uses_chr)?;
            inner.streaming = Some(new_msrd.streaming.clone());

            mxmd.save(output_path)?;
            new_msrd.save(output_path.with_extension("wismt"))?;
        }
    }

    Ok(count)
}

fn replace_textures(
    mibls: Vec<(usize, Mibl)>,
    textures: &mut Vec<ExtractedTexture<Mibl, xc3_lib::mxmd::TextureUsage>>,
) {
    // TODO: Also extract the name.
    *textures = mibls
        .into_iter()
        .map(|(i, mibl)| {
            ExtractedTexture::from_mibl(
                &mibl,
                textures.get(i).map(|t| t.name.clone()).unwrap_or_default(),
                textures
                    .get(i)
                    .map(|t| t.usage)
                    .unwrap_or(xc3_lib::mxmd::TextureUsage::Col),
            )
        })
        .collect();
}

fn has_chr_textures(mxmd: &Mxmd) -> bool {
    if let Some(streaming) = match &mxmd.inner {
        xc3_lib::mxmd::MxmdInner::V111(mxmd) => &mxmd.streaming,
        xc3_lib::mxmd::MxmdInner::V112(mxmd) => &mxmd.streaming,
        xc3_lib::mxmd::MxmdInner::V40(mxmd) => &mxmd.streaming,
    } {
        // Some Xenoblade 3 models still require empty chr/tex/nx data even if disabled by flags.
        // Check the offset instead of flags to be safe.
        // TODO: Does this also work for Xenoblade X DE?
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
    let file_name = path.file_stem()?.to_str()?;
    let (file_name, rhs) = file_name.split_once('.')?;
    let (index, _) = rhs.split_once('.').unwrap_or((rhs, ""));

    let input_file_name = Path::new(input).file_stem()?.to_str()?;
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

    let (dds_textures, jpeg_textures) = extract_wilay_textures(wilay)?;

    save_unnamed_dds(&dds_textures, output_folder, file_name)?;

    for (i, jpeg) in jpeg_textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.jpeg"));
        std::fs::write(path, jpeg)?;
    }

    Ok(dds_textures.len() + jpeg_textures.len())
}

fn extract_wilay_images_to_folder(
    wilay: MaybeXbc1<Wilay>,
    input: &Path,
    output_folder: &Path,
    ext: &str,
) -> anyhow::Result<usize> {
    let file_name = input.file_name().unwrap();

    let (dds_textures, jpeg_textures) = extract_wilay_textures(wilay)?;

    for (i, dds) in dds_textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.{ext}"));
        dds.save_image(path)?;
    }

    for (i, jpeg) in jpeg_textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.{ext}"));
        if matches!(ext.to_lowercase().as_str(), "jpeg" | "jpg") {
            // Avoid introducing additional error by decoding and encoding.
            std::fs::write(path, jpeg)?;
        } else {
            // The output is not JPEG, so we need to decode first.
            let image = image_dds::image::load_from_memory_with_format(
                jpeg,
                image_dds::image::ImageFormat::Jpeg,
            )?;
            image.to_rgba8().save_image(path)?;
        }
    }

    Ok(dds_textures.len() + jpeg_textures.len())
}

fn extract_wilay_textures(wilay: MaybeXbc1<Wilay>) -> anyhow::Result<(Vec<Dds>, Vec<Vec<u8>>)> {
    let wilay = match wilay {
        MaybeXbc1::Uncompressed(wilay) => wilay,
        MaybeXbc1::Xbc1(xbc1) => xbc1.extract()?,
    };

    // LAPS wilay have no images to extract.
    match wilay {
        Wilay::Dhal(dhal) => extract_dhal_dds_jpeg(dhal),
        Wilay::Lagp(lagp) => Ok((extract_lagp_textures(lagp)?, Vec::new())),
        Wilay::Laps(_) => Ok((Vec::new(), Vec::new())),
    }
}

fn extract_lagp_textures(lagp: Lagp) -> anyhow::Result<Vec<Dds>> {
    let mut result = Vec::new();
    if let Some(textures) = lagp.textures {
        for texture in textures.textures {
            let dds = Mibl::from_bytes(&texture.mibl_data)?.to_dds()?;
            result.push(dds)
        }
    }
    Ok(result)
}

fn extract_dhal_dds_jpeg(dhal: Dhal) -> anyhow::Result<(Vec<Dds>, Vec<Vec<u8>>)> {
    let mut dds_textures = Vec::new();
    if let Some(textures) = dhal.textures {
        for texture in textures.textures {
            let dds = Mibl::from_bytes(&texture.mibl_data)?.to_dds()?;
            dds_textures.push(dds);
        }
    }
    let mut jpeg_textures = Vec::new();
    if let Some(textures) = dhal.uncompressed_textures {
        for texture in textures.textures {
            jpeg_textures.push(texture.jpeg_data);
        }
    }
    Ok((dds_textures, jpeg_textures))
}

pub fn extract_wimdo_to_folder(
    mxmd: Mxmd,
    input: &Path,
    output_folder: &Path,
) -> anyhow::Result<usize> {
    let textures = extract_wimdo_textures(mxmd, input)?;
    let file_name = input.file_name().unwrap();
    save_named_dds(&textures, output_folder, file_name)?;
    Ok(textures.len())
}

fn extract_wimdo_images_to_folder(
    mxmd: Mxmd,
    input: &Path,
    output_folder: &Path,
    ext: &str,
) -> anyhow::Result<usize> {
    let textures = extract_wimdo_textures(mxmd, input)?;
    let file_name = input.file_name().unwrap();
    save_named_dds_images(&textures, output_folder, file_name, ext)?;
    Ok(textures.len())
}

fn extract_wimdo_textures(mxmd: Mxmd, input: &Path) -> anyhow::Result<Vec<(String, Dds)>> {
    // TODO: chr/ folder as parameter?
    let chr_folder = chr_folder(input);
    if has_chr_textures(&mxmd) && chr_folder.is_none() {
        panic!("chr/ folder required by wimdo and wismt but cannot be inferred from input path");
    }

    // Assume streaming textures override packed textures if present.
    let mut result = Vec::new();
    match mxmd.inner {
        xc3_lib::mxmd::MxmdInner::V40(mxmd) => {
            if mxmd.streaming.is_some() {
                let msrd = Msrd::from_file(input.with_extension("wismt"))?;
                let (_, _, textures) = msrd.extract_files_legacy(chr_folder.as_deref())?;

                for texture in textures {
                    let dds = texture.surface_final()?.to_dds()?;
                    result.push((texture.name, dds));
                }
                Ok(result)
            } else if let Some(textures) = mxmd.packed_textures {
                for texture in textures.textures {
                    let mibl = Mibl::from_bytes(&texture.mibl_data)?;
                    let dds = mibl.to_dds()?;
                    result.push((texture.name, dds));
                }
                Ok(result)
            } else {
                Ok(Vec::new())
            }
        }
        xc3_lib::mxmd::MxmdInner::V111(mxmd) => {
            if mxmd.streaming.is_some() {
                let msrd = Msrd::from_file(input.with_extension("wismt"))?;
                let (_, _, textures) = msrd.extract_files(chr_folder.as_deref())?;

                for texture in textures {
                    let dds = texture.surface_final()?.to_dds()?;
                    result.push((texture.name, dds));
                }
                Ok(result)
            } else if let Some(textures) = mxmd.packed_textures {
                for texture in textures.textures {
                    let mibl = Mibl::from_bytes(&texture.mibl_data)?;
                    let dds = mibl.to_dds()?;
                    result.push((texture.name, dds));
                }
                Ok(result)
            } else {
                Ok(Vec::new())
            }
        }
        xc3_lib::mxmd::MxmdInner::V112(mxmd) => {
            if mxmd.streaming.is_some() {
                let msrd = Msrd::from_file(input.with_extension("wismt"))?;
                let (_, _, textures) = msrd.extract_files(chr_folder.as_deref())?;

                for texture in textures {
                    let dds = texture.surface_final()?.to_dds()?;
                    result.push((texture.name, dds));
                }
                Ok(result)
            } else if let Some(textures) = mxmd.packed_textures {
                for texture in textures.textures {
                    let mibl = Mibl::from_bytes(&texture.mibl_data)?;
                    let dds = mibl.to_dds()?;
                    result.push((texture.name, dds));
                }
                Ok(result)
            } else {
                Ok(Vec::new())
            }
        }
    }
}

pub fn extract_camdo_to_folder(
    mxmd: MxmdLegacy,
    input: &Path,
    output_folder: &Path,
) -> anyhow::Result<usize> {
    let textures = extract_camdo_textures(mxmd, input)?;
    let file_name = input.file_name().unwrap();
    save_named_dds(&textures, output_folder, file_name)?;
    Ok(textures.len())
}

pub fn extract_camdo_images_to_folder(
    mxmd: MxmdLegacy,
    input: &Path,
    output_folder: &Path,
    ext: &str,
) -> anyhow::Result<usize> {
    let textures = extract_camdo_textures(mxmd, input)?;
    let file_name = input.file_name().unwrap();
    save_named_dds_images(&textures, output_folder, file_name, ext)?;
    Ok(textures.len())
}

// TODO: Avoid duplicating this logic with xc3_model?
fn extract_camdo_textures(mxmd: MxmdLegacy, input: &Path) -> anyhow::Result<Vec<(String, Dds)>> {
    let mut result = Vec::new();

    // Assume streaming textures override packed textures if present.
    if let Some(streaming) = mxmd.streaming {
        let casmt = std::fs::read(input.with_extension("casmt")).unwrap();

        let low_data = &casmt[streaming.low_texture_data_offset as usize
            ..streaming.low_texture_data_offset as usize + streaming.low_texture_size as usize];
        let high_data = &casmt[streaming.texture_data_offset as usize
            ..streaming.texture_data_offset as usize + streaming.texture_size as usize];

        let (_, textures) = streaming
            .inner
            .extract_textures(low_data, high_data, |bytes| Mtxt::from_bytes(bytes))?;

        for texture in textures {
            let dds = texture.mtxt_final().to_dds()?;
            result.push((texture.name, dds));
        }
        Ok(result)
    } else if let Some(textures) = mxmd.packed_textures {
        for texture in textures.textures {
            let mtxt = Mtxt::from_bytes(&texture.mtxt_data)?;
            let dds = mtxt.to_dds()?;
            result.push((texture.name, dds));
        }
        Ok(result)
    } else {
        Ok(Vec::new())
    }
}

pub fn extract_bmn_to_folder(
    bmn: Bmn,
    input: &Path,
    output_folder: &Path,
) -> anyhow::Result<usize> {
    let textures = extract_bmn_textures(bmn)?;
    let file_name = input.file_name().unwrap();
    save_unnamed_dds(&textures, output_folder, file_name)?;
    Ok(textures.len())
}

pub fn extract_bmn_images_to_folder(
    bmn: Bmn,
    input: &Path,
    output_folder: &Path,
    ext: &str,
) -> anyhow::Result<usize> {
    let textures = extract_bmn_textures(bmn)?;
    let file_name = input.file_name().unwrap();
    save_unnamed_dds_images(&textures, output_folder, file_name, ext)?;
    Ok(textures.len())
}

fn extract_bmn_textures(bmn: Bmn) -> anyhow::Result<Vec<Dds>> {
    let mut result = Vec::new();
    if let Some(unk16) = bmn.unk16 {
        for texture in unk16.textures {
            if !texture.mtxt_data.is_empty() {
                let dds = Mtxt::from_bytes(&texture.mtxt_data)?.to_dds()?;
                result.push(dds);
            }
        }
    }

    Ok(result)
}

fn extract_caavp_to_folder(mtxts: Vec<Mtxt>, input: &Path) -> Result<(), anyhow::Error> {
    let file_name = input.file_name().unwrap();
    let output_folder = input.parent().unwrap();
    let dds_textures = extract_caavp_textures(mtxts)?;
    save_unnamed_dds(&dds_textures, output_folder, file_name)?;
    Ok(())
}

fn extract_caavp_textures(mtxts: Vec<Mtxt>) -> Result<Vec<Dds>, anyhow::Error> {
    let dds_textures = mtxts
        .iter()
        .map(|t| t.to_dds())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(dds_textures)
}

fn extract_caavp_images_to_folder(
    mtxts: Vec<Mtxt>,
    input: &Path,
    output_folder: &Path,
    ext: &str,
) -> Result<(), anyhow::Error> {
    let textures = extract_caavp_textures(mtxts)?;
    let file_name = input.file_name().unwrap();
    save_unnamed_dds_images(&textures, output_folder, file_name, ext)?;
    Ok(())
}

pub fn update_wifnt(input: &str, input_image: &str, output: &str) -> anyhow::Result<()> {
    let mut laft = MaybeXbc1::<Laft>::from_file(input)?;

    let dds = Dds::from_file(input_image)?;
    let mibl = Mibl::from_dds(&dds)?;

    match &mut laft {
        MaybeXbc1::Uncompressed(laft) => {
            laft.texture = Some(mibl);
            laft.save(output)?;
        }
        MaybeXbc1::Xbc1(xbc1) => {
            let mut laft: Laft = xbc1.extract()?;
            laft.texture = Some(mibl);
            let xbc1 = Xbc1::new(xbc1.name.clone(), &laft, CompressionType::Zlib)?;
            xbc1.save(output)?;
        }
    }

    Ok(())
}

pub fn batch_convert_files(
    input_folder: &str,
    pattern: &str,
    ext: Option<&str>,
) -> anyhow::Result<usize> {
    // TODO: properly count converted files.
    let ext = ext.unwrap_or("png");
    Ok(
        globwalk::GlobWalkerBuilder::from_patterns(input_folder, &[pattern])
            .build()
            .unwrap()
            .par_bridge()
            .map(|entry| {
                // TODO: Avoid unwrap?
                let path = entry.as_ref().unwrap().path();
                let file = load_input_file(path).unwrap();
                match ext.to_lowercase().as_str() {
                    "dds" => {
                        extract_and_save_dds(path, file).unwrap();
                    }
                    _ => {
                        extract_and_save_image(path, file, ext).unwrap();
                    }
                }

                1
            })
            .sum(),
    )
}

fn extract_and_save_dds(path: &Path, file: File) -> anyhow::Result<()> {
    match file {
        File::Mibl(mibl) => mibl.to_dds()?.save(path.with_extension("dds"))?,
        File::Mtxt(mtxt) => mtxt.to_dds()?.save(path.with_extension("dds"))?,
        File::Dds(dds) => dds.save(path.with_extension("dds"))?,
        File::Image(_) => Err(anyhow::anyhow!("cannot convert image to DDS"))?,
        File::Wilay(wilay) => {
            // TODO: don't extract wilay JPEG?
            extract_wilay_to_folder(*wilay, path, path.parent().unwrap())?;
        }
        File::Wimdo(mxmd) => {
            extract_wimdo_to_folder(*mxmd, path, path.parent().unwrap())?;
        }
        File::Camdo(mxmd) => {
            extract_camdo_to_folder(*mxmd, path, path.parent().unwrap())?;
        }
        File::Bmn(bmn) => {
            extract_bmn_to_folder(bmn, path, path.parent().unwrap())?;
        }
        File::Wifnt(laft) => laft_mibl(&laft)?
            .to_dds()?
            .save(path.with_extension("dds"))?,
        File::XcxFnt(fnt) => fnt.texture.to_dds()?.save(path.with_extension("dds"))?,
        File::Caavp(mtxts) => {
            extract_caavp_to_folder(mtxts, path)?;
        }
    }
    Ok(())
}

fn extract_and_save_image(path: &Path, file: File, ext: &str) -> anyhow::Result<()> {
    match file {
        File::Mibl(mibl) => {
            mibl.save_image(path.with_extension(ext))?;
        }
        File::Mtxt(mtxt) => {
            mtxt.save_image(path.with_extension(ext))?;
        }
        File::Dds(dds) => {
            dds.save_image(path.with_extension(ext))?;
        }
        File::Image(image) => image.save_image(path.with_extension(ext))?,
        File::Wilay(wilay) => {
            extract_wilay_images_to_folder(*wilay, path, path.parent().unwrap(), ext)?;
        }
        File::Wimdo(mxmd) => {
            extract_wimdo_images_to_folder(*mxmd, path, path.parent().unwrap(), ext)?;
        }
        File::Camdo(mxmd) => {
            extract_camdo_images_to_folder(*mxmd, path, path.parent().unwrap(), ext)?;
        }
        File::Bmn(bmn) => {
            extract_bmn_images_to_folder(bmn, path, path.parent().unwrap(), ext)?;
        }
        File::Wifnt(laft) => {
            laft_mibl(&laft)?.save_image(path.with_extension(ext))?;
        }
        File::XcxFnt(fnt) => {
            fnt.texture.save_image(path.with_extension(ext))?;
        }
        File::Caavp(mtxts) => {
            extract_caavp_images_to_folder(mtxts, path, path.parent().unwrap(), ext)?;
        }
    }
    Ok(())
}

fn laft_mibl(laft: &MaybeXbc1<Laft>) -> anyhow::Result<Mibl> {
    match laft {
        MaybeXbc1::Uncompressed(laft) => laft
            .texture
            .clone()
            .ok_or(anyhow!("no texture in wifnt file")),
        MaybeXbc1::Xbc1(xbc1) => xbc1
            .extract::<Laft>()?
            .texture
            .clone()
            .ok_or(anyhow!("no texture in wifnt file")),
    }
}

fn save_named_dds(
    textures: &[(String, Dds)],
    output_folder: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(), anyhow::Error> {
    for (i, (name, dds)) in textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.{name}.dds"));
        dds.save(path)?;
    }
    Ok(())
}

fn save_named_dds_images(
    textures: &[(String, Dds)],
    output_folder: &Path,
    file_name: &std::ffi::OsStr,
    ext: &str,
) -> Result<(), anyhow::Error> {
    for (i, (name, dds)) in textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.{name}.{ext}"));
        dds.save_image(path)?;
    }
    Ok(())
}

fn save_unnamed_dds(
    dds_textures: &[Dds],
    output_folder: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(), anyhow::Error> {
    for (i, dds) in dds_textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.dds"));
        dds.save(path)?;
    }
    Ok(())
}

fn save_unnamed_dds_images(
    textures: &[Dds],
    output_folder: &Path,
    file_name: &std::ffi::OsStr,
    ext: &str,
) -> Result<(), anyhow::Error> {
    for (i, dds) in textures.iter().enumerate() {
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.{ext}"));
        dds.save_image(path)?;
    }
    Ok(())
}

fn clone_dds(dds: &Dds) -> Dds {
    Dds {
        header: dds.header.clone(),
        header10: dds.header10.clone(),
        data: dds.data.clone(),
    }
}

pub trait SaveImageExt {
    fn save_image<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()>;
}

impl SaveImageExt for Dds {
    fn save_image<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        image_dds::image_from_dds(self, 0)?.save_image(path)
    }
}

impl SaveImageExt for Mibl {
    fn save_image<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        self.to_dds()?.save_image(path)
    }
}

impl SaveImageExt for Mtxt {
    fn save_image<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        self.to_dds()?.save_image(path)
    }
}

impl SaveImageExt for RgbaImage {
    fn save_image<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        // Workaround for JPEG export not being supported for rgba images.
        self.save(&path)
            .or_else(|_| DynamicImage::from(self.clone()).to_rgb8().save(path))
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_index_paths() {
        assert_eq!(
            Some(0),
            image_index(Path::new("a/b/file.0.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            Some(7),
            image_index(Path::new("file.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(Some(7), image_index(Path::new("file.7.dds"), "file.wilay"));
        assert_eq!(
            Some(7),
            image_index(Path::new("file.7.optional_name.dds"), "file.wilay")
        );
        assert_eq!(
            None,
            image_index(Path::new("file2.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            None,
            image_index(Path::new("a/b/file.0.dds"), "b/c/file2.wilay")
        );
    }
}
