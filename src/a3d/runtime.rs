use std::collections::HashMap;

use super::{BehaviorRuntime, PhysicsRuntimeBody, PhysicsWorldConfig, SceneObject};

#[derive(Debug, Clone)]
pub struct LoadedWorld {
    pub title: String,
    pub physics: PhysicsWorldConfig,
    pub objects: Vec<SceneObject>,
    physics_bodies: HashMap<String, PhysicsRuntimeBody>,
}

impl LoadedWorld {
    pub fn from_expanded(
        title: String,
        physics: PhysicsWorldConfig,
        objects: Vec<SceneObject>,
    ) -> Result<Self, String> {
        let mut physics_bodies = HashMap::new();

        for object in &objects {
            if let Some(body_config) = object.physics {
                physics_bodies.insert(object.id.clone(), PhysicsRuntimeBody::from(body_config));
            }
        }

        Ok(Self {
            title,
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

        self.rebuild_parent_matrices();
    }

    pub fn object(&self, id: &str) -> Option<&SceneObject> {
        self.objects.iter().find(|object| object.id == id)
    }

    pub fn object_mut(&mut self, id: &str) -> Option<&mut SceneObject> {
        self.objects.iter_mut().find(|object| object.id == id)
    }

    /// Rebuild every flattened object's parent matrix from the current
    /// transforms of its ancestor group objects.
    ///
    /// Group expansion stores parents before descendants and qualifies child
    /// IDs as `parent/child`, so one linear pass is sufficient.
    pub fn rebuild_parent_matrices(&mut self) {
        use crate::math::Mat4;

        let mut world_matrices = HashMap::<String, Mat4>::new();

        for object in &mut self.objects {
            let parent_matrix = object
                .id
                .rsplit_once('/')
                .and_then(|(parent_id, _)| world_matrices.get(parent_id).copied())
                .unwrap_or_else(Mat4::identity);

            object.parent_matrix = parent_matrix;
            world_matrices.insert(object.id.clone(), object.world_matrix());
        }
    }

    pub fn object_effectively_visible(&self, id: &str) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        if !object.render.visible {
            return false;
        }

        let mut ancestor = id.rsplit_once('/').map(|(parent, _)| parent);
        while let Some(parent_id) = ancestor {
            let Some(parent) = self.object(parent_id) else {
                break;
            };
            if !parent.render.visible {
                return false;
            }
            ancestor = parent_id.rsplit_once('/').map(|(next, _)| next);
        }

        true
    }

    pub fn translate_object(&mut self, id: &str, delta: [f32; 3]) -> bool {
        let Some(object) = self.object_mut(id) else {
            return false;
        };
        for (component, delta) in object.transform.position.iter_mut().zip(delta) {
            *component += delta;
        }
        self.rebuild_parent_matrices();
        true
    }

    pub fn rotate_object(&mut self, id: &str, delta_degrees: [f32; 3]) -> bool {
        let Some(object) = self.object_mut(id) else {
            return false;
        };
        for (component, delta) in object
            .transform
            .rotation_degrees
            .iter_mut()
            .zip(delta_degrees)
        {
            *component += delta;
        }
        self.rebuild_parent_matrices();
        true
    }

    pub fn reset_object_transform(&mut self, id: &str) -> bool {
        let Some(object) = self.object_mut(id) else {
            return false;
        };
        object.transform = super::Transform::default();
        self.rebuild_parent_matrices();
        true
    }

    pub fn toggle_object_visibility(&mut self, id: &str) -> Option<bool> {
        let object = self.object_mut(id)?;
        object.render.visible = !object.render.visible;
        Some(object.render.visible)
    }

    pub fn scale_object_uniform(&mut self, id: &str, factor: f32) -> bool {
        if !factor.is_finite() || factor <= 0.0 {
            return false;
        }

        let Some(object) = self.object_mut(id) else {
            return false;
        };

        object.transform.scale = object
            .transform
            .scale
            .map(|component| (component * factor).clamp(0.01, 1000.0));

        self.rebuild_parent_matrices();
        true
    }
}
