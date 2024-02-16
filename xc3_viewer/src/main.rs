use std::{
    path::Path,
    time::{Duration, Instant},
};

use clap::Parser;
use futures::executor::block_on;
use glam::{vec3, Vec3};
use log::{debug, error, info};
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::{Window, WindowBuilder},
};
use xc3_model::{animation::Animation, load_animations, shader_database::ShaderDatabase};
use xc3_wgpu::{CameraData, ModelGroup, RenderMode, Xc3Renderer, COLOR_FORMAT};

#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    // Camera
    translation: Vec3,
    rotation_xyz: Vec3,

    renderer: Xc3Renderer,

    model_name: String,
    groups: Vec<ModelGroup>,

    // Animation
    animations: Vec<Animation>,
    animation_index: usize,
    current_time_seconds: f32,
    previous_frame_start: Instant,

    input_state: InputState,
}

#[derive(Default)]
struct InputState {
    is_mouse_left_clicked: bool,
    is_mouse_right_clicked: bool,
    previous_cursor_position: PhysicalPosition<f64>,
}

impl<'a> State<'a> {
    async fn new(
        window: &'a Window,
        model_path: &str,
        anim_path: Option<&String>,
        animation_index: usize,
        database_path: Option<&String>,
    ) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        debug!("{:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: xc3_wgpu::FEATURES,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: COLOR_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // TODO: Make the monolib/shader path optional?
        // Assume paths are somewhere in a full game dump.
        let mut root_folder = Path::new(model_path);
        while let Some(parent) = root_folder.parent() {
            if root_folder.join("monolib/shader").exists() {
                break;
            } else {
                root_folder = parent;
            }
        }
        let monolib_shader = root_folder.join("monolib/shader");
        let renderer = Xc3Renderer::new(&device, &queue, size.width, size.height, monolib_shader);

        // Initialize the camera transform.
        let translation = vec3(0.0, -0.5, -15.0);
        let rotation_xyz = Vec3::ZERO;
        let camera_data = calculate_camera_data(size, translation, rotation_xyz);
        renderer.update_camera(&queue, &camera_data);

        let database = database_path.map(|p| ShaderDatabase::from_file(p).unwrap());

        let start = std::time::Instant::now();

        // Infer the type of model to load based on the extension.
        let groups = match Path::new(model_path).extension().unwrap().to_str().unwrap() {
            "wimdo" | "pcmdo" => {
                // TODO: Dropping vertex buffers is expensive?
                let root = xc3_model::load_model(model_path, database.as_ref())
                    .expect(&format!("{model_path:?} should be a valid .wimdo file"));
                info!("Load root: {:?}", start.elapsed());
                xc3_wgpu::load_model(&device, &queue, &[root])
            }
            "wismhd" => {
                let roots = xc3_model::load_map(model_path, database.as_ref())
                    .expect(&format!("{model_path:?} should be a valid .wismhd file"));
                info!("Load {} roots: {:?}", roots.len(), start.elapsed());
                xc3_wgpu::load_model(&device, &queue, &roots)
            }
            ext => panic!("unrecognized extension {ext}"),
        };

        let elapsed = start.elapsed();

        let mesh_count: usize = groups
            .iter()
            .map(|m| {
                m.models
                    .iter()
                    .flat_map(|models| {
                        models
                            .models
                            .iter()
                            .map(|model| model.meshes.len() * model.instance_count)
                    })
                    .sum::<usize>()
            })
            .sum();
        info!(
            "Load {:?} groups and {:?} meshes: {:?}",
            groups.len(),
            mesh_count,
            elapsed
        );

        let model_name = Path::new(model_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let animations = anim_path
            .map(|p| load_animations(p).expect("anim path should point to a valid animation file"))
            .unwrap_or_default();
        update_window_title(window, &model_name, &animations, animation_index);

        Self {
            surface,
            device,
            queue,
            size,
            config,
            translation,
            rotation_xyz,
            model_name,
            groups,
            renderer,
            animations,
            animation_index,
            current_time_seconds: 0.0,
            input_state: Default::default(),
            previous_frame_start: Instant::now(),
        }
    }

    fn update_camera(&self, size: winit::dpi::PhysicalSize<u32>) {
        let camera_data = calculate_camera_data(size, self.translation, self.rotation_xyz);
        self.renderer.update_camera(&self.queue, &camera_data);
    }

    fn update_debug_settings(&mut self, render_mode: RenderMode) {
        self.renderer
            .update_debug_settings(&self.queue, render_mode);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // Update each resource that depends on window size.
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.renderer
                .resize(&self.device, new_size.width, new_size.height);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if let Some(anim) = self.animations.get(self.animation_index) {
            // Framerate independent animation timing.
            // This relies on interpolation or frame skipping.
            let current_frame_start = std::time::Instant::now();
            let delta_t = current_frame_start
                .duration_since(self.previous_frame_start)
                .as_secs_f64() as f32;
            self.current_time_seconds += delta_t;
            self.previous_frame_start = current_frame_start;

            for group in &self.groups {
                group.update_bone_transforms(&self.queue, anim, self.current_time_seconds);
            }
        }

        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.renderer
            .render_models(&output_view, &mut encoder, &self.groups);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    // Make this a reusable library that only requires glam?
    fn handle_input(&mut self, event: &WindowEvent, window: &Window) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                match &event.logical_key {
                    winit::keyboard::Key::Named(named) => match named {
                        // Basic camera controls using arrow keys.
                        NamedKey::ArrowLeft => self.translation.x += 0.1,
                        NamedKey::ArrowRight => self.translation.x -= 0.1,
                        NamedKey::ArrowUp => self.translation.y -= 0.1,
                        NamedKey::ArrowDown => self.translation.y += 0.1,
                        // Animation playback.
                        NamedKey::Space => self.current_time_seconds = 0.0,
                        _ => (),
                    },
                    winit::keyboard::Key::Character(c) => {
                        match c.as_str() {
                            // Debug a selected G-Buffer texture.
                            "0" => self.update_debug_settings(RenderMode::Shaded),
                            "1" => self.update_debug_settings(RenderMode::GBuffer0),
                            "2" => self.update_debug_settings(RenderMode::GBuffer1),
                            "3" => self.update_debug_settings(RenderMode::GBuffer2),
                            "4" => self.update_debug_settings(RenderMode::GBuffer3),
                            "5" => self.update_debug_settings(RenderMode::GBuffer4),
                            "6" => self.update_debug_settings(RenderMode::GBuffer5),
                            // Animation playback.
                            "." => {
                                if event.state == ElementState::Released {
                                    self.current_time_seconds = 0.0;
                                    self.animation_index += 1;
                                    update_window_title(
                                        window,
                                        &self.model_name,
                                        &self.animations,
                                        self.animation_index,
                                    );
                                }
                            }
                            "," => {
                                if event.state == ElementState::Released {
                                    self.current_time_seconds = 0.0;
                                    self.animation_index = self.animation_index.saturating_sub(1);
                                    update_window_title(
                                        window,
                                        &self.model_name,
                                        &self.animations,
                                        self.animation_index,
                                    );
                                }
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                // Track mouse clicks to only rotate when dragging while clicked.
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.input_state.is_mouse_left_clicked = true
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.input_state.is_mouse_left_clicked = false
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.input_state.is_mouse_right_clicked = true
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.input_state.is_mouse_right_clicked = false
                    }
                    _ => (),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.input_state.is_mouse_left_clicked {
                    let delta_x = position.x - self.input_state.previous_cursor_position.x;
                    let delta_y = position.y - self.input_state.previous_cursor_position.y;

                    // Swap XY so that dragging left/right rotates left/right.
                    self.rotation_xyz.x += (delta_y * 0.01) as f32;
                    self.rotation_xyz.y += (delta_x * 0.01) as f32;
                } else if self.input_state.is_mouse_right_clicked {
                    let delta_x = position.x - self.input_state.previous_cursor_position.x;
                    let delta_y = position.y - self.input_state.previous_cursor_position.y;

                    // Translate an equivalent distance in screen space based on the camera.
                    // The viewport height and vertical field of view define the conversion.
                    let fac = FOV_Y.sin() * self.translation.z.abs() / self.size.height as f32;

                    // Negate y so that dragging up "drags" the model up.
                    self.translation.x += delta_x as f32 * fac;
                    self.translation.y -= delta_y as f32 * fac;
                }
                // Always update the position to avoid jumps when moving between clicks.
                self.input_state.previous_cursor_position = *position;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Scale zoom speed with distance to make it easier to zoom out large scenes.
                let delta_z = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => *y * self.translation.z.abs() * 0.1,
                    MouseScrollDelta::PixelDelta(p) => {
                        p.y as f32 * self.translation.z.abs() * 0.005
                    }
                };

                self.translation.z += delta_z;
            }
            _ => (),
        }
    }
}

fn update_window_title(window: &Window, model_name: &str, anims: &[Animation], anim_index: usize) {
    if let Some(anim) = anims.get(anim_index) {
        window.set_title(&format!(
            "{} - {} - {}",
            concat!("xc3_wgpu ", env!("CARGO_PKG_VERSION")),
            model_name,
            anim.name
        ));
    }
}

// TODO: Move to xc3_wgpu?
fn calculate_camera_data(
    size: winit::dpi::PhysicalSize<u32>,
    translation: glam::Vec3,
    rotation: glam::Vec3,
) -> CameraData {
    let aspect = size.width as f32 / size.height as f32;

    let view = glam::Mat4::from_translation(translation)
        * glam::Mat4::from_rotation_x(rotation.x)
        * glam::Mat4::from_rotation_y(rotation.y);

    let projection = glam::Mat4::perspective_rh(FOV_Y, aspect, Z_NEAR, Z_FAR);

    let view_projection = projection * view;

    let position = view.inverse().col(3);

    CameraData {
        view,
        view_projection,
        position,
    }
}

pub fn current_time_seconds(
    current_time_seconds: f32,
    time_since_last_frame: Duration,
    playback_speed: f32,
) -> f32 {
    // Calculate the time since the start of the animation in seconds.
    // This relies on interpolation or frame skipping.
    let delta_t = time_since_last_frame.as_secs_f64() as f32;
    current_time_seconds + delta_t * playback_speed
}

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// The .wimdo or .wismhd file.
    model: String,
    /// The GBuffer JSON database generated by xc3_shader.
    database: Option<String>,
    /// The .mot animation file.
    #[arg(long)]
    anim: Option<String>,
    /// The BC entry index for the ANIM. Defaults to 0.
    #[arg(long)]
    anim_index: Option<usize>,
}

fn main() {
    // TODO: Can these both be active at once?
    // Ignore most logs to avoid flooding the console.
    #[cfg(not(feature = "tracing"))]
    {
        simple_logger::SimpleLogger::new()
            .with_level(log::LevelFilter::Info)
            .with_module_level("wgpu", log::LevelFilter::Warn)
            .with_module_level("naga", log::LevelFilter::Warn)
            .init()
            .unwrap();
    }

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .unwrap();

    let cli = Cli::parse();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title(concat!("xc3_wgpu ", env!("CARGO_PKG_VERSION")))
        .build(&event_loop)
        .unwrap();

    let mut state = block_on(State::new(
        &window,
        &cli.model,
        cli.anim.as_ref(),
        cli.anim_index.unwrap_or_default(),
        cli.database.as_ref(),
    ));
    event_loop
        .run(|event, target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                    state.update_camera(*physical_size);
                    window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {}
                WindowEvent::RedrawRequested => {
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                        Err(e) => error!("{e:?}"),
                    }
                    window.request_redraw();
                }
                _ => {
                    state.handle_input(event, &window);
                    state.update_camera(window.inner_size());
                    window.request_redraw();
                }
            },
            _ => (),
        })
        .unwrap();
}
