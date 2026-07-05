use std::collections::HashMap;

use super::{A3dManifest, BehaviorRuntime, PhysicsRuntimeBody, PhysicsWorldConfig, SceneObject};

#[derive(Debug, Clone)]
pub struct LoadedWorld {
    pub title: String,
    pub physics: PhysicsWorldConfig,
    pub objects: Vec<SceneObject>,
    physics_bodies: HashMap<String, PhysicsRuntimeBody>,
}

impl LoadedWorld {
    pub fn from_manifest(manifest: A3dManifest) -> Result<Self, String> {
        manifest.validate()?;

        let physics = manifest.world.physics;
        let mut objects = Vec::with_capacity(manifest.objects.len());
        let mut physics_bodies = HashMap::new();

        for object_config in manifest.objects {
            let object = SceneObject::from(object_config);

            if let Some(body_config) = object.physics {
                physics_bodies.insert(object.id.clone(), PhysicsRuntimeBody::from(body_config));
            }

            objects.push(object);
        }

        Ok(Self {
            title: manifest.title,
            physics,
            objects,
            physics_bodies,
        })
    }

    pub fn update(&mut self, dt_seconds: f32) {
        for object in &mut self.objects {
            let behaviors = object
                .behaviors
                .iter()
                .cloned()
                .map(BehaviorRuntime::new)
                .collect::<Vec<_>>();

            for behavior in behaviors {
                behavior.update_object(object, dt_seconds);
            }

            if let Some(body) = self.physics_bodies.get_mut(&object.id) {
                body.update_object(object, self.physics, dt_seconds);
            }
        }
    }

    pub fn object(&self, id: &str) -> Option<&SceneObject> {
        self.objects.iter().find(|object| object.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::LoadedWorld;
    use crate::a3d::{
        A3dManifest, A3dObject, A3dViewport, A3dWorld, AssetRef, BehaviorConfig,
        PhysicsWorldConfig, RenderConfig, Transform,
    };

    #[test]
    fn loaded_world_runs_behavior_updates() {
        let manifest = A3dManifest {
            version: 1,
            title: "test".to_string(),
            world: A3dWorld {
                physics: PhysicsWorldConfig::default(),
            },
            camera: None,
            viewport: A3dViewport::default(),
            objects: vec![A3dObject {
                id: "box".to_string(),
                asset: AssetRef::Mesh {
                    path: "shapes/box.obj".to_string(),
                },
                transform: Transform::default(),
                render: RenderConfig::default(),
                behaviors: vec![BehaviorConfig::Translate {
                    velocity: [1.0, 0.0, 0.0],
                }],
                physics: None,
            }],
        };

        let mut world = LoadedWorld::from_manifest(manifest).expect("manifest should load");
        world.update(2.0);

        assert_eq!(
            world
                .object("box")
                .expect("object should exist")
                .transform
                .position,
            [2.0, 0.0, 0.0]
        );
    }
}
