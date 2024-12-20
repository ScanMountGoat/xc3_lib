use glam::Mat4;
use log::info;
use wgpu::util::DeviceExt;

pub struct Collision {
    vertex_buffer: wgpu::Buffer,

    index_buffer: wgpu::Buffer,
    index_count: u32,

    instance_buffer: wgpu::Buffer,
    instance_count: u32,
}

pub fn load_collisions(
    device: &wgpu::Device,
    collision_meshes: &xc3_model::collision::CollisionMeshes,
) -> Vec<Collision> {
    let start = std::time::Instant::now();

    let mut collisions = Vec::new();
    for mesh in &collision_meshes.meshes {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("collision vertex buffer"),
            contents: bytemuck::cast_slice(&collision_meshes.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("collision index buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_transforms = if !mesh.instances.is_empty() {
            mesh.instances.as_slice()
        } else {
            &[Mat4::IDENTITY]
        };
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("collision instance buffer"),
            contents: bytemuck::cast_slice(&instance_transforms),
            usage: wgpu::BufferUsages::VERTEX,
        });

        collisions.push(Collision {
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            instance_buffer,
            instance_count: instance_transforms.len() as u32,
        });
    }

    info!("Load {} collision: {:?}", collisions.len(), start.elapsed());

    collisions
}

impl Collision {
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..self.index_count, 0, 0..self.instance_count);
    }
}
