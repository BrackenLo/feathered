//====================================================================

use std::collections::HashSet;

use feathered_render_tools::shared::ModelVertex;
use shipyard::Component;

pub mod aabb;
pub mod gjk;

//====================================================================

#[derive(Component, Debug)]
#[track(Insertion)]
#[track(Modification)]
pub struct CollisionMesh {
    points: Vec<glam::Vec3>,
}

impl CollisionMesh {
    pub fn from_model_vertices(vertices: &[ModelVertex]) -> Option<Self> {
        let points = vertices
            .iter()
            .map(|vertex| {
                let x = ordered_float::OrderedFloat(vertex.pos.x);
                let y = ordered_float::OrderedFloat(vertex.pos.y);
                let z = ordered_float::OrderedFloat(vertex.pos.z);

                (x, y, z)
            })
            .collect::<HashSet<_>>();

        log::trace!(
            "Loading collision mesh from vertices: vertex count was {}. Reduced to {}",
            vertices.len(),
            points.len()
        );

        let points = points
            .into_iter()
            .map(|(x, y, z)| glam::vec3(x.0, y.0, z.0))
            .collect::<Vec<_>>();

        match points.is_empty() {
            true => None,
            false => Some(Self { points }),
        }
    }

    pub fn find_furthest_point(&self, direction: glam::Vec3) -> glam::Vec3 {
        let direction = direction.normalize();

        let index = self
            .points
            .iter()
            .enumerate()
            .map(|(index, point)| (index, point.dot(direction)))
            .reduce(|a, b| match a.1 > b.1 {
                true => a,
                false => b,
            });

        match index {
            Some((index, _)) => self.points[index],
            None => {
                log::warn!("Collision mesh has no valid points");
                glam::Vec3::ZERO
            }
        }
    }
}

//====================================================================
