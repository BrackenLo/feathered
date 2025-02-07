//====================================================================

use feathered_shipyard::prelude::*;
use shipyard::{Component, IntoIter};

//====================================================================

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder.add_workload_last(Update, sys_update_global);
    }
}

fn sys_update_global(v_transform: View<Transform>, mut vm_global: ViewMut<GlobalTransform>) {
    (v_transform.inserted_or_modified(), &mut vm_global)
        .iter()
        .for_each(|(transform, mut global)| global.0 = transform.to_affine());
}

//====================================================================

#[derive(Component, Debug, Default, Clone, PartialEq)]
#[track(All)]
pub struct GlobalTransform(pub glam::Affine3A);

impl GlobalTransform {
    #[inline]
    pub fn to_matrix(&self) -> glam::Mat4 {
        self.0.into()
    }

    #[inline]
    pub fn translation(&self) -> glam::Vec3 {
        self.0.translation.into()
    }

    #[inline]
    pub fn to_scale_rotation_translation(&self) -> (glam::Vec3, glam::Quat, glam::Vec3) {
        self.0.to_scale_rotation_translation()
    }
}

impl From<&GlobalTransform> for glam::Mat4 {
    #[inline]
    fn from(value: &GlobalTransform) -> Self {
        value.0.into()
    }
}

impl From<&GlobalTransform> for glam::Affine3A {
    #[inline]
    fn from(value: &GlobalTransform) -> Self {
        value.0
    }
}

//====================================================================

#[derive(Component, Clone, Debug, PartialEq)]
#[track(All)]
pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        }
    }
}

impl Transform {
    #[inline]
    pub fn from_translation(translation: impl Into<glam::Vec3>) -> Self {
        let translation = translation.into();
        Self {
            translation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation(rotation: impl Into<glam::Quat>) -> Self {
        let rotation = rotation.into();
        Self {
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale(scale: impl Into<glam::Vec3>) -> Self {
        let scale = scale.into();
        Self {
            scale,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation_translation(
        rotation: impl Into<glam::Quat>,
        translation: impl Into<glam::Vec3>,
    ) -> Self {
        let rotation = rotation.into();
        let translation = translation.into();
        Self {
            translation,
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale_translation(
        scale: impl Into<glam::Vec3>,
        translation: impl Into<glam::Vec3>,
    ) -> Self {
        let translation = translation.into();
        let scale = scale.into();
        Self {
            scale,
            translation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale_rotation(
        scale: impl Into<glam::Vec3>,
        rotation: impl Into<glam::Quat>,
    ) -> Self {
        let scale = scale.into();
        let rotation = rotation.into();
        Self {
            scale,
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale_rotation_translation(
        scale: impl Into<glam::Vec3>,
        rotation: impl Into<glam::Quat>,
        translation: impl Into<glam::Vec3>,
    ) -> Self {
        let scale = scale.into();
        let rotation = rotation.into();
        let translation = translation.into();
        Self {
            scale,
            rotation,
            translation,
        }
    }
}

impl Transform {
    pub fn look_to(&mut self, direction: impl Into<glam::Vec3>, up: impl Into<glam::Vec3>) {
        let back = -direction.into().normalize();
        let up = up.into();

        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthogonal_vector());
        let up = back.cross(right);
        self.rotation = glam::Quat::from_mat3(&glam::Mat3::from_cols(right, up, back));
    }

    #[inline]
    pub fn look_at(&mut self, target: impl Into<glam::Vec3>, up: impl Into<glam::Vec3>) {
        self.look_to(target.into() - self.translation, up);
    }

    #[inline]
    pub fn forward(&self) -> glam::Vec3 {
        (self.rotation * glam::Vec3::Z).normalize_or_zero()
    }

    #[inline]
    pub fn right(&self) -> glam::Vec3 {
        (self.rotation * glam::Vec3::X).normalize_or_zero()
    }

    #[inline]
    pub fn lerp(&mut self, target: &Transform, s: f32) {
        self.translation = self.translation.lerp(target.translation, s);
        self.rotation = self.rotation.lerp(target.rotation, s);
        self.scale = self.scale.lerp(target.scale, s);
    }
}

impl Transform {
    #[inline]
    pub fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    #[inline]
    pub fn to_affine(&self) -> glam::Affine3A {
        glam::Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    #[inline]
    pub fn to_array(&self) -> [f32; 16] {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
            .to_cols_array()
    }

    #[inline]
    pub fn to_normal_matrix_array(&self) -> [f32; 9] {
        glam::Mat3::from_quat(self.rotation).to_cols_array()
    }
}

impl From<&Transform> for glam::Mat4 {
    #[inline]
    fn from(value: &Transform) -> Self {
        glam::Mat4::from_scale_rotation_translation(value.scale, value.rotation, value.translation)
    }
}

impl From<&Transform> for glam::Affine3A {
    #[inline]
    fn from(value: &Transform) -> Self {
        glam::Affine3A::from_scale_rotation_translation(
            value.scale,
            value.rotation,
            value.translation,
        )
    }
}

//--------------------------------------------------

// TODO - Review these operations
impl std::ops::Add for Transform {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Transform) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::AddAssign for Transform {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.add_assign(&rhs);
    }
}

impl std::ops::Sub for Transform {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Add<&Self> for Transform {
    type Output = Self;

    fn add(mut self, rhs: &Self) -> Self::Output {
        self.translation += rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation);
        self.scale *= rhs.scale;
        self
    }
}

impl std::ops::AddAssign<&Self> for Transform {
    fn add_assign(&mut self, rhs: &Self) {
        self.translation += rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation);
        self.scale *= rhs.scale;
    }
}

impl std::ops::Sub<&Self> for Transform {
    type Output = Self;

    fn sub(mut self, rhs: &Self) -> Self::Output {
        self.translation -= rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation.inverse());
        self.scale /= rhs.scale;

        self
    }
}

//====================================================================
