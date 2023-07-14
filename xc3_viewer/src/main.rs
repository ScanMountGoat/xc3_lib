use std::path::Path;

use futures::executor::block_on;
use glam::{vec3, Vec3};
use log::{debug, error, info};
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use xc3_wgpu::{
    material::load_database,
    model::{load_model, ModelGroup},
    renderer::{CameraData, Xc3Renderer},
    COLOR_FORMAT,
};

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    translation: Vec3,
    rotation_xyz: Vec3,

    renderer: Xc3Renderer,

    // TODO: Better way to render multiple map models?
    models: Vec<ModelGroup>,

    input_state: InputState,
}

#[derive(Default)]
struct InputState {
    is_mouse_left_clicked: bool,
    is_mouse_right_clicked: bool,
    previous_cursor_position: PhysicalPosition<f64>,
}

impl State {
    async fn new(window: &Window, model_path: &Path, database_path: &str) -> Self {
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

        let database = Some(load_database(database_path));

        let start = std::time::Instant::now();

        // Infer the type of model to load based on the extension.
        let models = match model_path.extension().unwrap().to_str().unwrap() {
            "wimdo" => {
                let root = xc3_model::load_model(model_path, database.as_ref());
                load_model(&device, &queue, &[root])
            }
            "wismhd" => {
                let roots = xc3_model::load_map(model_path, database.as_ref());
                load_model(&device, &queue, &roots)
            }
            _ => todo!(),
        };

        let mesh_count: usize = models
            .iter()
            .map(|m| {
                m.models
                    .iter()
                    .map(|model| model.meshes.len() * model.instances.len())
                    .sum::<usize>()
            })
            .sum();
        info!(
            "Load {:?} models and {:?} meshes: {:?}",
            models.len(),
            mesh_count,
            start.elapsed()
        );

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
            input_state: Default::default(),
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
    fn handle_input(&mut self, event: &WindowEvent) {
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

fn main() {
    // Ignore most wgpu logs to avoid flooding the console.
    simple_logger::SimpleLogger::new()
        .with_module_level("wgpu", log::LevelFilter::Warn)
        .with_module_level("naga", log::LevelFilter::Warn)
        .with_module_level("xc3_lib", log::LevelFilter::Info)
        .init()
        .unwrap();

    let args: Vec<_> = std::env::args().collect();

    let model_path = Path::new(&args[1]);

    let database_path = &args[2];

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(concat!("xc3_wgpu ", env!("CARGO_PKG_VERSION")))
        .build(&event_loop)
        .unwrap();

    let mut state = block_on(State::new(&window, model_path, database_path));
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
                state.handle_input(event);
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
