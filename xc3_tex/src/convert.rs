use std::{io::Cursor, path::Path};

use image_dds::{ddsfile::Dds, image::RgbaImage, ImageFormat, Surface};
use xc3_lib::{dds::DdsExt, dhal::Dhal, lagp::Lagp, mibl::Mibl, mxmd::Mxmd, xbc1::Xbc1};

pub enum File {
    Mibl(Mibl),
    Dds(Dds),
    Image(RgbaImage),
    Wilay(Wilay),
}

pub enum Wilay {
    Dhal(Dhal),
    Lagp(Lagp),
}

impl Wilay {
    // TODO: Move this to xc3_lib?
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        match Xbc1::from_file(path) {
            Ok(xbc1) => xbc1
                .extract()
                .map(Wilay::Dhal)
                .unwrap_or_else(|_| xbc1.extract().map(Wilay::Lagp).unwrap()),
            Err(_) => Dhal::from_file(path)
                .map(Wilay::Dhal)
                .unwrap_or_else(|_| Lagp::from_file(path).map(Wilay::Lagp).unwrap()),
        }
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
                panic!("Wilay must be saved to an output folder instead of a single image")
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
                panic!("Wilay must be saved to an output folder instead of a single image")
            }
        }
    }

    pub fn to_image(&self) -> RgbaImage {
        match self {
            File::Mibl(mibl) => image_dds::image_from_dds(&mibl.to_dds().unwrap(), 0).unwrap(),
            File::Dds(dds) => image_dds::image_from_dds(dds, 0).unwrap(),
            File::Image(image) => image.clone(),
            File::Wilay(_) => {
                panic!("Wilay must be saved to an output folder instead of a single image")
            }
        }
    }
}

pub fn update_wilay_from_folder(input: &str, input_folder: &str, output: &str) {
    // Replace existing images in a .wilay file.
    // TODO: Error if indices are out of range?
    // TODO: match the original if it uses xbc1 compression?
    let mut wilay = Wilay::from_file(input);
    match &mut wilay {
        Wilay::Dhal(dhal) => {
            if let Some(textures) = &mut dhal.textures {
                replace_wilay_mibl(textures, input, input_folder);
            }
            if let Some(textures) = &mut dhal.uncompressed_textures {
                replace_wilay_jpeg(textures, input, input_folder);
            }
            dhal.save(output).unwrap();
        }
        Wilay::Lagp(lagp) => {
            if let Some(textures) = &mut lagp.textures {
                replace_wilay_mibl(textures, input, input_folder);
            }
            lagp.save(output).unwrap();
        }
    }
}

fn replace_wilay_mibl(textures: &mut xc3_lib::dhal::Textures, input: &str, input_folder: &str) {
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
            }
        }
    }
}

fn replace_wilay_jpeg(
    textures: &mut xc3_lib::dhal::UncompressedTextures,
    input: &str,
    input_folder: &str,
) {
    for entry in std::fs::read_dir(input_folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("jpeg") {
            if let Some(i) = image_index(&path, input) {
                textures.textures[i].jpeg_data = std::fs::read(path).unwrap();
            }
        }
    }
}

pub fn update_wimdo_from_folder(input: &str, input_folder: &str, output: &str) {
    // TODO: Error if indices are out of range?
    let mut mxmd = Mxmd::from_file(input).unwrap();
    // TODO: also update wismt?
    // TODO: avoid duplicating logic with xc3_model?
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

pub fn save_wilay_to_folder(wilay: Wilay, input: &Path, output_folder: &Path) {
    let file_name = input.file_name().unwrap();
    match wilay {
        Wilay::Dhal(dhal) => {
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
            }
            if let Some(textures) = dhal.uncompressed_textures {
                for (i, texture) in textures.textures.iter().enumerate() {
                    let path = output_folder
                        .join(file_name)
                        .with_extension(format!("{i}.jpeg"));
                    std::fs::write(path, &texture.jpeg_data).unwrap();
                }
            }
        }
        Wilay::Lagp(lagp) => {
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
            }
        }
    }
}

// TODO: Move this to xc3_lib?
pub fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Mibl {
    Xbc1::from_file(path).unwrap().extract().unwrap()
}

pub fn create_wismt_single_tex(mibl: &Mibl) -> Xbc1 {
    // TODO: Set the name properly.
    Xbc1::new("b2062367_middle.witx".to_string(), mibl).unwrap()
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
