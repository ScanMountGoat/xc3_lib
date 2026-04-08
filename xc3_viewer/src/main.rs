use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow};
use clap::Parser;
use futures::executor::block_on;
use glam::{Mat4, Vec3, vec3};
use log::info;
use winit::{
    application::ApplicationHandler, dpi::PhysicalPosition, event::*, event_loop::EventLoop,
    keyboard::NamedKey, window::Window,
};
use xc3_model::{animation::Animation, load_animations, shader_database::ShaderDatabase};
use xc3_wgpu::{CameraData, Collision, ModelGroup, MonolibShaderTextures, RenderMode, Renderer};

#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 * 0.988;

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    // Camera
    translation: Vec3,
    rotation_xyz: Vec3,

    renderer: Renderer,
    render_mode: RenderMode,

    model_names: String,
    groups: Vec<ModelGroup>,

    // Animation
    animations: Vec<Animation>,
    animation_index: usize,
    current_time_seconds: f32,
    previous_frame_start: Instant,

    collisions: Vec<Collision>,

    draw_bones: bool,
    draw_bounds: bool,

    root_index: Option<usize>,
    group_index: Option<usize>,
    models_index: Option<usize>,
    model_index: Option<usize>,

    input_state: InputState,
    movement: bool,
    movement_speed: f32,
    mouse_sensitivity: f32,
}

#[derive(Default)]
struct InputState {
    is_mouse_left_clicked: bool,
    is_mouse_right_clicked: bool,
    previous_cursor_position: PhysicalPosition<f64>,
    move_forward: bool,  // W
    move_backward: bool, // S
    move_left: bool,     // A
    move_right: bool,    // D
    move_up: bool,       // E
    move_down: bool,     // Q
}

/// Returns the camera's world-space forward and right unit vectors for the
/// current pitch/yaw. Uses pure trigonometry so there is no matrix inversion
/// or floating-point drift. Yaw rotates around the global Y axis; pitch tilts
/// up/down from that horizontal plane.
///
/// Convention matches glam's right-handed coordinate system (−Z forward at rest).
fn camera_directions(pitch: f32, yaw: f32) -> (Vec3, Vec3) {
    let forward = Vec3::new(
        yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
    (forward, right)
}

impl State {
    async fn new(
        window: Window,
        cli: &Cli,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> anyhow::Result<Self> {
        let window = Arc::new(window);
        let backends = match &cli.backend {
            Some(backend) => match backend.to_lowercase().as_str() {
                "dx12" => wgpu::Backends::DX12,
                "vulkan" => wgpu::Backends::VULKAN,
                "metal" => wgpu::Backends::METAL,
                _ => wgpu::Backends::all(),
            },
            None => wgpu::Backends::all(),
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
                event_loop.owned_display_handle(),
            ))
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        info!("{:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: xc3_wgpu::FEATURES,
                required_limits: xc3_wgpu::LIMITS,
                ..Default::default()
            })
            .await
            .unwrap();

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
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
        let mut root_folder = Path::new(&cli.files[0]);
        while let Some(parent) = root_folder.parent() {
            if root_folder.join("monolib/shader").exists() {
                break;
            } else {
                root_folder = parent;
            }
        }
        let monolib_shader =
            MonolibShaderTextures::from_file(&device, &queue, root_folder.join("monolib/shader"));
        let mut renderer = Renderer::new(
            &device,
            &queue,
            size.width,
            size.height,
            config.format,
            &monolib_shader,
        );

        // Initialize the camera transform.
        let translation = if cli.movement {
            vec3(0.0, 0.5, 15.0)
        } else {
            vec3(0.0, -0.5, -15.0)
        };
        let rotation_xyz = Vec3::ZERO;
        let camera_data = calculate_camera_data(size, translation, rotation_xyz, cli.movement);
        renderer.update_camera(&queue, &camera_data);

        let start = std::time::Instant::now();

        let database = match &cli.database {
            Some(p) => Some(
                ShaderDatabase::from_file(p)
                    .with_context(|| format!("{p:?} is not a valid shader database file"))?,
            ),
            None => ShaderDatabase::from_file(database_path()?).ok(),
        };

        info!("Load shader database: {:?}", start.elapsed());

        let start = std::time::Instant::now();

        let mut groups = Vec::new();
        let mut collisions = Vec::new();

        let mut model_roots = Vec::new();
        let mut map_roots = Vec::new();

        for file in &cli.files {
            match Path::new(file).extension().unwrap().to_str().unwrap() {
                "wimdo" | "pcmdo" => {
                    // TODO: merge roots or just merge skeletons?
                    let root = xc3_model::load_model(file, database.as_ref())
                        .with_context(|| format!("failed to load .wimdo model from {file:?}"))?;
                    model_roots.push(root);
                }
                "camdo" => {
                    let root = xc3_model::load_model_legacy(file, database.as_ref())
                        .with_context(|| format!("failed to load .camdo model from {file:?}"))?;
                    model_roots.push(root);
                }
                "wismhd" => {
                    let roots = xc3_model::load_map(file, database.as_ref())
                        .with_context(|| format!("failed to load .wismhd map from {file:?}"))?;
                    map_roots.extend(roots);
                }
                "wiidcm" | "idcm" => {
                    let collision_meshes = xc3_model::load_collisions(file)
                        .with_context(|| format!("failed to load collisions from {file:?}"))?;
                    collisions.extend(xc3_wgpu::load_collisions(&device, &collision_meshes));
                }
                ext => return Err(anyhow!(format!("unrecognized file extension {ext}"))),
            }
        }

        // Disable instancing if we only want to render a single model.
        if cli.model.is_some() {
            for root in &mut map_roots {
                for group in &mut root.groups {
                    for models in &mut group.models {
                        for model in &mut models.models {
                            model.instances = vec![Mat4::IDENTITY];
                        }
                    }
                }
            }
        }

        if !model_roots.is_empty() || !map_roots.is_empty() {
            info!(
                "Load {} roots: {:?}",
                model_roots.len() + map_roots.len(),
                start.elapsed()
            );
        }
        if !collisions.is_empty() {
            info!(
                "Load {} collisions: {:?}",
                collisions.len(),
                start.elapsed()
            );
        }

        let start = std::time::Instant::now();

        if !model_roots.is_empty() {
            groups.extend(xc3_wgpu::load_model(
                &device,
                &queue,
                &model_roots,
                &monolib_shader,
            ));
        }
        if !map_roots.is_empty() {
            groups.extend(xc3_wgpu::load_map(
                &device,
                &queue,
                &map_roots,
                &monolib_shader,
            ));
        }

        let mesh_count: usize = groups
            .iter()
            .map(|m| {
                m.models
                    .iter()
                    .flat_map(|models| models.models.iter().map(|model| model.meshes.len()))
                    .sum::<usize>()
            })
            .sum();
        if !groups.is_empty() || mesh_count > 0 {
            info!(
                "Load {:?} groups and {:?} meshes: {:?}",
                groups.len(),
                mesh_count,
                start.elapsed()
            );
        }

        let file_names = cli
            .files
            .iter()
            .map(|m| {
                Path::new(m)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(" ");

        let start = std::time::Instant::now();
        let animations = match &cli.anim {
            Some(p) => load_animations(p)
                .with_context(|| format!("{p:?} is not a valid animation file"))?,
            None => Vec::new(),
        };
        if !animations.is_empty() {
            info!(
                "Load {} animations: {:?}",
                animations.len(),
                start.elapsed()
            );
        }
        let animation_index = cli.anim_index.unwrap_or_default();

        // Filter the groups to render ahead of time.
        let groups: Vec<_> = groups
            .into_iter()
            .filter(|g| {
                cli.root.map(|i| g.root_index == i).unwrap_or(true)
                    && cli.group.map(|i| g.group_index == i).unwrap_or(true)
            })
            .collect();

        Ok(Self {
            window,
            surface,
            device,
            queue,
            size,
            config,
            translation,
            rotation_xyz,
            model_names: file_names,
            groups,
            collisions,
            renderer,
            animations,
            animation_index,
            current_time_seconds: 0.0,
            input_state: Default::default(),
            previous_frame_start: Instant::now(),
            draw_bones: cli.bones,
            draw_bounds: cli.bounds,
            root_index: cli.root,
            group_index: cli.group,
            models_index: cli.models,
            model_index: cli.model,
            render_mode: RenderMode::Shaded,
            movement: cli.movement,
            movement_speed: cli.movement_speed,
            mouse_sensitivity: cli.mouse_sensitivity,
        })
    }

    fn update_camera(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let camera_data = calculate_camera_data(size, self.translation, self.rotation_xyz, self.movement);
        self.renderer.update_camera(&self.queue, &camera_data);
    }

    fn update_debug_settings(&mut self, render_mode: RenderMode, channel: i32) {
        self.render_mode = render_mode;
        self.renderer
            .update_debug_settings(&self.queue, render_mode, channel);
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

    fn render(&mut self, output: wgpu::SurfaceTexture) {
        let now = std::time::Instant::now();
        let delta_t = now
            .duration_since(self.previous_frame_start)
            .as_secs_f64() as f32;
        self.previous_frame_start = now;

        if self.movement {
            let speed = self.movement_speed * delta_t;
            let (forward, right) = camera_directions(self.rotation_xyz.x, self.rotation_xyz.y);

            if self.input_state.move_forward  { self.translation += forward * speed; }
            if self.input_state.move_backward { self.translation -= forward * speed; }
            if self.input_state.move_right    { self.translation += right   * speed; }
            if self.input_state.move_left     { self.translation -= right   * speed; }
            if self.input_state.move_up       { self.translation.y += speed; }
            if self.input_state.move_down     { self.translation.y -= speed; }

            if self.input_state.move_forward
                || self.input_state.move_backward
                || self.input_state.move_right
                || self.input_state.move_left
                || self.input_state.move_up
                || self.input_state.move_down
            {
                self.update_camera(self.size);
            }
        }

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
                group.update_morph_weights(&self.queue, anim, self.current_time_seconds);
            }
        }

        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.renderer.render_models(
            &output_view,
            &mut encoder,
            &self.groups,
            &self.collisions,
            self.draw_bounds,
            self.draw_bones,
            self.models_index,
            self.model_index,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    // Make this a reusable library that only requires glam?
    fn handle_input(&mut self, event: &WindowEvent) {
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
                        NamedKey::Space => {
                            if event.state == ElementState::Released {
                                self.current_time_seconds = 0.0;
                            }
                        }
                        _ => (),
                    },
                    winit::keyboard::Key::Character(c) => {
                        let pressed  = event.state == ElementState::Pressed;

                        if self.movement {
                            match c.as_str() {
                                "w" | "W" => { self.input_state.move_forward  = pressed; return; }
                                "s" | "S" => { self.input_state.move_backward = pressed; return; }
                                "a" | "A" => { self.input_state.move_left     = pressed; return; }
                                "d" | "D" => { self.input_state.move_right    = pressed; return; }
                                "q" | "Q" => { self.input_state.move_down     = pressed; return; }
                                "e" | "E" => { self.input_state.move_up       = pressed; return; }
                                _ => {}
                            }
                        }

                        match c.as_str() {
                            // Debug a selected G-Buffer texture.
                            // This also resets the color channel to all channels.
                            "0" => self.update_debug_settings(RenderMode::Shaded, -1),
                            "1" => self.update_debug_settings(RenderMode::GBuffer0, -1),
                            "2" => self.update_debug_settings(RenderMode::GBuffer1, -1),
                            "3" => self.update_debug_settings(RenderMode::GBuffer2, -1),
                            "4" => self.update_debug_settings(RenderMode::GBuffer3, -1),
                            "5" => self.update_debug_settings(RenderMode::GBuffer4, -1),
                            "6" => self.update_debug_settings(RenderMode::GBuffer5, -1),
                            "7" => self.update_debug_settings(RenderMode::GBuffer6, -1),
                            // Debug selected color channel.
                            "r" | "x" => self.update_debug_settings(self.render_mode, 0),
                            "g" | "y" => self.update_debug_settings(self.render_mode, 1),
                            "b" | "z" => self.update_debug_settings(self.render_mode, 2),
                            "a" | "w" => self.update_debug_settings(self.render_mode, 3),
                            // Animation playback.
                            "." => {
                                if event.state == ElementState::Released {
                                    self.current_time_seconds = 0.0;
                                    self.animation_index += 1;
                                    self.set_window_title();
                                }
                            }
                            "," => {
                                if event.state == ElementState::Released {
                                    self.current_time_seconds = 0.0;
                                    self.animation_index = self.animation_index.saturating_sub(1);
                                    self.set_window_title();
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
                let dx = (position.x - self.input_state.previous_cursor_position.x) as f32;
                let dy = (position.y - self.input_state.previous_cursor_position.y) as f32;

                if self.input_state.is_mouse_left_clicked {
                    if self.movement {
                        self.rotation_xyz.y += dx * 0.01 * self.mouse_sensitivity;
                        self.rotation_xyz.x += dy * 0.01 * self.mouse_sensitivity;
                        self.rotation_xyz.x = self.rotation_xyz.x.clamp(-MAX_PITCH, MAX_PITCH);
                    } else {
                        self.rotation_xyz.x += (dy * 0.01) as f32;
                        self.rotation_xyz.y += (dx * 0.01) as f32;
                    }
                } else if self.input_state.is_mouse_right_clicked {
                    if self.movement {
                        let (_forward, right) =
                            camera_directions(self.rotation_xyz.x, self.rotation_xyz.y);
                        // Derive camera up from right × forward (already normalized).
                        let (_fwd, _r) = camera_directions(self.rotation_xyz.x, self.rotation_xyz.y);
                        let fac = self.movement_speed * 0.002;
                        self.translation -= right * dx * fac;
                        self.translation.y += dy * fac;
                    } else {
                        let fac = FOV_Y.sin() * self.translation.z.abs() / self.size.height as f32;
                        self.translation.x += dx * fac;
                        self.translation.y -= dy * fac;
                    }
                }

                self.input_state.previous_cursor_position = *position;
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => *y,
                    MouseScrollDelta::PixelDelta(p)    => p.y as f32 * 0.05,
                };

                if self.movement {
                    let (forward, _) = camera_directions(self.rotation_xyz.x, self.rotation_xyz.y);
                    self.translation += forward * scroll * self.movement_speed * 0.5;
                } else {
                    self.translation.z += scroll * self.translation.z.abs() * 0.1;
                }
            }

            _ => (),
        }
    }

    fn set_window_title(&self) {
        let mut title = if let Some(anim) = self.animations.get(self.animation_index) {
            format!(
                "{} - {} - {}",
                concat!("xc3_viewer ", env!("CARGO_PKG_VERSION")),
                self.model_names,
                anim.name
            )
        } else {
            format!(
                "{} - {}",
                concat!("xc3_viewer ", env!("CARGO_PKG_VERSION")),
                self.model_names,
            )
        };
        if let Some(i) = self.root_index {
            title = format!("{title} root {i}");
        }
        if let Some(i) = self.group_index {
            title = format!("{title} group {i}");
        }
        if let Some(i) = self.models_index {
            title = format!("{title} models {i}");
        }
        if let Some(i) = self.model_index {
            title = format!("{title} model {i}");
        }

        self.window.set_title(&title);
    }
}

// TODO: Move to xc3_wgpu?
fn calculate_camera_data(
    size: winit::dpi::PhysicalSize<u32>,
    translation: glam::Vec3,
    rotation: glam::Vec3,
    movement: bool,
) -> CameraData {
    let aspect = size.width as f32 / size.height as f32;

    let view = if movement {
        let yaw   = rotation.y;
        let pitch = rotation.x;
        Mat4::from_rotation_x(pitch)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_translation(-translation)
    } else {
        Mat4::from_translation(translation)
            * Mat4::from_rotation_x(rotation.x)
            * Mat4::from_rotation_y(rotation.y)
    };

    let projection = glam::Mat4::perspective_rh(FOV_Y, aspect, Z_NEAR, Z_FAR);

    let view_projection = projection * view;

    let position = view.inverse().col(3);

    CameraData {
        view,
        projection,
        view_projection,
        position,
        width: size.width,
        height: size.height,
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

fn database_path() -> std::io::Result<std::path::PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .unwrap_or(Path::new(""))
        .join("xc_combined.bin"))
}

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// The wimdo, wismhd, camdo, wiidcm, or idcm files.
    files: Vec<String>,
    /// The shader database generated by xc3_shader.
    #[arg(long)]
    database: Option<String>,
    /// The .mot animation file.
    #[arg(long)]
    anim: Option<String>,
    /// The BC entry index for the ANIM. Defaults to 0.
    #[arg(long)]
    anim_index: Option<usize>,
    /// Draw axes for each bone in the skeleton.
    #[arg(long)]
    bones: bool,
    /// Draw model bounding boxes.
    #[arg(long)]
    bounds: bool,
    /// Override for the graphics backend.
    #[arg(
        long,
        value_parser = clap::builder::PossibleValuesParser::new(
        ["dx12", "vulkan", "metal"]
    ))]
    backend: Option<String>,
    /// Index for the wimdo or camdo or root in a wismhd to render.
    /// If not specified, all roots will be rendered.
    #[arg(long)]
    root: Option<usize>,
    /// Index for the group of model collections to render.
    /// If not specified, all groups will be rendered.
    #[arg(long)]
    group: Option<usize>,
    /// Index for the model collections to render.
    /// If not specified, all model collections will be rendered.
    #[arg(long)]
    models: Option<usize>,
    /// Index for the model to render.
    /// If not specified, all models will be rendered.
    #[arg(long)]
    model: Option<usize>,

    /// Enable freecam: W/S = forward/back, A/D = strafe, Q/E = down/up.
    /// Left-drag to look around, right-drag to pan, scroll to dolly.
    #[arg(long)]
    movement: bool,

    /// Freecam movement speed in units per second (default: 10).
    /// Use a higher value for large maps, lower for small models.
    ///   e.g. --movement-speed 50
    #[arg(long, default_value_t = 10.0)]
    movement_speed: f32,

    /// Mouse look sensitivity multiplier for freecam (default: 0.3).
    /// Does not affect orbit mode. Range 0.1 (slow) – 2.0 (fast).
    ///   e.g. --mouse-sensitivity 0.5
    #[arg(long, default_value_t = 0.3)]
    mouse_sensitivity: f32,
}

struct App {
    state: Option<State>,
    cli: Cli,
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(concat!("xc3_viewer ", env!("CARGO_PKG_VERSION"))),
            )
            .unwrap();

        self.state = block_on(State::new(window, &self.cli, event_loop)).ok();
        if let Some(state) = &self.state {
            state.set_window_title();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if event == WindowEvent::CloseRequested {
            event_loop.exit();
            return;
        };

        // Window specific event handling.
        if let Some(state) = self.state.as_mut() {
            if window_id != state.window.id() {
                return;
            }

            match event {
                WindowEvent::Resized(physical_size) => {
                    state.resize(physical_size);
                    state.update_camera(physical_size);
                    state.window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {}
                WindowEvent::RedrawRequested => {
                    match state.surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(output) => state.render(output),
                        wgpu::CurrentSurfaceTexture::Suboptimal(_)   => state.resize(state.size),
                        wgpu::CurrentSurfaceTexture::Timeout         => {}
                        wgpu::CurrentSurfaceTexture::Occluded        => {}
                        wgpu::CurrentSurfaceTexture::Outdated        => state.resize(state.size),
                        wgpu::CurrentSurfaceTexture::Lost            => state.resize(state.size),
                        wgpu::CurrentSurfaceTexture::Validation      => {}
                    }
                    state.window.request_redraw();
                }
                _ => {
                    state.handle_input(&event);
                    state.update_camera(state.window.inner_size());
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    // TODO: Use tracing instead of log or convert log to tracing events?
    // Ignore most logs to avoid flooding the console.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("wgpu", log::LevelFilter::Warn)
        .with_module_level("naga", log::LevelFilter::Warn)
        .init()
        .unwrap();

    // TODO: layer to print log messages?
    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .unwrap();

    let cli = Cli::parse();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App { state: None, cli };
    event_loop
        .run_app(&mut app)
        .with_context(|| "failed to complete event loop")?;
    Ok(())
}
