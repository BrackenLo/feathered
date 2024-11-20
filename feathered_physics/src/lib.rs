//====================================================================

use std::collections::HashSet;

use feathered_render_tools::shared::ModelVertex;
use feathered_spatial::GlobalTransform;
use shipyard::Component;

//====================================================================

/// For use with 3d
#[derive(Component, Debug)]
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

// https://winter.dev/articles/gjk-algorithm
// https://www.youtube.com/watch?v=ajv46BSqcK4

pub fn check_gjk(
    mesh_a: (&CollisionMesh, &glam::Vec3),
    mesh_b: (&CollisionMesh, &glam::Vec3),
) -> bool {
    let start_dir = (mesh_b.1 - mesh_a.1).normalize();

    let mut simplex = Simplex::default();

    let a = get_support_point(mesh_a, mesh_b, start_dir);
    simplex.points.push(a);

    let mut next_dir = -a.normalize();

    loop {
        let support = get_support_point(mesh_a, mesh_b, next_dir);

        if support.dot(next_dir) <= 0. {
            return false;
        }

        simplex.push_front(support);

        if simplex.next(&mut next_dir) {
            return true;
        }
    }
}

#[inline]
fn get_support_point(
    a: (&CollisionMesh, &glam::Vec3),
    b: (&CollisionMesh, &glam::Vec3),
    dir: glam::Vec3,
) -> glam::Vec3 {
    (a.0.find_furthest_point(dir) + a.1) - (b.0.find_furthest_point(-dir) + b.1)
}

#[inline]
fn check_same_direction(direction: glam::Vec3, ao: glam::Vec3) -> bool {
    direction.dot(ao) > 0.
}

#[derive(Default)]
struct Simplex {
    points: Vec<glam::Vec3>,
}

impl Simplex {
    fn push_front(&mut self, point: glam::Vec3) {
        self.points.insert(0, point);
        self.points.truncate(4);
    }

    fn next(&mut self, direction: &mut glam::Vec3) -> bool {
        match self.points.len() {
            2 => self.line(direction),
            3 => self.triangle(direction),
            4 => self.tetrahedron(direction),
            _ => panic!(""),
        }
    }

    fn line(&mut self, direction: &mut glam::Vec3) -> bool {
        let a = self.points[0];
        let b = self.points[1];

        let ab = b - a;
        let ao = -a;

        match check_same_direction(ab, ao) {
            true => *direction = ab.cross(ao).cross(ab),
            false => {
                self.points = vec![a];
                *direction = ao;
            }
        }

        false
    }

    fn triangle(&mut self, direction: &mut glam::Vec3) -> bool {
        let a = self.points[0];
        let b = self.points[1];
        let c = self.points[2];

        let ab = b - a;
        let ac = c - a;
        let ao = -a;

        let abc = ab.cross(ac);

        match check_same_direction(abc.cross(ac), ao) {
            true => match check_same_direction(ac, ao) {
                true => {
                    self.points = vec![a, c];
                    *direction = ac.cross(ao).cross(ac);
                    false
                }

                false => self.line(direction),
            },

            false => match check_same_direction(ab.cross(abc), ao) {
                true => self.line(direction),

                false => {
                    match check_same_direction(abc, ao) {
                        true => *direction = abc,
                        false => {
                            self.points = vec![a, c, b];
                            *direction = -abc;
                        }
                    };
                    false
                }
            },
        }
    }

    fn tetrahedron(&mut self, direction: &mut glam::Vec3) -> bool {
        let a = self.points[0];
        let b = self.points[1];
        let c = self.points[2];
        let d = self.points[3];

        let ab = b - a;
        let ac = c - a;
        let ad = d - a;
        let ao = -a;

        let abc = ab.cross(ac);
        let acd = ac.cross(ad);
        let adb = ad.cross(ab);

        if check_same_direction(abc, ao) {
            self.points = vec![a, b, c];
            return self.triangle(direction);
        }

        if check_same_direction(acd, ao) {
            self.points = vec![a, c, d];
            return self.triangle(direction);
        }

        if check_same_direction(adb, ao) {
            self.points = vec![a, d, b];
            return self.triangle(direction);
        }

        true
    }
}
