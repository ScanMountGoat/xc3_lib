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
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use xc3_lib::bc::Anim;
use xc3_wgpu::{
    model::ModelGroup,
    renderer::{CameraData, Xc3Renderer},
    COLOR_FORMAT,
};

#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    // Camera
    translation: Vec3,
    rotation_xyz: Vec3,

    renderer: Xc3Renderer,

    models: Vec<ModelGroup>,

    // Animation
    anims: Vec<Anim>,
    anim_index: usize,
    current_frame: f32,
    previous_frame_start: Instant,

    input_state: InputState,
}

#[derive(Default)]
struct InputState {
    is_mouse_left_clicked: bool,
    is_mouse_right_clicked: bool,
    previous_cursor_position: PhysicalPosition<f64>,
}

impl State {
    async fn new(
        window: &Window,
        model_path: &str,
        anim_path: Option<&String>,
        anim_index: usize,
        database_path: Option<&String>,
    ) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = unsafe { instance.create_surface(window).unwrap() };
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
                    features: xc3_wgpu::FEATURES,
                    limits: wgpu::Limits::default(),
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
        };
        surface.configure(&device, &config);

        let renderer = Xc3Renderer::new(&device, size.width, size.height);

        // Initialize the camera transform.
        let translation = vec3(0.0, -0.5, -15.0);
        let rotation_xyz = Vec3::ZERO;
        let camera_data = calculate_camera_data(size, translation, rotation_xyz);
        renderer.update_camera(&queue, &camera_data);

        let database = database_path.map(xc3_model::GBufferDatabase::from_file);

        let start = std::time::Instant::now();

        // Infer the type of model to load based on the extension.
        let models = match Path::new(model_path).extension().unwrap().to_str().unwrap() {
            "wimdo" => {
                // TODO: Dropping vertex buffers is expensive?
                let root = xc3_model::load_model(model_path, database.as_ref());
                info!("Load root: {:?}", start.elapsed());
                xc3_wgpu::model::load_model(&device, &queue, &[root])
            }
            "wismhd" => {
                let roots = xc3_model::load_map(model_path, database.as_ref());
                info!("Load {} roots: {:?}", roots.len(), start.elapsed());
                xc3_wgpu::model::load_model(&device, &queue, &roots)
            }
            _ => todo!(),
        };

        let elapsed = start.elapsed();

        let mesh_count: usize = models
            .iter()
            .map(|m| {
                m.models
                    .iter()
                    .flat_map(|models| {
                        models
                            .models
                            .iter()
                            .map(|model| model.meshes.len() * model.instances.len())
                    })
                    .sum::<usize>()
            })
            .sum();
        info!(
            "Load {:?} groups and {:?} meshes: {:?}",
            models.len(),
            mesh_count,
            elapsed
        );

        let mut anims = Vec::new();
        if let Some(anim_path) = anim_path {
            let sar1 = xc3_lib::sar1::Sar1::from_file(anim_path).unwrap();
            for entry in &sar1.entries {
                if let xc3_lib::sar1::EntryData::Bc(bc) = entry.read_data().unwrap() {
                    if let xc3_lib::bc::BcData::Anim(anim) = bc.data {
                        // println!("{:#?}", data);
                        anims.push(anim);
                    }
                }
            }
        }
        update_window_title(window, &anims, anim_index);

        Self {
            surface,
            device,
            queue,
            size,
            config,
            translation,
            rotation_xyz,
            models,
            renderer,
            anims,
            anim_index,
            current_frame: 0.0,
            input_state: Default::default(),
            previous_frame_start: Instant::now(),
        }
    }

    fn update_camera(&self, size: winit::dpi::PhysicalSize<u32>) {
        let camera_data = calculate_camera_data(size, self.translation, self.rotation_xyz);
        self.renderer.update_camera(&self.queue, &camera_data);
    }

    fn update_debug_settings(&self, index: u32) {
        self.renderer.update_debug_settings(&self.queue, index);
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
        if let Some(anim) = self.anims.get(self.anim_index) {
            // Framerate independent animation timing.
            let current_frame_start = std::time::Instant::now();
            self.current_frame = next_frame(
                self.current_frame,
                current_frame_start.duration_since(self.previous_frame_start),
                anim.binding.animation.frame_count as f32,
                1.0,
                false,
            );
            self.previous_frame_start = current_frame_start;

            for model in &self.models {
                for models in &model.models {
                    models.update_bone_transforms(&self.queue, anim, self.current_frame);
                }
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
            .render_models(&output_view, &mut encoder, &self.models);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    // Make this a reusable library that only requires glam?
    fn handle_input(&mut self, event: &WindowEvent, window: &Window) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                // Basic camera controls using arrow keys.
                if let Some(keycode) = input.virtual_keycode {
                    match keycode {
                        VirtualKeyCode::Left => self.translation.x += 0.1,
                        VirtualKeyCode::Right => self.translation.x -= 0.1,
                        VirtualKeyCode::Up => self.translation.y -= 0.1,
                        VirtualKeyCode::Down => self.translation.y += 0.1,
                        // Debug a selected G-Buffer texture.
                        VirtualKeyCode::Key0 => self.update_debug_settings(0),
                        VirtualKeyCode::Key1 => self.update_debug_settings(1),
                        VirtualKeyCode::Key2 => self.update_debug_settings(2),
                        VirtualKeyCode::Key3 => self.update_debug_settings(3),
                        VirtualKeyCode::Key4 => self.update_debug_settings(4),
                        VirtualKeyCode::Key5 => self.update_debug_settings(5),
                        VirtualKeyCode::Key6 => self.update_debug_settings(6),
                        // Animation playback.
                        VirtualKeyCode::Space => self.current_frame = 0.0,
                        VirtualKeyCode::PageUp => {
                            if input.state == ElementState::Released {
                                self.current_frame = 0.0;
                                self.anim_index += 1;
                                update_window_title(window, &self.anims, self.anim_index);
                            }
                        }
                        VirtualKeyCode::PageDown => {
                            if input.state == ElementState::Released {
                                self.current_frame = 0.0;
                                self.anim_index = self.anim_index.saturating_sub(1);
                                update_window_title(window, &self.anims, self.anim_index);
                            }
                        }
                        _ => (),
                    }
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

                // Clamp to prevent the user from zooming through the origin.
                self.translation.z = (self.translation.z + delta_z).min(-1.0);
            }
            _ => (),
        }
    }
}

fn update_window_title(window: &Window, anims: &[Anim], anim_index: usize) {
    if let Some(anim) = anims.get(anim_index) {
        window.set_title(&format!(
            "{} - {}",
            concat!("xc3_wgpu ", env!("CARGO_PKG_VERSION")),
            anim.binding.animation.name
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

pub fn next_frame(
    current_frame: f32,
    time_since_last_frame: Duration,
    final_frame_index: f32,
    playback_speed: f32,
    should_loop: bool,
) -> f32 {
    // Convert elapsed time to a delta in frames.
    // This relies on interpolation or frame skipping.
    // TODO: How robust is this implementation?

    // TODO: Pass in the frames per second?
    let millis_per_frame = 1000.0f64 / 30.0f64;
    let delta_t_frames = time_since_last_frame.as_millis() as f64 / millis_per_frame;

    let mut next_frame = current_frame + (delta_t_frames as f32 * playback_speed);

    if next_frame > final_frame_index {
        if should_loop {
            // Wrap around to loop the animation.
            // This may not be seamless if the animations have different lengths.
            next_frame = if final_frame_index > 0.0 {
                next_frame.rem_euclid(final_frame_index)
            } else {
                // Use 0.0 instead of NaN for empty animations.
                0.0
            };
        } else {
            // Reduce chances of overflow.
            next_frame = final_frame_index;
        }
    }

    next_frame
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The .wimdo or .wismhd file.
    model: String,
    /// The GBuffer JSON database generated by xc3_shader.
    database: Option<String>,
    /// The .mot animation file.
    anim: Option<String>,
    /// The BC entry index for the ANIM. Defaults to 0.
    anim_index: Option<usize>,
}

fn main() {
    // TODO: Can these both be active at once?
    // Ignore most wgpu logs to avoid flooding the console.
    #[cfg(not(feature = "tracing"))]
    {
        simple_logger::SimpleLogger::new()
            .with_module_level("wgpu", log::LevelFilter::Warn)
            .with_module_level("naga", log::LevelFilter::Warn)
            .with_module_level("xc3_lib", log::LevelFilter::Info)
            .init()
            .unwrap();
    }

    #[cfg(feature = "tracing")]
    {
        let (chrome_layer, _guard) = tracing_chrome::ChromeLayerBuilder::new().build();
        tracing_subscriber::registry()
            .with(
                // Limit tracing to these projects.
                chrome_layer.with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    metadata.target().starts_with("xc3_wgpu")
                        || metadata.target().starts_with("xc3_viewer")
                })),
            )
            .init();
    }

    let cli = Cli::parse();

    let event_loop = EventLoop::new();
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
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                state.resize(*physical_size);
                state.update_camera(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                state.resize(**new_inner_size);
            }
            _ => {
                state.handle_input(event, &window);
                state.update_camera(window.inner_size());
            }
        },
        Event::RedrawRequested(_) => match state.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
            Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
            Err(e) => error!("{e:?}"),
        },
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => (),
    });
}
