use std::{io::Cursor, path::Path};

use image_dds::{ddsfile::Dds, image::RgbaImage, ImageFormat, Surface};
use xc3_lib::{
    dds::DdsExt,
    dhal::Dhal,
    lagp::Lagp,
    mibl::Mibl,
    msrd::{
        streaming::{chr_tex_nx_folder, HighTexture},
        Msrd,
    },
    mxmd::Mxmd,
    xbc1::{MaybeXbc1, Xbc1},
};

// TODO: Support apmd?
pub enum File {
    Mibl(Mibl),
    Dds(Dds),
    Image(RgbaImage),
    Wilay(Wilay),
    Wimdo(Mxmd),
}

// TODO: Move this to xc3_lib?
pub enum Wilay {
    Dhal(MaybeXbc1<Dhal>),
    Lagp(MaybeXbc1<Lagp>),
}

impl Wilay {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        MaybeXbc1::<Dhal>::from_file(path)
            .map(Wilay::Dhal)
            .unwrap_or_else(|_| MaybeXbc1::<Lagp>::from_file(path).map(Wilay::Lagp).unwrap())
    }
}

impl File {
    pub fn to_dds(&self, format: Option<ImageFormat>) -> Dds {
        match self {
            File::Mibl(mibl) => mibl.to_dds().unwrap(),
            File::Dds(dds) => {
                // Handle changes in image format while preserving layers and mipmaps.
                // TODO: dds doesn't implement clone?
                match format {
                    Some(format) => Surface::from_dds(dds)
                        .unwrap()
                        .decode_rgba8()
                        .unwrap()
                        .encode(
                            format,
                            image_dds::Quality::Normal,
                            image_dds::Mipmaps::GeneratedAutomatic,
                        )
                        .unwrap()
                        .to_dds()
                        .unwrap(),
                    None => Dds {
                        header: dds.header.clone(),
                        header10: dds.header10.clone(),
                        data: dds.data.clone(),
                    },
                }
            }
            File::Image(image) => image_dds::dds_from_image(
                image,
                format.unwrap(),
                image_dds::Quality::Normal,
                image_dds::Mipmaps::GeneratedAutomatic,
            )
            .unwrap(),
            File::Wilay(_) => {
                panic!("wilay textures must be saved to an output folder instead of a single image")
            }
            File::Wimdo(_) => {
                panic!("wimdo textures must be saved to an output folder instead of a single image")
            }
        }
    }

    pub fn to_mibl(&self, format: Option<ImageFormat>) -> Mibl {
        match self {
            File::Mibl(mibl) => mibl.clone(),
            File::Dds(dds) => Mibl::from_dds(dds).unwrap(),
            File::Image(image) => {
                let dds = image_dds::dds_from_image(
                    image,
                    format.unwrap(),
                    image_dds::Quality::Normal,
                    image_dds::Mipmaps::GeneratedAutomatic,
                )
                .unwrap();
                Mibl::from_dds(&dds).unwrap()
            }
            File::Wilay(_) => {
                panic!("wilay textures must be saved to an output folder instead of a single image")
            }
            File::Wimdo(_) => {
                panic!("wimdo textures must be saved to an output folder instead of a single image")
            }
        }
    }

    pub fn to_image(&self) -> RgbaImage {
        match self {
            File::Mibl(mibl) => image_dds::image_from_dds(&mibl.to_dds().unwrap(), 0).unwrap(),
            File::Dds(dds) => image_dds::image_from_dds(dds, 0).unwrap(),
            File::Image(image) => image.clone(),
            File::Wilay(_) => {
                panic!("wilay textures must be saved to an output folder instead of a single image")
            }
            File::Wimdo(_) => {
                panic!("wimdo textures must be saved to an output folder instead of a single image")
            }
        }
    }
}

pub fn update_wilay_from_folder(input: &str, input_folder: &str, output: &str) -> usize {
    // Replace existing images in a .wilay file.
    // TODO: Error if indices are out of range?
    let mut wilay = Wilay::from_file(input);
    let mut count = 0;
    match &mut wilay {
        Wilay::Dhal(dhal) => match dhal {
            MaybeXbc1::Uncompressed(dhal) => {
                replace_dhal_textures(dhal, &mut count, input, input_folder);
                dhal.save(output).unwrap();
            }
            MaybeXbc1::Xbc1(xbc1) => {
                let mut dhal: Dhal = xbc1.extract().unwrap();
                replace_dhal_textures(&mut dhal, &mut count, input, input_folder);
                let xbc1 = Xbc1::new(xbc1.name.clone(), &dhal).unwrap();
                xbc1.save(output).unwrap();
            }
        },
        Wilay::Lagp(lagp) => match lagp {
            MaybeXbc1::Uncompressed(lagp) => {
                replace_lagp_textures(lagp, &mut count, input, input_folder);
                lagp.save(output).unwrap();
            }
            MaybeXbc1::Xbc1(xbc1) => {
                let mut lagp: Lagp = xbc1.extract().unwrap();
                replace_lagp_textures(&mut lagp, &mut count, input, input_folder);
                let xbc1 = Xbc1::new(xbc1.name.clone(), &lagp).unwrap();
                xbc1.save(output).unwrap();
            }
        },
    }

    count
}

fn replace_lagp_textures(lagp: &mut Lagp, count: &mut usize, input: &str, input_folder: &str) {
    if let Some(textures) = &mut lagp.textures {
        *count += replace_wilay_mibl(textures, input, input_folder);
    }
}

fn replace_dhal_textures(dhal: &mut Dhal, count: &mut usize, input: &str, input_folder: &str) {
    if let Some(textures) = &mut dhal.textures {
        *count += replace_wilay_mibl(textures, input, input_folder);
    }
    if let Some(textures) = &mut dhal.uncompressed_textures {
        *count += replace_wilay_jpeg(textures, input, input_folder);
    }
}

fn replace_wilay_mibl(
    textures: &mut xc3_lib::dhal::Textures,
    input: &str,
    input_folder: &str,
) -> usize {
    let mut count = 0;

    for entry in std::fs::read_dir(input_folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("dds") {
            if let Some(i) = image_index(&path, input) {
                // TODO: Add a to_bytes helper?
                let dds = Dds::from_file(path).unwrap();
                let mibl = Mibl::from_dds(&dds).unwrap();
                let mut writer = Cursor::new(Vec::new());
                mibl.write(&mut writer).unwrap();

                textures.textures[i].mibl_data = writer.into_inner();

                count += 1;
            }
        }
    }

    count
}

fn replace_wilay_jpeg(
    textures: &mut xc3_lib::dhal::UncompressedTextures,
    input: &str,
    input_folder: &str,
) -> usize {
    let mut count = 0;

    for entry in std::fs::read_dir(input_folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("jpeg") {
            if let Some(i) = image_index(&path, input) {
                textures.textures[i].jpeg_data = std::fs::read(path).unwrap();
                count += 1;
            }
        }
    }

    count
}

pub fn update_wimdo_from_folder(
    input: &str,
    input_folder: &str,
    output: &str,
    chr_tex_nx: Option<String>,
) -> usize {
    let input_path = Path::new(input);
    let output_path = Path::new(output);

    // TODO: Error if indices are out of range?
    // TODO: avoid duplicating logic with xc3_model?
    let mut mxmd = Mxmd::from_file(input).unwrap();

    let uses_chr = has_chr_textures(&mxmd);

    let chr_tex_nx_input = chr_tex_nx_folder(input_path).or(chr_tex_nx.map(Into::into));
    if uses_chr && chr_tex_nx_input.is_none() {
        panic!(
            "chr/tex/nx folder required by wimdo and wismt but cannot be inferred from input path"
        );
    }

    // We need to repack the entire wismt even though we only modify textures.
    let msrd = Msrd::from_file(input_path.with_extension("wismt")).unwrap();
    let (vertex, spch, mut textures) = msrd.extract_files(chr_tex_nx_input.as_deref()).unwrap();

    let mut count = 0;

    for entry in std::fs::read_dir(input_folder).unwrap() {
        let path = entry.unwrap().path();
        if let Some(i) = image_index(&path, input) {
            if let Ok(dds) = Dds::from_file(path) {
                let new_mibl = Mibl::from_dds(&dds).unwrap();
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
    let new_msrd = Msrd::from_extracted_files(&vertex, &spch, &textures, uses_chr).unwrap();

    mxmd.streaming = Some(new_msrd.streaming.clone());
    mxmd.save(output_path).unwrap();
    new_msrd.save(output_path.with_extension("wismt")).unwrap();

    count
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
    // "mnu417_cont01.88.dds" -> 88
    let path = path.with_extension("");
    let file_name = path.file_name()?.to_str()?;
    let (file_name, index) = file_name.rsplit_once('.')?;

    let input_file_name = Path::new(input).with_extension("");
    let input_file_name = input_file_name.file_name()?.to_str()?;
    if file_name == input_file_name {
        index.parse().ok()
    } else {
        None
    }
}

pub fn extract_wilay_to_folder(wilay: Wilay, input: &Path, output_folder: &Path) -> usize {
    let file_name = input.file_name().unwrap();
    let mut count = 0;
    match wilay {
        Wilay::Dhal(dhal) => match dhal {
            MaybeXbc1::Uncompressed(dhal) => {
                extract_dhal(dhal, output_folder, file_name, &mut count);
            }
            MaybeXbc1::Xbc1(xbc1) => {
                let dhal = xbc1.extract().unwrap();
                extract_dhal(dhal, output_folder, file_name, &mut count);
            }
        },
        Wilay::Lagp(lagp) => match lagp {
            MaybeXbc1::Uncompressed(lagp) => {
                extract_lagp(lagp, output_folder, file_name, &mut count);
            }
            MaybeXbc1::Xbc1(xbc1) => {
                let lagp = xbc1.extract().unwrap();
                extract_lagp(lagp, output_folder, file_name, &mut count);
            }
        },
    }

    count
}

fn extract_lagp(lagp: Lagp, output_folder: &Path, file_name: &std::ffi::OsStr, count: &mut usize) {
    if let Some(textures) = lagp.textures {
        for (i, texture) in textures.textures.iter().enumerate() {
            let dds = Mibl::from_bytes(&texture.mibl_data)
                .unwrap()
                .to_dds()
                .unwrap();
            let path = output_folder
                .join(file_name)
                .with_extension(format!("{i}.dds"));
            dds.save(path).unwrap();
        }

        *count += textures.textures.len();
    }
}

fn extract_dhal(dhal: Dhal, output_folder: &Path, file_name: &std::ffi::OsStr, count: &mut usize) {
    if let Some(textures) = dhal.textures {
        for (i, texture) in textures.textures.iter().enumerate() {
            let dds = Mibl::from_bytes(&texture.mibl_data)
                .unwrap()
                .to_dds()
                .unwrap();
            let path = output_folder
                .join(file_name)
                .with_extension(format!("{i}.dds"));
            dds.save(path).unwrap();
        }
        *count += textures.textures.len();
    }
    if let Some(textures) = dhal.uncompressed_textures {
        for (i, texture) in textures.textures.iter().enumerate() {
            let path = output_folder
                .join(file_name)
                .with_extension(format!("{i}.jpeg"));
            std::fs::write(path, &texture.jpeg_data).unwrap();
        }

        *count += textures.textures.len();
    }
}

pub fn extract_wimdo_to_folder(mxmd: Mxmd, input: &Path, output_folder: &Path) -> usize {
    let file_name = input.file_name().unwrap();

    // TODO: packed mxmd textures.
    // TODO: chr/tex/nx folder as parameter?
    let chr_tex_nx = chr_tex_nx_folder(input);
    if has_chr_textures(&mxmd) && chr_tex_nx.is_none() {
        panic!(
            "chr/tex/nx folder required by wimdo and wismt but cannot be inferred from input path"
        );
    }

    let msrd = Msrd::from_file(input.with_extension("wismt")).unwrap();
    let (_, _, textures) = msrd.extract_files(chr_tex_nx.as_deref()).unwrap();

    for (i, texture) in textures.iter().enumerate() {
        let dds = texture.mibl_final().to_dds().unwrap();
        let path = output_folder
            .join(file_name)
            .with_extension(format!("{i}.dds"));
        dds.save(path).unwrap();
    }

    textures.len()
}

// TODO: Move this to xc3_lib?
pub fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Mibl {
    Xbc1::from_file(path).unwrap().extract().unwrap()
}

pub fn create_wismt_single_tex(mibl: &Mibl) -> Xbc1 {
    // TODO: Set the name properly.
    Xbc1::new("middle.witx".to_string(), mibl).unwrap()
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
            None,
            image_index(&Path::new("file2.7.dds"), "b/c/file.wilay")
        );
        assert_eq!(
            None,
            image_index(&Path::new("a/b/file.0.dds"), "b/c/file2.wilay")
        );
    }
}
