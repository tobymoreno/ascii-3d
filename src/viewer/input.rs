use crossterm::event::KeyCode;

use super::ViewerState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerInput {
    Handled,
    Quit,
}

fn apply_rotation_key(code: KeyCode, state: &mut ViewerState) -> bool {
    match code {
        KeyCode::Char('x') => state.rotation_x_degrees += 2.0,
        KeyCode::Char('X') => state.rotation_x_degrees -= 2.0,
        KeyCode::Char('y') => state.rotation_y_degrees += 2.0,
        KeyCode::Char('Y') => state.rotation_y_degrees -= 2.0,
        KeyCode::Char('z') => state.rotation_z_degrees += 2.0,
        KeyCode::Char('Z') => state.rotation_z_degrees -= 2.0,
        _ => return false,
    }
    true
}

pub fn handle_camera_key(code: KeyCode, state: &mut ViewerState) -> ViewerInput {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return ViewerInput::Quit,
        KeyCode::Left => state.origin_x += 0.5,
        KeyCode::Right => state.origin_x -= 0.5,
        KeyCode::Up => state.origin_y -= 0.5,
        KeyCode::Down => state.origin_y += 0.5,
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::PageUp => state.origin_z += 0.5,
        KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::PageDown => state.origin_z -= 0.5,
        KeyCode::Char('0') => {
            state.origin_x = 0.0;
            state.origin_y = 0.0;
            state.origin_z = 0.0;
            state.rotation_x_degrees = 0.0;
            state.rotation_y_degrees = 0.0;
            state.rotation_z_degrees = 0.0;
        }
        KeyCode::Char('r') => *state = ViewerState::default(),
        _ if apply_rotation_key(code, state) => {}
        _ => {}
    }
    ViewerInput::Handled
}

pub fn handle_scene_origin_key(code: KeyCode, state: &mut ViewerState) -> ViewerInput {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return ViewerInput::Quit,
        KeyCode::Left => state.origin_x -= 0.5,
        KeyCode::Right => state.origin_x += 0.5,
        KeyCode::Up => state.origin_y += 0.5,
        KeyCode::Down => state.origin_y -= 0.5,
        KeyCode::PageUp => state.origin_z += 0.5,
        KeyCode::PageDown => state.origin_z -= 0.5,
        KeyCode::Char('+') | KeyCode::Char('=') => state.zoom *= 1.1,
        KeyCode::Char('-') | KeyCode::Char('_') => state.zoom /= 1.1,
        KeyCode::Char('a') => state.show_axes = true,
        KeyCode::Char('A') => state.show_axes = false,
        KeyCode::Char('0') => {
            state.origin_x = 0.0;
            state.origin_y = 0.0;
            state.origin_z = 0.0;
        }
        KeyCode::Char('r') => *state = ViewerState::default(),
        _ if apply_rotation_key(code, state) => {}
        _ => {}
    }
    ViewerInput::Handled
}

pub fn handle_key(code: KeyCode, state: &mut ViewerState) -> ViewerInput {
    handle_scene_origin_key(code, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_plus_dollies_without_scaling() {
        let mut state = ViewerState::default();
        let zoom = state.zoom;
        handle_camera_key(KeyCode::Char('+'), &mut state);
        assert_eq!(state.zoom, zoom);
        assert_eq!(state.origin_z, 0.5);
    }

    #[test]
    fn scene_origin_plus_scales_without_dolly() {
        let mut state = ViewerState::default();
        handle_scene_origin_key(KeyCode::Char('+'), &mut state);
        assert!(state.zoom > 1.0);
        assert_eq!(state.origin_z, 0.0);
    }
}
