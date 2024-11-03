use std::path::Path;

use clap::{Parser, ValueEnum};
use futures::executor::block_on;
use glam::{vec3, Mat4, Vec3};
use image::ImageBuffer;
use xc3_model::{load_animations, shader_database::ShaderDatabase};
use xc3_wgpu::{CameraData, MonolibShaderTextures, Renderer};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const FOV_Y: f32 = 0.5;

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// The game dump root folder containing the "monolib" folder.
    /// Supports Xenoblade X, Xenoblade 1 DE, Xenoblade 2 and Xenoblade 3.
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
}

#[derive(Copy, PartialEq, Clone, Eq, ValueEnum)]
enum FileExtension {
    Wimdo,
    Pcmdo,
    Wismhd,
    Camdo,
}

fn main() {
    let cli = Cli::parse();

    // Ignore most logs to avoid flooding the console.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: xc3_wgpu::FEATURES,
            required_limits: wgpu::Limits {
                max_color_attachment_bytes_per_sample: 48,
                ..Default::default()
            },
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .unwrap();

    // Assume the path is the game root folder.
    let monolib_shader = MonolibShaderTextures::from_file(
        &device,
        &queue,
        Path::new(&cli.root_folder).join("monolib/shader"),
    );

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

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: WIDTH as u64 * HEIGHT as u64 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    });

    let mut renderer = Renderer::new(
        &device,
        &queue,
        WIDTH,
        HEIGHT,
        texture_desc.format,
        &monolib_shader,
    );

    // Initialize the camera transform.
    let translation = vec3(0.0, -1.0, -10.0);
    let rotation = vec3(0.0, -20f32.to_radians(), 0.0);
    let camera_data = calculate_camera_data(WIDTH, HEIGHT, translation, rotation);
    renderer.update_camera(&queue, &camera_data);

    let database = cli
        .shader_database
        .map(ShaderDatabase::from_file)
        .transpose()
        .unwrap();

    // TODO: Work through mxmd in wiefb files in xc2?
    let ext = match cli.extension {
        FileExtension::Wimdo => "wimdo",
        FileExtension::Pcmdo => "pcmdo",
        FileExtension::Wismhd => "wismhd",
        FileExtension::Camdo => "camdo",
    };
    globwalk::GlobWalkerBuilder::from_patterns(&cli.root_folder, &[format!("*.{ext}")])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let model_path = path.to_string_lossy().to_string();

            // TODO: Fix files that don't load.
            let paths = [
                // XC1
                "pc062700.wimdo",
                "ma0000.wismhd",
                // XC2
                "tg_ui_hitpoint.wimdo",
            ];
            if paths.iter().any(|p| model_path.ends_with(p)) {
                return;
            }

            println!("{:?}", model_path);
            let groups = match cli.extension {
                FileExtension::Wimdo | FileExtension::Pcmdo => {
                    let root = xc3_model::load_model(model_path, database.as_ref()).unwrap();
                    frame_model_bounds(&queue, &root, &mut renderer);
                    xc3_wgpu::load_model(&device, &queue, &[root], &monolib_shader)
                }
                FileExtension::Wismhd => {
                    let roots = xc3_model::load_map(model_path, database.as_ref()).unwrap();
                    frame_map_bounds(&queue, &roots, &mut renderer);
                    xc3_wgpu::load_map(&device, &queue, &roots, &monolib_shader)
                }
                FileExtension::Camdo => {
                    let root = xc3_model::load_model_legacy(model_path, database.as_ref()).unwrap();
                    frame_model_bounds(&queue, &root, &mut renderer);
                    xc3_wgpu::load_model(&device, &queue, &[root], &monolib_shader)
                }
            };

            if cli.anim {
                // Search for paths with non empty anims using in game naming conventions.
                // TODO: Better heuristics based on all game versions.
                let possible_anim_paths = [
                    path.with_extension("mot"),
                    path.with_extension("_obj.mot"),
                    path.with_extension("_field.mot"),
                ];
                possible_anim_paths
                    .iter()
                    .find(|p| apply_anim(&queue, &groups, p));
            }

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            renderer.render_models(&output_view, &mut encoder, &groups, false, cli.bones);

            let output_path = path.with_extension("png");
            save_screenshot(
                &device,
                &queue,
                encoder,
                &output,
                &output_buffer,
                size,
                output_path,
            );

            // Clean up resources.
            queue.submit(std::iter::empty());
            device.poll(wgpu::Maintain::Wait);
        });
}

fn apply_anim(queue: &wgpu::Queue, groups: &[xc3_wgpu::ModelGroup], path: &Path) -> bool {
    let animations = load_animations(path).unwrap();
    if let Some(animation) = animations.first() {
        for group in groups {
            group.update_bone_transforms(queue, animation, 0.0);
        }
        true
    } else {
        false
    }
}

fn frame_model_bounds(queue: &wgpu::Queue, root: &xc3_model::ModelRoot, renderer: &mut Renderer) {
    frame_bounds(queue, renderer, root.models.min_xyz, root.models.max_xyz);
}

fn frame_map_bounds(queue: &wgpu::Queue, roots: &[xc3_model::MapRoot], renderer: &mut Renderer) {
    let min_xyz = roots
        .iter()
        .flat_map(|r| {
            r.groups
                .iter()
                .flat_map(|g| g.models.iter().map(|m| m.min_xyz))
        })
        .reduce(Vec3::min)
        .unwrap();

    let max_xyz = roots
        .iter()
        .flat_map(|r| {
            r.groups
                .iter()
                .flat_map(|g| g.models.iter().map(|m| m.max_xyz))
        })
        .reduce(Vec3::max)
        .unwrap();

    frame_bounds(queue, renderer, min_xyz, max_xyz);
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

fn save_screenshot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    output: &wgpu::Texture,
    output_buffer: &wgpu::Buffer,
    size: wgpu::Extent3d,
    output_path: std::path::PathBuf,
) {
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::ImageCopyBuffer {
            buffer: output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: Some(HEIGHT),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);

    // Save the output texture.
    // Adapted from WGPU Example https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/capture
    {
        // TODO: Find ways to optimize this?
        let buffer_slice = output_buffer.slice(..);

        // TODO: Reuse the channel?
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        block_on(rx.receive()).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let buffer =
            ImageBuffer::<image::Rgba<u8>, _>::from_raw(WIDTH, HEIGHT, data.to_owned()).unwrap();
        buffer.save(output_path).unwrap();
    }
    output_buffer.unmap();
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
    }
}
