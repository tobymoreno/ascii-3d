mod arbitrary_vector;
mod asset_axes;
mod asset_axes_rotation;
mod axes;
mod bezier_axes;
mod camera;
mod camera_motion;
mod camera_turntable;
mod cross_product;
mod obj_box;
mod quad4;
mod rotation;
mod single_p;

pub use arbitrary_vector::render as render_arbitrary_vector;
pub use asset_axes::render as render_asset_axes;
pub use asset_axes_rotation::render as render_asset_axes_rotation;
pub use axes::{draw_axes, render as render_axes};
pub use bezier_axes::render as render_bezier_axes;
pub use camera::render as render_camera;
pub use camera_motion::render as render_camera_motion;
pub use camera_turntable::render as render_camera_turntable;
pub use cross_product::{
    render_negative_z as render_cross_negative_z, render_positive_z as render_cross_positive_z,
};
pub use obj_box::render as render_obj_box;
pub use quad4::render as render_quad4;
pub use rotation::{RotationAxis, render as render_rotation};
pub use single_p::render as render_single_p;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    SingleP,
    BezierAxes,
    AssetAxesRotateX,
    AssetAxesRotateY,
    AssetAxesRotateZ,
    Quad4,
    CameraMotion,
    CameraTurntable,
    CameraLookAt,
    ObjBox,
    RotateAxesZ,
    RotateAxesY,
    RotateAxesX,
    CrossNegativeZ,
    CrossPositiveZ,
    ArbitraryVector,
    Axes,
}

impl Scene {
    /// Scenes are ordered newest-first.
    pub const ALL: [Self; 17] = [
        Self::SingleP,
        Self::BezierAxes,
        Self::AssetAxesRotateX,
        Self::AssetAxesRotateY,
        Self::AssetAxesRotateZ,
        Self::Quad4,
        Self::CameraMotion,
        Self::CameraTurntable,
        Self::CameraLookAt,
        Self::ObjBox,
        Self::RotateAxesZ,
        Self::RotateAxesY,
        Self::RotateAxesX,
        Self::CrossNegativeZ,
        Self::CrossPositiveZ,
        Self::ArbitraryVector,
        Self::Axes,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::SingleP => "single_p word parent with P glyph",
            Self::BezierAxes => "Bezier curve child of Cartesian axes",
            Self::AssetAxesRotateX => "asset Cartesian axes rotating around X",
            Self::AssetAxesRotateY => "asset Cartesian axes rotating around Y",
            Self::AssetAxesRotateZ => "asset Cartesian axes rotating around Z",
            Self::Quad4 => "loaded quad4.obj projected in XYZ space",
            Self::CameraMotion => "animated camera motion",
            Self::CameraTurntable => "camera Y-turntable inspection",
            Self::CameraLookAt => "look_at camera basis",
            Self::ObjBox => "rotating OBJ wireframe box",
            Self::RotateAxesZ => "rotate Cartesian axes around Z",
            Self::RotateAxesY => "rotate Cartesian axes around Y",
            Self::RotateAxesX => "rotate Cartesian axes around X",
            Self::CrossNegativeZ => "B x A points along -Z",
            Self::CrossPositiveZ => "A x B points along +Z",
            Self::ArbitraryVector => "arbitrary Vec3",
            Self::Axes => "3D Cartesian axes",
        }
    }

    pub const fn is_animated(self) -> bool {
        matches!(
            self,
            Self::AssetAxesRotateX
                | Self::AssetAxesRotateY
                | Self::AssetAxesRotateZ
                | Self::CameraMotion
                | Self::CameraTurntable
                | Self::ObjBox
                | Self::RotateAxesX
                | Self::RotateAxesY
                | Self::RotateAxesZ
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Scene;

    #[test]
    fn newest_scene_is_single_p() {
        assert_eq!(Scene::ALL.first(), Some(&Scene::SingleP));
    }

    #[test]
    fn bezier_and_asset_axes_scenes_follow_single_p() {
        assert_eq!(Scene::ALL[1], Scene::BezierAxes);
        assert_eq!(Scene::ALL[2], Scene::AssetAxesRotateX);
        assert_eq!(Scene::ALL[3], Scene::AssetAxesRotateY);
        assert_eq!(Scene::ALL[4], Scene::AssetAxesRotateZ);
    }

    #[test]
    fn quad4_is_sixth() {
        assert_eq!(Scene::ALL[5], Scene::Quad4);
    }

    #[test]
    fn oldest_scene_is_last() {
        assert_eq!(Scene::ALL.last(), Some(&Scene::Axes));
    }

    #[test]
    fn scene_count_is_seventeen() {
        assert_eq!(Scene::ALL.len(), 17);
    }

    #[test]
    fn animated_scenes_are_identified() {
        assert!(!Scene::SingleP.is_animated());
        assert!(!Scene::BezierAxes.is_animated());

        assert!(Scene::AssetAxesRotateX.is_animated());
        assert!(Scene::AssetAxesRotateY.is_animated());
        assert!(Scene::AssetAxesRotateZ.is_animated());
        assert!(Scene::CameraMotion.is_animated());
        assert!(Scene::CameraTurntable.is_animated());
        assert!(Scene::ObjBox.is_animated());
        assert!(Scene::RotateAxesX.is_animated());
        assert!(Scene::RotateAxesY.is_animated());
        assert!(Scene::RotateAxesZ.is_animated());

        assert!(!Scene::Quad4.is_animated());
        assert!(!Scene::CameraLookAt.is_animated());
        assert!(!Scene::Axes.is_animated());
    }
}
