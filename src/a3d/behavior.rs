use crate::math::Vec3;

use super::SceneObject;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorConfig {
    Static,
    Rotate {
        axis: [f32; 3],
        degrees_per_second: f32,
    },
    Translate {
        velocity: [f32; 3],
    },
    Bounce {
        velocity: [f32; 3],
        bounds_min: [f32; 3],
        bounds_max: [f32; 3],
    },
    Orbit {
        center: [f32; 3],
        axis: [f32; 3],
        degrees_per_second: f32,
        radius: f32,
    },
}

impl BehaviorConfig {
    pub fn update_object(&self, object: &mut SceneObject, dt_seconds: f32) {
        match self {
            Self::Static => {}

            Self::Rotate {
                axis,
                degrees_per_second,
            } => {
                object.transform.rotation_degrees[0] += axis[0] * degrees_per_second * dt_seconds;
                object.transform.rotation_degrees[1] += axis[1] * degrees_per_second * dt_seconds;
                object.transform.rotation_degrees[2] += axis[2] * degrees_per_second * dt_seconds;

                for angle in &mut object.transform.rotation_degrees {
                    *angle %= 360.0;
                }
            }

            Self::Translate { velocity } => {
                object.transform.position[0] += velocity[0] * dt_seconds;
                object.transform.position[1] += velocity[1] * dt_seconds;
                object.transform.position[2] += velocity[2] * dt_seconds;
            }

            Self::Bounce {
                velocity,
                bounds_min,
                bounds_max,
            } => {
                // This config-only behavior is intentionally deterministic and stateless.
                // It moves by configured velocity and mirrors position back into bounds.
                for axis in 0..3 {
                    object.transform.position[axis] += velocity[axis] * dt_seconds;

                    if object.transform.position[axis] < bounds_min[axis] {
                        object.transform.position[axis] =
                            bounds_min[axis] + (bounds_min[axis] - object.transform.position[axis]);
                    }

                    if object.transform.position[axis] > bounds_max[axis] {
                        object.transform.position[axis] =
                            bounds_max[axis] - (object.transform.position[axis] - bounds_max[axis]);
                    }
                }
            }

            Self::Orbit {
                center,
                axis,
                degrees_per_second,
                radius,
            } => {
                // First useful orbit implementation: Y-axis orbit.
                // Other axes are accepted in config but will be expanded later.
                let _axis = Vec3::new(axis[0], axis[1], axis[2]);
                let angle = object.transform.rotation_degrees[1] + degrees_per_second * dt_seconds;
                let radians = angle.to_radians();

                object.transform.position[0] = center[0] + radians.cos() * radius;
                object.transform.position[2] = center[2] + radians.sin() * radius;
                object.transform.rotation_degrees[1] = angle % 360.0;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BehaviorRuntime {
    pub config: BehaviorConfig,
}

impl BehaviorRuntime {
    pub const fn new(config: BehaviorConfig) -> Self {
        Self { config }
    }

    pub fn update_object(&self, object: &mut SceneObject, dt_seconds: f32) {
        self.config.update_object(object, dt_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::BehaviorConfig;
    use crate::a3d::{AssetRef, RenderConfig, SceneObject, Transform};

    fn test_object() -> SceneObject {
        SceneObject {
            id: "object".to_string(),
            asset: AssetRef::Mesh {
                path: "shapes/box.obj".to_string(),
            },
            transform: Transform::default(),
            render: RenderConfig::default(),
            behaviors: vec![],
            physics: None,
        }
    }

    #[test]
    fn rotate_behavior_updates_configured_axis() {
        let mut object = test_object();

        BehaviorConfig::Rotate {
            axis: [0.0, 1.0, 0.0],
            degrees_per_second: 90.0,
        }
        .update_object(&mut object, 0.5);

        assert_eq!(object.transform.rotation_degrees, [0.0, 45.0, 0.0]);
    }

    #[test]
    fn translate_behavior_moves_position() {
        let mut object = test_object();

        BehaviorConfig::Translate {
            velocity: [2.0, 0.0, -1.0],
        }
        .update_object(&mut object, 0.5);

        assert_eq!(object.transform.position, [1.0, 0.0, -0.5]);
    }
}
