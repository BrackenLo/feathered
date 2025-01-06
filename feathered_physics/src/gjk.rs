//====================================================================

use std::collections::HashSet;

use feathered_spatial::GlobalTransform;
use shipyard::{track, Component, Get, IntoIter, IntoWithId, View, ViewMut};

use crate::CollisionMesh;

//====================================================================

fn sys_rebuild_built_complex_collision(
    v_global: View<GlobalTransform>,
    v_collision: View<CollisionMesh, track::InsertionAndModification>,
    mut vm_built_collision: ViewMut<BuiltComplexCollisionMesh>,
) {
    // Get entities to be rebuilt
    let rebuild = (
        v_global.inserted_or_modified(),
        &v_collision,
        &vm_built_collision,
    )
        .iter()
        .with_id()
        .map(|(id, _)| id)
        .collect::<HashSet<_>>();

    let rebuild = (
        &v_global,
        v_collision.inserted_or_modified(),
        &vm_built_collision,
    )
        .iter()
        .with_id()
        .fold(rebuild, |mut acc, (id, _)| {
            acc.insert(id);
            acc
        });

    // Rebuild collision meshs'
    rebuild.into_iter().for_each(|id| {
        let (global, collision, mut built) = (&v_global, &v_collision, &mut vm_built_collision)
            .get(id)
            .unwrap();

        if collision.points.len() != built.built.len() {
            // fix size
            todo!()
        }

        let matrix = global.to_matrix();

        collision
            .points
            .iter()
            .zip(built.built.iter_mut())
            .for_each(|(original, built)| {
                let built_point = matrix * original.extend(1.);
                *built = built_point.truncate();
            });
    });
}

//====================================================================

pub trait ComplexCollisionMeshAccess {
    fn find_furthest_point(&self, direction: glam::Vec3) -> glam::Vec3;
}

//--------------------------------------------------

#[derive(Component, Debug)]
pub struct BuiltComplexCollisionMesh {
    built: Vec<glam::Vec3>,
}

impl ComplexCollisionMeshAccess for BuiltComplexCollisionMesh {
    fn find_furthest_point(&self, direction: glam::Vec3) -> glam::Vec3 {
        let mut furthest_index = 0;
        let mut furthest_distance = 0.;

        self.built.iter().enumerate().for_each(|(index, point)| {
            let distance = point.dot(direction);

            if distance > furthest_distance {
                furthest_index = index;
                furthest_distance = distance;
            }
        });

        self.built[furthest_index]
    }
}

//--------------------------------------------------

pub struct ComplexCollisionMeshTransform<'a> {
    pub mesh: &'a CollisionMesh,
    pub transform_matrix: glam::Mat4,
}

impl<'a> ComplexCollisionMeshTransform<'a> {
    #[inline]
    pub fn from_global(mesh: &'a CollisionMesh, global: &GlobalTransform) -> Self {
        Self {
            mesh,
            transform_matrix: global.to_matrix(),
        }
    }
}

impl<'a> ComplexCollisionMeshAccess for ComplexCollisionMeshTransform<'a> {
    // FIX - rotation should be applied first
    fn find_furthest_point(&self, direction: glam::Vec3) -> glam::Vec3 {
        let mut furthest_index = 0;
        let mut furthest_distance = 0.;

        self.mesh
            .points
            .iter()
            .enumerate()
            .for_each(|(index, point)| {
                let distance = point.dot(direction);

                if distance > furthest_distance {
                    furthest_index = index;
                    furthest_distance = distance;
                }
            });

        let furthest_point = self.mesh.points[furthest_index];

        let transformed_point = self.transform_matrix * furthest_point.extend(1.);
        transformed_point.truncate()
    }
}

//====================================================================

// https://winter.dev/articles/gjk-algorithm
// https://www.youtube.com/watch?v=ajv46BSqcK4

pub fn check_gjk<M1, M2>(mesh_a: M1, mesh_b: M2, start_dir: impl Into<glam::Vec3>) -> bool
where
    M1: ComplexCollisionMeshAccess,
    M2: ComplexCollisionMeshAccess,
{
    let mut simplex = Simplex::default();

    let start_dir = start_dir.into().normalize();

    let a = mesh_a.find_furthest_point(start_dir) - mesh_b.find_furthest_point(-start_dir);
    simplex.points.push(a);

    let mut next_dir = -a.normalize();

    loop {
        let support = mesh_a.find_furthest_point(next_dir) - mesh_b.find_furthest_point(-next_dir);

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
fn check_same_direction(direction: glam::Vec3, ao: glam::Vec3) -> bool {
    direction.dot(ao) > 0.
}

#[derive(Default)]
struct Simplex {
    points: Vec<glam::Vec3>,
}

impl Simplex {
    #[inline]
    fn push_front(&mut self, point: glam::Vec3) {
        self.points.insert(0, point);
        self.points.truncate(4);
    }

    #[inline]
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
//====================================================================
