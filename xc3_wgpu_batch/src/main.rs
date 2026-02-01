use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use clap::{Parser, ValueEnum};
use futures::executor::block_on;
use glam::{Mat4, Vec3, vec3};
use rayon::prelude::*;
use tracing::trace_span;
use xc3_model::{load_animations, shader_database::ShaderDatabase};
use xc3_wgpu::{CameraData, MonolibShaderTextures, Renderer};

#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const FOV_Y: f32 = 0.5;

/// Model and map batch renderer for Xenoblade X, Xenoblade 1 DE, Xenoblade 2, Xenoblade 3, and Xenoblade X DE.
#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// The game dump root folder containing the "monolib" folder.
    root_folder: String,

    /// The file extension to load.
    extension: FileExtension,

    /// The shader database for texture assignments.
    /// If not specified, texture usage is inferred from the texture usage type.
    shader_database: Option<String>,

    /// Apply the first entry of the corresponding animation if found.
    #[arg(long)]
    anim: bool,

    /// Draw axes for each bone in the skeleton.
    #[arg(long)]
    bones: bool,

    /// Override for the graphics backend.
    #[arg(
        long,
        value_parser = clap::builder::PossibleValuesParser::new(
        ["dx12", "vulkan", "metal"]
    ))]
    backend: Option<String>,
}

#[derive(Copy, PartialEq, Clone, Eq, ValueEnum)]
enum FileExtension {
    Wimdo,
    Pcmdo,
    Wismhd,
    Camdo,
}

impl FileExtension {
    fn ext(&self) -> &'static str {
        match self {
            FileExtension::Wimdo => "wimdo",
            FileExtension::Pcmdo => "pcmdo",
            FileExtension::Wismhd => "wismhd",
            FileExtension::Camdo => "camdo",
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let start = std::time::Instant::now();

    // Ignore most logs to avoid flooding the console.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .init()
        .unwrap();

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .unwrap();

    let backends = match &cli.backend {
        Some(backend) => match backend.to_lowercase().as_str() {
            "dx12" => wgpu::Backends::DX12,
            "vulkan" => wgpu::Backends::VULKAN,
            "metal" => wgpu::Backends::METAL,
            _ => wgpu::Backends::all(),
        },
        None => wgpu::Backends::all(),
    };
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: xc3_wgpu::FEATURES,
        required_limits: xc3_wgpu::LIMITS,
        ..Default::default()
    }))
    .unwrap();

    // Assume paths are somewhere in a full game dump.
    let mut monolib = Path::new(&cli.root_folder);
    while let Some(parent) = monolib.parent() {
        if monolib.join("monolib/shader").exists() {
            break;
        } else {
            monolib = parent;
        }
    }
    let monolib_shader =
        &MonolibShaderTextures::from_file(&device, &queue, monolib.join("monolib/shader"));

    let size = wgpu::Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    };
    let texture_desc = wgpu::TextureDescriptor {
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    };
    let output = device.create_texture(&texture_desc);
    let output_view = output.create_view(&Default::default());

    let renderer = Mutex::new(Renderer::new(
        &device,
        &queue,
        WIDTH,
        HEIGHT,
        texture_desc.format,
        monolib_shader,
    ));

    // Initialize the camera transform.
    let translation = vec3(0.0, -1.0, -10.0);
    let rotation = vec3(0.0, -20f32.to_radians(), 0.0);
    let camera_data = calculate_camera_data(WIDTH, HEIGHT, translation, rotation);
    renderer.lock().unwrap().update_camera(&queue, &camera_data);

    let database = cli
        .shader_database
        .map(ShaderDatabase::from_file)
        .transpose()
        .unwrap();

    // TODO: Output which model fails if there is a panic?
    let ext = cli.extension.ext();
    let paths = collect_paths(&cli.root_folder, &[format!("*.{ext}")]);
    paths.par_iter().for_each(|path| {
        // Create a unique buffer to avoid mapping a buffer from multiple threads.
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: WIDTH as u64 * HEIGHT as u64 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        });

        let model_path = path.to_string_lossy().to_string();

        let groups = load_groups(
            &device,
            &queue,
            &model_path,
            cli.extension,
            monolib_shader,
            database.as_ref(),
        );

        match groups {
            Ok(groups) => {
                if cli.anim {
                    find_and_apply_idle_anim(&queue, path, &groups);
                }

                for (group_index, group) in groups.iter().enumerate() {
                    let groups = &groups[group_index..group_index + 1];

                    for (models_index, models) in group.models.iter().enumerate() {
                        for (model_index, model) in models.models.iter().enumerate() {
                            let span = trace_span!("render_model");
                            span.in_scope(|| {
                                let mut encoder = device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor {
                                        label: Some("Render Encoder"),
                                    },
                                );

                                // Each model updates the renderer's internal buffers for camera framing.
                                // We need to hold the lock until the output image has been copied to the buffer.
                                // Rendering is cheap, so this has little performance impact in practice.
                                let mut renderer = renderer.lock().unwrap();

                                frame_bounds(&queue, &mut renderer, model.min_xyz, model.max_xyz);

                                renderer.render_models(
                                    &output_view,
                                    &mut encoder,
                                    groups,
                                    &[],
                                    false,
                                    cli.bones,
                                    Some(models_index),
                                    Some(model_index),
                                );

                                encoder.copy_texture_to_buffer(
                                    wgpu::TexelCopyTextureInfo {
                                        aspect: wgpu::TextureAspect::All,
                                        texture: &output,
                                        mip_level: 0,
                                        origin: wgpu::Origin3d::ZERO,
                                    },
                                    wgpu::TexelCopyBufferInfo {
                                        buffer: &output_buffer,
                                        layout: wgpu::TexelCopyBufferLayout {
                                            offset: 0,
                                            bytes_per_row: Some(WIDTH * 4),
                                            rows_per_image: Some(HEIGHT),
                                        },
                                    },
                                    size,
                                );

                                let output_path = if cli.extension == FileExtension::Wismhd {
                                    path.with_extension(format!(
                                        "{group_index}_{models_index}_{model_index}.png"
                                    ))
                                } else {
                                    path.with_extension("png")
                                };

                                let buffer = output_buffer.clone();
                                encoder.map_buffer_on_submit(
                                    &output_buffer,
                                    wgpu::MapMode::Read,
                                    0..,
                                    move |result| {
                                        if let Ok(()) = result {
                                            save_screenshot(&buffer, output_path);
                                        }
                                    },
                                );
                                queue.submit([encoder.finish()]);
                            });
                        }
                    }
                }
            }
            Err(e) => println!("Error loading {model_path:?}: {e:?}"),
        }
    });

    println!("Finished in {:?}", start.elapsed());
}

fn find_and_apply_idle_anim(queue: &wgpu::Queue, path: &Path, groups: &[xc3_wgpu::ModelGroup]) {
    // Search for paths with non empty anims using in game naming conventions.
    // TODO: Better heuristics based on all game versions.
    let model_name = path.file_stem().unwrap().to_string_lossy().to_string();

    // Xenoblade 3 chr/ch names.
    let mut chr_name = model_name.to_string();
    chr_name.replace_range(chr_name.len() - 3.., "000");

    let possible_anim_paths = [
        path.with_extension("mot"),
        path.with_file_name(format!("{chr_name}_field.mot")),
        path.with_file_name(format!("{model_name}_field.mot")),
        // Xenoblade 1 DE
        path.with_file_name(model_name.replace("en", "me"))
            .with_extension("mot"),
        path.with_file_name(model_name.replace("np", "mn"))
            .with_extension("mot"),
        path.with_file_name(format!("{}.mot", chr_name.replace("pc", "mp"))),
    ];
    possible_anim_paths
        .iter()
        .find(|p| apply_anim(queue, groups, p, &model_name));
}

#[tracing::instrument(skip_all)]
fn load_groups(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_path: &str,
    ext: FileExtension,
    monolib_shader: &MonolibShaderTextures,
    database: Option<&ShaderDatabase>,
) -> anyhow::Result<Vec<xc3_wgpu::ModelGroup>> {
    match ext {
        FileExtension::Wimdo | FileExtension::Pcmdo => {
            let root = xc3_model::load_model(model_path, database)?;
            Ok(xc3_wgpu::load_model(device, queue, &[root], monolib_shader))
        }
        FileExtension::Wismhd => {
            let mut roots = xc3_model::load_map(model_path, database)?;

            // Disable instancing since we only want to frame and render a single model.
            for root in &mut roots {
                for group in &mut root.groups {
                    for models in &mut group.models {
                        for model in &mut models.models {
                            model.instances = vec![Mat4::IDENTITY];
                        }
                    }
                }
            }

            Ok(xc3_wgpu::load_map(device, queue, &roots, monolib_shader))
        }
        FileExtension::Camdo => {
            let root = xc3_model::load_model_legacy(model_path, database)?;
            Ok(xc3_wgpu::load_model(device, queue, &[root], monolib_shader))
        }
    }
}

fn apply_anim(
    queue: &wgpu::Queue,
    groups: &[xc3_wgpu::ModelGroup],
    path: &Path,
    model_name: &str,
) -> bool {
    match load_animations(path) {
        Ok(animations) => {
            if let Some(animation) = animations
                .iter()
                .find(|a| a.name.starts_with(&format!("{model_name}_idle")))
                .or_else(|| animations.iter().find(|a| a.name.starts_with("idle_")))
                .or_else(|| animations.iter().find(|a| a.name.contains("btidle")))
                .or_else(|| animations.iter().find(|a| a.name.contains("idle")))
                .or_else(|| animations.first())
            {
                for group in groups {
                    group.update_bone_transforms(queue, animation, 0.0);
                }
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn frame_bounds(queue: &wgpu::Queue, renderer: &mut Renderer, min_xyz: Vec3, max_xyz: Vec3) {
    let center = (min_xyz + max_xyz) / 2.0;
    let bounds_size = max_xyz - min_xyz;

    // Find the base of the triangle based on vertical FOV and model height.
    // The aspect ratio is 1.0, so FOV_X is also FOV_Y.
    // Take the max to frame both horizontally and vertically.
    // Add a small offset to better frame the entire model.
    let distance = bounds_size.y.max(bounds_size.x) / FOV_Y.tan() + 2.0;

    let rotation = vec3(0.0, -20f32.to_radians(), 0.0);
    let camera_data = calculate_camera_data(
        WIDTH,
        HEIGHT,
        vec3(center.x, -center.y, -distance),
        rotation,
    );
    renderer.update_camera(queue, &camera_data);
}

#[tracing::instrument(skip_all)]
fn save_screenshot(output_buffer: &wgpu::Buffer, output_path: PathBuf) {
    // Save the output texture.
    let buffer_slice = output_buffer.slice(..);
    let data = buffer_slice.get_mapped_range();
    rayon::spawn(move || {
        let mut buffer = image::RgbaImage::from_raw(WIDTH, HEIGHT, data.to_owned()).unwrap();
        // Force opaque to match sm4sh_viewer.
        buffer.pixels_mut().for_each(|rgba| rgba[3] = 255u8);
        buffer.save(output_path).unwrap();
    });
}

// TODO: Move to xc3_wgpu?
fn calculate_camera_data(width: u32, height: u32, translation: Vec3, rotation: Vec3) -> CameraData {
    let aspect = width as f32 / height as f32;

    let view = Mat4::from_translation(translation)
        * Mat4::from_rotation_x(rotation.x)
        * Mat4::from_rotation_y(rotation.y);

    let projection = Mat4::perspective_rh(FOV_Y, aspect, 0.1, 100000.0);

    let view_projection = projection * view;

    let position = view.inverse().col(3);

    CameraData {
        view,
        projection,
        view_projection,
        position,
        width,
        height,
    }
}

fn collect_paths(root: &str, patterns: &[String]) -> Vec<PathBuf> {
    // Collect paths ahead of time since par_iter is faster than par_bridge.
    globwalk::GlobWalkerBuilder::from_patterns(root, patterns)
        .build()
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path().to_owned()))
        .collect()
}
