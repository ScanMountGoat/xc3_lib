use glam::{vec4, Mat4, Vec3};
use wgpu::util::DeviceExt;

pub struct Bounds {
    pub max_xyz: Vec3,
    pub min_xyz: Vec3,
    bounds_vertex_buffer: wgpu::Buffer,
    bounds_index_buffer: wgpu::Buffer,
}

impl Bounds {
    pub fn new(device: &wgpu::Device, max_xyz: Vec3, min_xyz: Vec3, transform: &Mat4) -> Self {
        let (bounds_vertex_buffer, bounds_index_buffer) =
            wireframe_aabb_box_vertex_index(device, min_xyz, max_xyz, transform);

        // TODO: include transform in the min/max xyz values.
        Self {
            max_xyz,
            min_xyz,
            bounds_vertex_buffer,
            bounds_index_buffer,
        }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        culled: bool,
        bind_group1: &'a crate::shader::solid::bind_groups::BindGroup1,
        culled_bind_group1: &'a crate::shader::solid::bind_groups::BindGroup1,
    ) {
        render_pass.set_vertex_buffer(0, self.bounds_vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.bounds_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        if culled {
            culled_bind_group1.set(render_pass);
        } else {
            bind_group1.set(render_pass);
        }

        // 12 lines with 2 points each.
        render_pass.draw_indexed(0..24, 0, 0..1);
    }
}

fn wireframe_aabb_box_vertex_index(
    device: &wgpu::Device,
    min_xyz: Vec3,
    max_xyz: Vec3,
    transform: &Mat4,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let corners = [
        vec4(min_xyz.x, min_xyz.y, min_xyz.z, 1.0),
        vec4(max_xyz.x, min_xyz.y, min_xyz.z, 1.0),
        vec4(max_xyz.x, max_xyz.y, min_xyz.z, 1.0),
        vec4(min_xyz.x, max_xyz.y, min_xyz.z, 1.0),
        vec4(min_xyz.x, min_xyz.y, max_xyz.z, 1.0),
        vec4(max_xyz.x, min_xyz.y, max_xyz.z, 1.0),
        vec4(max_xyz.x, max_xyz.y, max_xyz.z, 1.0),
        vec4(min_xyz.x, max_xyz.y, max_xyz.z, 1.0),
    ]
    .map(|c| *transform * c);

    let bounds_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bounds vertex buffer"),
        contents: bytemuck::cast_slice(&corners),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let bounds_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bounds index buffer"),
        contents: bytemuck::cast_slice(&[
            [0u16, 1u16],
            [1u16, 2u16],
            [2u16, 3u16],
            [3u16, 0u16],
            [0u16, 4u16],
            [1u16, 5u16],
            [2u16, 6u16],
            [3u16, 7u16],
            [4u16, 5u16],
            [5u16, 6u16],
            [6u16, 7u16],
            [7u16, 4u16],
        ]),
        usage: wgpu::BufferUsages::INDEX,
    });

    (bounds_vertex_buffer, bounds_index_buffer)
}
