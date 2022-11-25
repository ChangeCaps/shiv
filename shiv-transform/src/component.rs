use std::ops::{Mul, MulAssign};

use glam::{Mat3, Mat4, Quat, Vec3};
use shiv::{bundle::Bundle, world::Component};

#[derive(Clone, Copy, Debug, Default, Bundle)]
pub struct TransformBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// The local transform of an entity.
///
/// This is the transform relative to the [`Parent`].
/// If there is no [`Parent`], this it is relative to the origin.
#[repr(C)]
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    #[inline]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    #[inline]
    pub const fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn with_xyz(mut self, x: f32, y: f32, z: f32) -> Self {
        self.translation = Vec3::new(x, y, z);
        self
    }

    #[inline]
    pub const fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    #[inline]
    pub const fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    #[inline]
    pub const fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Computes the matrix representation of this transform.
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Computes the inverse of this transform.
    #[inline]
    pub fn invserse(&self) -> Self {
        let inv_scale = self.scale.recip();
        let inv_rotation = self.rotation.conjugate();

        let mut inv_translation = -self.translation;
        inv_translation = inv_rotation.mul_vec3(inv_translation);
        inv_translation *= inv_scale;

        Self {
            translation: inv_translation,
            rotation: inv_rotation,
            scale: inv_scale,
        }
    }
}

impl Mul<Vec3> for Transform {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        let mut position = self.scale * rhs;
        position = self.rotation * position;
        position + self.translation
    }
}

impl Mul for Transform {
    type Output = Transform;

    #[inline]
    fn mul(self, rhs: Transform) -> Self::Output {
        let mut translation = self.scale * rhs.translation;
        translation = self.rotation * translation;
        translation += self.translation;

        Self {
            translation,
            rotation: self.rotation * rhs.rotation,
            scale: self.scale * rhs.scale,
        }
    }
}

impl MulAssign for Transform {
    #[inline]
    fn mul_assign(&mut self, rhs: Transform) {
        *self = *self * rhs;
    }
}

/// A global transform representing an affine transformation.
///
/// This is the transform relative to the origin.
#[repr(C)]
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GlobalTransform {
    pub translation: Vec3,
    pub rotation_scale: Mat3,
}

impl Default for GlobalTransform {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl GlobalTransform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation_scale: Mat3::IDENTITY,
    };

    #[inline]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn from_rotation_scale(rotation_scale: Mat3) -> Self {
        Self {
            rotation_scale,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(self.translation);
        let rotation_scale = Mat4::from_mat3(self.rotation_scale);
        translation * rotation_scale
    }
}

impl From<Transform> for GlobalTransform {
    #[inline]
    fn from(value: Transform) -> Self {
        Self {
            translation: value.translation,
            rotation_scale: Mat3::from_quat(value.rotation) * Mat3::from_diagonal(value.scale),
        }
    }
}

impl From<&Transform> for GlobalTransform {
    #[inline]
    fn from(value: &Transform) -> Self {
        Self {
            translation: value.translation,
            rotation_scale: Mat3::from_quat(value.rotation) * Mat3::from_diagonal(value.scale),
        }
    }
}

impl Mul<Vec3> for GlobalTransform {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        self.rotation_scale.mul_vec3(rhs) + self.translation
    }
}

impl Mul<Transform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, rhs: Transform) -> Self::Output {
        self * GlobalTransform::from(rhs)
    }
}

impl Mul for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, rhs: GlobalTransform) -> Self::Output {
        Self {
            translation: self * rhs.translation,
            rotation_scale: self.rotation_scale * rhs.rotation_scale,
        }
    }
}

impl MulAssign for GlobalTransform {
    #[inline]
    fn mul_assign(&mut self, rhs: GlobalTransform) {
        *self = *self * rhs;
    }
}
