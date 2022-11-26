use std::ops::{Mul, MulAssign};

use glam::{Affine3A, Mat3, Mat3A, Mat4, Quat, Vec3, Vec3A};
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

    #[inline]
    pub fn local_x(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    #[inline]
    pub fn local_y(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    #[inline]
    pub fn local_z(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    #[inline]
    pub fn left(&self) -> Vec3 {
        -self.local_x()
    }

    #[inline]
    pub fn right(&self) -> Vec3 {
        self.local_x()
    }

    #[inline]
    pub fn up(&self) -> Vec3 {
        self.local_y()
    }

    #[inline]
    pub fn down(&self) -> Vec3 {
        -self.local_y()
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        -self.local_z()
    }

    #[inline]
    pub fn back(&self) -> Vec3 {
        self.local_z()
    }

    #[inline]
    pub fn translate(&mut self, translation: Vec3) {
        self.translation += translation;
    }

    #[inline]
    pub fn scale(&mut self, scale: Vec3) {
        self.scale *= scale;
    }

    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    #[inline]
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_x(angle));
    }

    #[inline]
    pub fn rotate_y(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_y(angle));
    }

    #[inline]
    pub fn rotate_z(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_z(angle));
    }

    #[inline]
    pub fn look_at(&mut self, forward: Vec3, up: Vec3) {
        let right = up.cross(forward).normalize();
        let up = forward.cross(right).normalize();

        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
    }

    #[inline]
    pub fn looking_at(mut self, forward: Vec3, up: Vec3) -> Self {
        self.look_at(forward, up);
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
    pub matrix: Mat3,
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
        matrix: Mat3::IDENTITY,
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
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            matrix: Mat3::from_quat(rotation),
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            matrix: Mat3::from_diagonal(scale),
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn from_matrix(matrix: Mat3) -> Self {
        Self {
            matrix,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub const fn to_affine(self) -> Affine3A {
        Affine3A {
            translation: Vec3A::from_array(self.translation.to_array()),
            matrix3: Mat3A::from_cols_array(&self.matrix.to_cols_array()),
        }
    }

    #[inline]
    pub fn local_x(&self) -> Vec3 {
        Vec3::normalize(self.matrix * Vec3::X)
    }

    #[inline]
    pub fn local_y(&self) -> Vec3 {
        Vec3::normalize(self.matrix * Vec3::Y)
    }

    #[inline]
    pub fn local_z(&self) -> Vec3 {
        Vec3::normalize(self.matrix * Vec3::Z)
    }

    #[inline]
    pub fn left(&self) -> Vec3 {
        -self.local_x()
    }

    #[inline]
    pub fn right(&self) -> Vec3 {
        self.local_x()
    }

    #[inline]
    pub fn up(&self) -> Vec3 {
        self.local_y()
    }

    #[inline]
    pub fn down(&self) -> Vec3 {
        -self.local_y()
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        -self.local_z()
    }

    #[inline]
    pub fn back(&self) -> Vec3 {
        self.local_z()
    }

    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(self.translation);
        let rotation_scale = Mat4::from_mat3(self.matrix);
        translation * rotation_scale
    }
}

impl From<Transform> for GlobalTransform {
    #[inline]
    fn from(value: Transform) -> Self {
        Self {
            translation: value.translation,
            matrix: Mat3::from_quat(value.rotation) * Mat3::from_diagonal(value.scale),
        }
    }
}

impl From<&Transform> for GlobalTransform {
    #[inline]
    fn from(value: &Transform) -> Self {
        Self {
            translation: value.translation,
            matrix: Mat3::from_quat(value.rotation) * Mat3::from_diagonal(value.scale),
        }
    }
}

impl Mul<Vec3> for GlobalTransform {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        self.matrix.mul_vec3(rhs) + self.translation
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
            matrix: self.matrix * rhs.matrix,
        }
    }
}

impl MulAssign for GlobalTransform {
    #[inline]
    fn mul_assign(&mut self, rhs: GlobalTransform) {
        *self = *self * rhs;
    }
}
