use crate::math::Vec3;

use super::SceneObject;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorldConfig {
    #[serde(default = "PhysicsWorldConfig::default_gravity")]
    pub gravity: [f32; 3],

    #[serde(default)]
    pub damping: f32,

    #[serde(default)]
    pub bounds_min: Option<[f32; 3]>,

    #[serde(default)]
    pub bounds_max: Option<[f32; 3]>,
}

impl Default for PhysicsWorldConfig {
    fn default() -> Self {
        Self {
            gravity: Self::default_gravity(),
            damping: 0.0,
            bounds_min: None,
            bounds_max: None,
        }
    }
}

impl PhysicsWorldConfig {
    pub const fn default_gravity() -> [f32; 3] {
        [0.0, -9.8, 0.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    Static,
    Dynamic,
    Kinematic,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PhysicsBodyConfig {
    #[serde(default = "PhysicsBodyConfig::default_body_type")]
    pub body_type: BodyType,

    #[serde(default = "PhysicsBodyConfig::default_mass")]
    pub mass: f32,

    #[serde(default)]
    pub velocity: [f32; 3],

    #[serde(default)]
    pub acceleration: [f32; 3],

    #[serde(default)]
    pub restitution: f32,

    #[serde(default)]
    pub affected_by_gravity: bool,
}

impl PhysicsBodyConfig {
    pub const fn default_body_type() -> BodyType {
        BodyType::Dynamic
    }

    pub const fn default_mass() -> f32 {
        1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsRuntimeBody {
    pub body_type: BodyType,
    pub mass: f32,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub restitution: f32,
    pub affected_by_gravity: bool,
}

impl From<PhysicsBodyConfig> for PhysicsRuntimeBody {
    fn from(config: PhysicsBodyConfig) -> Self {
        Self {
            body_type: config.body_type,
            mass: config.mass,
            velocity: Vec3::new(config.velocity[0], config.velocity[1], config.velocity[2]),
            acceleration: Vec3::new(
                config.acceleration[0],
                config.acceleration[1],
                config.acceleration[2],
            ),
            restitution: config.restitution,
            affected_by_gravity: config.affected_by_gravity,
        }
    }
}

impl PhysicsRuntimeBody {
    pub fn update_object(
        &mut self,
        object: &mut SceneObject,
        world: PhysicsWorldConfig,
        dt_seconds: f32,
    ) {
        if self.body_type != BodyType::Dynamic {
            return;
        }

        let gravity = if self.affected_by_gravity {
            Vec3::new(world.gravity[0], world.gravity[1], world.gravity[2])
        } else {
            Vec3::zero()
        };

        self.velocity = self.velocity + (self.acceleration + gravity) * dt_seconds;

        if world.damping > 0.0 {
            let damping = (1.0 - world.damping * dt_seconds).clamp(0.0, 1.0);
            self.velocity = self.velocity * damping;
        }

        let mut position = object.transform.position_vec3();
        position = position + self.velocity * dt_seconds;

        if let (Some(bounds_min), Some(bounds_max)) = (world.bounds_min, world.bounds_max) {
            let mut values = [position.x, position.y, position.z];
            let mut velocities = [self.velocity.x, self.velocity.y, self.velocity.z];

            for axis in 0..3 {
                if values[axis] < bounds_min[axis] {
                    values[axis] = bounds_min[axis];
                    velocities[axis] = -velocities[axis] * self.restitution;
                } else if values[axis] > bounds_max[axis] {
                    values[axis] = bounds_max[axis];
                    velocities[axis] = -velocities[axis] * self.restitution;
                }
            }

            position = Vec3::new(values[0], values[1], values[2]);
            self.velocity = Vec3::new(velocities[0], velocities[1], velocities[2]);
        }

        object.transform.set_position_vec3(position);
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyType, PhysicsBodyConfig, PhysicsRuntimeBody, PhysicsWorldConfig};
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
            parent_matrix: crate::math::Mat4::identity(),
            editor_composite: false,
            editor_hidden: false,
            source_root: std::path::PathBuf::new(),
        }
    }

    #[test]
    fn gravity_moves_dynamic_body_down() {
        let mut object = test_object();
        let mut body = PhysicsRuntimeBody::from(PhysicsBodyConfig {
            body_type: BodyType::Dynamic,
            mass: 1.0,
            velocity: [0.0, 0.0, 0.0],
            acceleration: [0.0, 0.0, 0.0],
            restitution: 0.0,
            affected_by_gravity: true,
        });

        body.update_object(&mut object, PhysicsWorldConfig::default(), 1.0);

        assert!(object.transform.position[1] < 0.0);
        assert!(body.velocity.y < 0.0);
    }
}
