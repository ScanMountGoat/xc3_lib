use clap::{Parser, ValueEnum};
use futures::executor::block_on;
use glam::{vec3, Vec3};
use image::ImageBuffer;
use xc3_wgpu::{
    material::load_database,
    renderer::{CameraData, Xc3Renderer},
};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The root folder containing model files to search recursively.
    /// Supports Xenoblade 2 and Xenoblade 3.
    root_folder: String,

    /// The file extension to load.
    extension: FileExtension,

    /// The GBuffer JSON database for texture assignments.
    /// If not specified, the first texture is assumed to be albedo color.
    shader_database: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum FileExtension {
    Wimdo,
    Wismhd,
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
            features: wgpu::Features::TEXTURE_COMPRESSION_BC,
            limits: wgpu::Limits::default(),
        },
        None,
    ))
    .unwrap();

    let renderer = Xc3Renderer::new(&device, WIDTH, HEIGHT);

    // Initialize the camera transform.
    let translation = vec3(0.0, -1.0, -10.0);
    let rotation_xyz = Vec3::ZERO;
    let camera_data = calculate_camera_data(WIDTH, HEIGHT, translation, rotation_xyz);
    renderer.update_camera(&queue, &camera_data);

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
        format: xc3_wgpu::COLOR_FORMAT,
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

    let database = cli.shader_database.map(load_database);

    // TODO: Work through mxmd in wiefb files in xc2?
    let ext = match cli.extension {
        FileExtension::Wimdo => "wimdo",
        FileExtension::Wismhd => "wismhd",
    };
    globwalk::GlobWalkerBuilder::from_patterns(&cli.root_folder, &[format!("*.{ext}")])
        .build()
        .unwrap()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let model_path = path.to_string_lossy().to_string();

            // TODO: Fix files that don't load.
            let paths = [
                // XC3
                "oj00031600.wimdo", // 3D texture
                "oj03010100.wimdo", // DMPA
                "ch02011110.wimdo", // 3D texture
                "ch02011111.wimdo", // 3D texture
                "ch02011112.wimdo", // 3D texture
                "ch02011113.wimdo", // 3D texture
                "ch02011114.wimdo", // 3D texture
                "ch02011115.wimdo", // 3D texture
                "ch02011116.wimdo", // 3D texture
                "ch02011117.wimdo", // 3D texture
                "ch02011118.wimdo", // 3D texture
                "ch02011119.wimdo", // 3D texture
                "ch02011120.wimdo", // 3D texture
                // XC2
                "oj108004.wimdo",
                "we010601.wimdo",
                "we010602.wimdo",
            ];
            if paths.iter().any(|p| model_path.ends_with(p)) {
                return;
            }

            println!("{:?}", model_path);
            let roots = match cli.extension {
                FileExtension::Wimdo => {
                    vec![xc3_model::load_model(model_path, database.as_ref())]
                }
                FileExtension::Wismhd => xc3_model::load_map(model_path, database.as_ref()),
            };

            let groups = xc3_wgpu::model::load_model(&device, &queue, &roots);

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            renderer.render_models(&output_view, &mut encoder, &groups);

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
        let mut buffer =
            ImageBuffer::<image::Rgba<u8>, _>::from_raw(WIDTH, HEIGHT, data.to_owned()).unwrap();
        // Convert BGRA to RGBA.
        buffer.pixels_mut().for_each(|p| p.0.swap(0, 2));

        buffer.save(output_path).unwrap();
    }
    output_buffer.unmap();
}

// TODO: Move to xc3_wgpu?
fn calculate_camera_data(
    width: u32,
    height: u32,
    translation: glam::Vec3,
    rotation: glam::Vec3,
) -> CameraData {
    let aspect = width as f32 / height as f32;

    let view = glam::Mat4::from_translation(translation)
        * glam::Mat4::from_rotation_x(rotation.x)
        * glam::Mat4::from_rotation_y(rotation.y);

    let projection = glam::Mat4::perspective_rh(0.5, aspect, 0.1, 100000.0);

    let view_projection = projection * view;

    let position = view.inverse().col(3);

    CameraData {
        view,
        view_projection,
        position,
    }
}
