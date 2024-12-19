use glam::vec3;
use log::info;
use wgpu::util::DeviceExt;

pub struct Collision {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

pub fn load_collisions(
    device: &wgpu::Device,
    collision_meshes: &xc3_model::collision::CollisionMeshes,
) -> Vec<Collision> {
    let start = std::time::Instant::now();

    let mut collisions = Vec::new();
    for mesh in &collision_meshes.meshes {
        if !mesh.instances.is_empty() {
            for instance in &mesh.instances {
                // TODO: Separate shader with instanced rendering to share buffers
                let vertices: Vec<_> = collision_meshes
                    .vertices
                    .iter()
                    .map(|v| {
                        instance
                            .transform_point3(vec3(v[0], v[1], v[2]))
                            .extend(0.0)
                    })
                    .collect();
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("collision vertex buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("collision index buffer"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                collisions.push(Collision {
                    vertex_buffer,
                    index_buffer,
                    index_count: mesh.indices.len() as u32,
                });
            }
        } else {
            // TODO: Not all collsion meshes are instanced?
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

            collisions.push(Collision {
                vertex_buffer,
                index_buffer,
                index_count: mesh.indices.len() as u32,
            });
        }
    }

    info!("Load {} collision: {:?}", collisions.len(), start.elapsed());

    collisions
}

impl Collision {
    pub fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        bind_group1: &crate::shader::solid::bind_groups::BindGroup1,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        bind_group1.set(render_pass);

        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
