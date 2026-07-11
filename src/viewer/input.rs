use crossterm::event::KeyCode;

use super::ViewerState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerInput {
    Handled,
    Quit,
}

pub fn handle_key(code: KeyCode, state: &mut ViewerState) -> ViewerInput {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => ViewerInput::Quit,

        KeyCode::Left => {
            state.origin_x -= 0.5;
            ViewerInput::Handled
        }
        KeyCode::Right => {
            state.origin_x += 0.5;
            ViewerInput::Handled
        }
        KeyCode::Up => {
            state.origin_y += 0.5;
            ViewerInput::Handled
        }
        KeyCode::Down => {
            state.origin_y -= 0.5;
            ViewerInput::Handled
        }
        KeyCode::PageUp => {
            state.origin_z += 0.5;
            ViewerInput::Handled
        }
        KeyCode::PageDown => {
            state.origin_z -= 0.5;
            ViewerInput::Handled
        }

        KeyCode::Char('x') => {
            state.rotation_x_degrees += 2.0;
            ViewerInput::Handled
        }
        KeyCode::Char('X') => {
            state.rotation_x_degrees -= 2.0;
            ViewerInput::Handled
        }
        KeyCode::Char('y') => {
            state.rotation_y_degrees += 2.0;
            ViewerInput::Handled
        }
        KeyCode::Char('Y') => {
            state.rotation_y_degrees -= 2.0;
            ViewerInput::Handled
        }
        KeyCode::Char('z') => {
            state.rotation_z_degrees += 2.0;
            ViewerInput::Handled
        }
        KeyCode::Char('Z') => {
            state.rotation_z_degrees -= 2.0;
            ViewerInput::Handled
        }

        KeyCode::Char('+') | KeyCode::Char('=') => {
            state.zoom *= 1.1;
            ViewerInput::Handled
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            state.zoom /= 1.1;
            ViewerInput::Handled
        }

        KeyCode::Char('a') => {
            state.show_axes = true;
            ViewerInput::Handled
        }
        KeyCode::Char('A') => {
            state.show_axes = false;
            ViewerInput::Handled
        }

        KeyCode::Char('0') => {
            state.origin_x = 0.0;
            state.origin_y = 0.0;
            state.origin_z = 0.0;
            ViewerInput::Handled
        }
        KeyCode::Char('r') => {
            *state = ViewerState::default();
            ViewerInput::Handled
        }

        _ => ViewerInput::Handled,
    }
}
