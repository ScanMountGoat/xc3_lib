use std::path::Path;

use glam::{Mat4, Vec4};
use xc3_lib::idcm::Idcm;

use crate::error::LoadCollisionsError;

#[derive(Debug, PartialEq, Clone)]
pub struct CollisionMeshes {
    /// Shared XYZ vertices for each mesh with an unused fourth component.
    pub vertices: Vec<Vec4>,
    pub meshes: Vec<CollisionMesh>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct CollisionMesh {
    pub name: String,

    /// Transform for each instance or an empty list if this mesh has only a single instance.
    pub instances: Vec<Mat4>,

    /// Triangle list vertex indices.
    pub indices: Vec<u32>,
}

/// Load all collisions from a `.wiidcm` or `.idcm` file.
///
/// # Examples
/// ``` rust no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let collisions = xc3_model::load_collisions("xeno1/map/ma0101.wiidcm");
/// let collisions = xc3_model::load_collisions("xeno2/map/ma01a.wiidcm");
/// let collisions = xc3_model::load_collisions("xeno3/map/ma59a.idcm");
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(skip_all)]
pub fn load_collisions<P: AsRef<Path>>(
    idcm_path: P,
) -> Result<CollisionMeshes, LoadCollisionsError> {
    let idcm = Idcm::from_file(idcm_path)?;

    let mut meshes: Vec<_> = idcm
        .meshes
        .into_iter()
        .zip(idcm.mesh_names)
        .map(|(mesh, name)| {
            let mut indices = Vec::new();

            let (start, count) = match mesh {
                xc3_lib::idcm::MeshVersioned::MeshLegacy(m) => {
                    (m.face_group_start_index, m.face_group_count)
                }
                xc3_lib::idcm::MeshVersioned::Mesh(m) => {
                    (m.face_group_start_index, m.face_group_count)
                }
            };

            // Each fan needs to be handled individually.
            for group in idcm
                .face_groups
                .iter()
                .skip(start as usize)
                .take(count as usize)
            {
                let start = idcm.groups[group.group_index as usize].start_index;

                // Convert to triangle lists with the correct winding order.
                for i in 0..group.faces.vertex_indices.len().saturating_sub(2) {
                    // 0 1 2 3 ... -> (0, 1, 2) (2, 1, 3) ...
                    // https://registry.khronos.org/VulkanSC/specs/1.0-extensions/html/vkspec.html#drawing-triangle-fans
                    indices.extend_from_slice(&[
                        group.faces.vertex_indices[i + 1] as u32 + start,
                        group.faces.vertex_indices[i + 2] as u32 + start,
                        group.faces.vertex_indices[0] as u32 + start,
                    ]);
                }
            }

            CollisionMesh {
                name: name.name,
                instances: Vec::new(),
                indices,
            }
        })
        .collect();

    for ((index, _), transform) in idcm
        .instances
        .mesh_indices
        .iter()
        .zip(&idcm.instances.transforms)
    {
        // Transforms are row-major instead of the typical column-major.
        meshes[*index as usize]
            .instances
            .push(Mat4::from_cols_array_2d(&transform.transform).transpose());
    }

    Ok(CollisionMeshes {
        vertices: idcm.vertices.into_iter().map(Into::into).collect(),
        meshes,
    })
}
