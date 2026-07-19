use super::app::{NativeEditorApp, NativeEditorTarget};

const MOVE_STEP: f32 = 0.10;
const ROTATION_STEP_DEGREES: f32 = 5.0;
const SCALE_FACTOR: f32 = 1.05;
const MIN_SCALE: f32 = 0.01;
const MAX_SCALE: f32 = 100.0;
const MIN_CAMERA_DISTANCE: f32 = 0.25;
const MAX_CAMERA_DISTANCE: f32 = 250.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum TransformKeyAction {
    Move([f32; 3]),
    Rotate(usize, f32),
    Scale(f32),
}

pub(super) fn handle_transform_keys(app: &mut NativeEditorApp, context: &egui::Context) {
    let action = context.input(|input| {
        if input.modifiers.ctrl || input.modifiers.command || input.modifiers.alt {
            return None;
        }
        let rotation = if input.modifiers.shift {
            -ROTATION_STEP_DEGREES
        } else {
            ROTATION_STEP_DEGREES
        };
        if input.key_pressed(egui::Key::ArrowLeft) {
            Some(TransformKeyAction::Move([-MOVE_STEP, 0.0, 0.0]))
        } else if input.key_pressed(egui::Key::ArrowRight) {
            Some(TransformKeyAction::Move([MOVE_STEP, 0.0, 0.0]))
        } else if input.key_pressed(egui::Key::ArrowUp) {
            Some(TransformKeyAction::Move([0.0, MOVE_STEP, 0.0]))
        } else if input.key_pressed(egui::Key::ArrowDown) {
            Some(TransformKeyAction::Move([0.0, -MOVE_STEP, 0.0]))
        } else if input.key_pressed(egui::Key::PageUp) {
            Some(TransformKeyAction::Move([0.0, 0.0, MOVE_STEP]))
        } else if input.key_pressed(egui::Key::PageDown) {
            Some(TransformKeyAction::Move([0.0, 0.0, -MOVE_STEP]))
        } else if input.key_pressed(egui::Key::X) {
            Some(TransformKeyAction::Rotate(0, rotation))
        } else if input.key_pressed(egui::Key::Y) {
            Some(TransformKeyAction::Rotate(1, rotation))
        } else if input.key_pressed(egui::Key::Z) {
            Some(TransformKeyAction::Rotate(2, rotation))
        } else if input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Text(text) if text == "+" || text == "="))
        {
            Some(TransformKeyAction::Scale(SCALE_FACTOR))
        } else if input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Text(text) if text == "-" || text == "_"))
        {
            Some(TransformKeyAction::Scale(1.0 / SCALE_FACTOR))
        } else {
            None
        }
    });

    match action {
        Some(TransformKeyAction::Move(delta)) => translate(app, delta),
        Some(TransformKeyAction::Rotate(axis, degrees)) => rotate(app, axis, degrees),
        Some(TransformKeyAction::Scale(factor)) => scale_or_dolly(app, factor),
        None => false,
    };
}

fn translate(app: &mut NativeEditorApp, delta: [f32; 3]) -> bool {
    match app.session.inspected_target().cloned() {
        Some(NativeEditorTarget::Scene) => {
            for (component, amount) in app.scene_transform.position.iter_mut().zip(delta) {
                *component += amount;
            }
            app.status = format!(
                "Scene origin [{:.2}, {:.2}, {:.2}]",
                app.scene_transform.position[0],
                app.scene_transform.position[1],
                app.scene_transform.position[2]
            );
            true
        }
        Some(NativeEditorTarget::Camera) => {
            let (right, up) = app.camera.view_axes();
            let forward = app.camera.forward();
            let world_delta = right * delta[0] + up * delta[1] + forward * delta[2];
            app.camera.position = app.camera.position + world_delta;
            app.status = format!(
                "Camera target [{:.2}, {:.2}, {:.2}]",
                app.camera.target().x,
                app.camera.target().y,
                app.camera.target().z
            );
            true
        }
        Some(NativeEditorTarget::Object(id)) => {
            let Some(world) = app.world.as_mut() else {
                return false;
            };
            let Some(object) = world.object_mut(&id) else {
                return false;
            };
            for (component, amount) in object.transform.position.iter_mut().zip(delta) {
                *component += amount;
            }
            let position = object.transform.position;
            world.rebuild_parent_matrices();
            app.status = format!(
                "Moved {id} to [{:.2}, {:.2}, {:.2}]",
                position[0], position[1], position[2]
            );
            true
        }
        None => false,
    }
}

fn rotate(app: &mut NativeEditorApp, axis: usize, degrees: f32) -> bool {
    match app.session.inspected_target().cloned() {
        Some(NativeEditorTarget::Scene) => {
            app.scene_transform.rotation_degrees[axis] =
                (app.scene_transform.rotation_degrees[axis] + degrees).rem_euclid(360.0);
            app.status = format!(
                "Scene {} rotation {:.1}°",
                ['X', 'Y', 'Z'][axis],
                app.scene_transform.rotation_degrees[axis]
            );
            true
        }
        Some(NativeEditorTarget::Camera) => {
            let radians = degrees.to_radians();
            match axis {
                0 => {
                    app.camera.pitch_radians =
                        (app.camera.pitch_radians + radians).clamp(-1.553_343, 1.553_343);
                }
                1 => app.camera.yaw_radians += radians,
                2 => app.camera.roll_radians += radians,
                _ => return false,
            }

            // Match the terminal viewer: rotate the view around the camera's
            // fixed position. The focus target changes with yaw/pitch; the
            // camera itself does not orbit around the scene or world origin.
            app.status = format!("Camera {} rotation {:.1}°", ['X', 'Y', 'Z'][axis], degrees);
            true
        }
        Some(NativeEditorTarget::Object(id)) => {
            let Some(world) = app.world.as_mut() else {
                return false;
            };
            let Some(object) = world.object_mut(&id) else {
                return false;
            };
            object.transform.rotation_degrees[axis] =
                (object.transform.rotation_degrees[axis] + degrees).rem_euclid(360.0);
            let value = object.transform.rotation_degrees[axis];
            world.rebuild_parent_matrices();
            app.status = format!("Rotated {id} {} to {:.1}°", ['X', 'Y', 'Z'][axis], value);
            true
        }
        None => false,
    }
}

fn scale_or_dolly(app: &mut NativeEditorApp, factor: f32) -> bool {
    match app.session.inspected_target().cloned() {
        Some(NativeEditorTarget::Scene) => {
            app.scene_transform.scale = app
                .scene_transform
                .scale
                .map(|value| (value * factor).clamp(MIN_SCALE, MAX_SCALE));
            app.status = format!("Scene scale {:.3}", app.scene_transform.scale[0]);
            true
        }
        Some(NativeEditorTarget::Camera) => {
            let target = app.camera.target();
            app.camera.focus_distance = (app.camera.focus_distance / factor)
                .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
            app.camera.position = target - app.camera.forward() * app.camera.focus_distance;
            app.status = format!("Camera distance {:.2}", app.camera.focus_distance);
            true
        }
        Some(NativeEditorTarget::Object(id)) => {
            let Some(world) = app.world.as_mut() else {
                return false;
            };
            let Some(object) = world.object_mut(&id) else {
                return false;
            };
            object.transform.scale = object
                .transform
                .scale
                .map(|value| (value * factor).clamp(MIN_SCALE, MAX_SCALE));
            let value = object.transform.scale[0];
            world.rebuild_parent_matrices();
            app.status = format!("Scaled {id} to {:.3}", value);
            true
        }
        None => false,
    }
}
