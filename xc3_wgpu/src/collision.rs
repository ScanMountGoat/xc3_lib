use std::path::Path;

use glam::{vec3, Mat4};
use log::info;
use wgpu::util::DeviceExt;
use xc3_lib::idcm::Idcm;

pub struct Collision {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

// TODO: take an xc3_model type instead.
// TODO: convert to triangle list in xc3_model?
pub fn load_collisions<P: AsRef<Path>>(device: &wgpu::Device, path: P) -> Vec<Collision> {
    let start = std::time::Instant::now();

    let idcm = Idcm::from_file(path).unwrap();

    let mut mesh_indices = Vec::new();
    for mesh in &idcm.meshes {
        let mut indices = Vec::new();
        for group in idcm
            .face_groups
            .iter()
            .skip(mesh.face_group_start_index as usize)
            .take(mesh.face_group_count as usize)
        {
            for faces in &group.faces.triangle_strips {
                indices.extend_from_slice(faces);
            }
        }

        mesh_indices.push(indices);
    }

    let mut instances = vec![Vec::new(); idcm.meshes.len()];
    for ((index, _), transform) in idcm
        .instances
        .mesh_indices
        .iter()
        .zip(&idcm.instances.transforms)
    {
        // Transforms are row-major instead of the typical column-major.
        instances[*index as usize].push(Mat4::from_cols_array_2d(&transform.transform).transpose());
    }

    let mut collisions = Vec::new();
    for (indices, instances) in mesh_indices.iter().zip(&instances) {
        if !instances.is_empty() {
            for instance in instances {
                // TODO: Separate shader with instanced rendering to share buffers
                let vertices: Vec<_> = idcm
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
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                collisions.push(Collision {
                    vertex_buffer,
                    index_buffer,
                    index_count: indices.len() as u32,
                });
            }
        } else {
            // TODO: Not all collsion meshes are instanced?
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("collision vertex buffer"),
                contents: bytemuck::cast_slice(&idcm.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("collision index buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            collisions.push(Collision {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
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
