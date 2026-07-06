use crossterm::event::KeyCode;

use crate::menu::MenuKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    Quit,
    OpenA3dFilePicker,
    ReloadA3d,

    ToggleControlMode,
    SetControlModeScene,
    SetControlModeCamera,
    SetControlModeLight,

    NextScene,
    PreviousScene,
    ResetWorldCamera,
    ResetActiveControl,

    NextGlyphStroke,
    PreviousGlyphStroke,

    OpenMenu(MenuKind),
    CloseMenu,
    MenuUp,
    MenuDown,
    MenuSelect,

    MoveCameraForward,
    MoveCameraBackward,
    MoveCameraLeft,
    MoveCameraRight,
    MoveCameraDown,
    MoveCameraUp,
    RotateCameraLeft,
    RotateCameraRight,
    RotateCameraUp,
    RotateCameraDown,

    RotateWorldLeft,
    RotateWorldRight,
    RotateWorldUp,
    RotateWorldDown,
    ResetWorldObject,

    MoveLightForward,
    MoveLightBackward,
    MoveLightLeft,
    MoveLightRight,
    MoveLightDown,
    MoveLightUp,

    ToggleCameraDebug,
    ResetCamera,
    ToggleNearPlaneDebug,

    ToggleWorldAxes,
    ToggleWorldGrid,

    NextGlyph,
    PreviousGlyph,
    SelectGlyph,

    ToggleSimulationPause,
    StepSimulation,

    ToggleDepthView,
    ToggleProjectionDebug,
    ToggleFrameTiming,
    ToggleDebugConsole,
    ShowOsGraphicsOverlay,

    DebugRotateLoadedA3dObjectZPositive,
    DebugRotateLoadedA3dObjectZNegative,
}

pub fn menu_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc => Some(AppCommand::CloseMenu),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(AppCommand::MenuUp),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(AppCommand::MenuDown),
        KeyCode::Enter => Some(AppCommand::MenuSelect),
        _ => None,
    }
}

pub fn scene_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Char('z') => Some(AppCommand::DebugRotateLoadedA3dObjectZPositive),
        KeyCode::Char('Z') => Some(AppCommand::DebugRotateLoadedA3dObjectZNegative),
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control)),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),

        KeyCode::Char('x') | KeyCode::Char('X') => Some(AppCommand::OpenMenu(MenuKind::File)),
        KeyCode::Char('m') | KeyCode::Char('M') => Some(AppCommand::OpenMenu(MenuKind::Scenes)),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(AppCommand::OpenMenu(MenuKind::Glyphs)),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(AppCommand::OpenMenu(MenuKind::Physics)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::OpenMenu(MenuKind::Debug)),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
            Some(AppCommand::RotateWorldLeft)
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
            Some(AppCommand::RotateWorldRight)
        }
        KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => Some(AppCommand::RotateWorldUp),
        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
            Some(AppCommand::RotateWorldDown)
        }

        _ => None,
    }
}

pub fn camera_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Char('z') => Some(AppCommand::DebugRotateLoadedA3dObjectZPositive),
        KeyCode::Char('Z') => Some(AppCommand::DebugRotateLoadedA3dObjectZNegative),
        KeyCode::Esc => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('l') | KeyCode::Char('L') => Some(AppCommand::SetControlModeLight),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control)),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('w') | KeyCode::Char('W') => Some(AppCommand::MoveCameraForward),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(AppCommand::MoveCameraBackward),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(AppCommand::MoveCameraLeft),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::MoveCameraRight),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::MoveCameraDown),
        KeyCode::Char('e') | KeyCode::Char('E') => Some(AppCommand::MoveCameraUp),

        KeyCode::Left => Some(AppCommand::RotateCameraLeft),
        KeyCode::Right => Some(AppCommand::RotateCameraRight),
        KeyCode::Up => Some(AppCommand::RotateCameraUp),
        KeyCode::Down => Some(AppCommand::RotateCameraDown),

        _ => None,
    }
}

pub fn light_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Char('z') => Some(AppCommand::DebugRotateLoadedA3dObjectZPositive),
        KeyCode::Char('Z') => Some(AppCommand::DebugRotateLoadedA3dObjectZNegative),
        KeyCode::Esc => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control)),
        KeyCode::Char('l') | KeyCode::Char('L') => Some(AppCommand::SetControlModeLight),
        KeyCode::Char('W') => Some(AppCommand::SetControlModeScene),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        KeyCode::Char('w') => Some(AppCommand::MoveLightForward),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(AppCommand::MoveLightBackward),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(AppCommand::MoveLightLeft),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::MoveLightRight),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::MoveLightDown),
        KeyCode::Char('e') | KeyCode::Char('E') => Some(AppCommand::MoveLightUp),

        KeyCode::Left => Some(AppCommand::MoveLightLeft),
        KeyCode::Right => Some(AppCommand::MoveLightRight),
        KeyCode::Up => Some(AppCommand::MoveLightUp),
        KeyCode::Down => Some(AppCommand::MoveLightDown),

        _ => None,
    }
}
