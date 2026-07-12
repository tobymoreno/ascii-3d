use crossterm::event::KeyCode;

use crate::menu::MenuKind;
use crate::xyz_control::XyzControlEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    Quit,
    OpenA3dFilePicker,
    ReloadA3d,
    OpenWorldObjects,

    ToggleControlMode,
    SetControlModeScene,
    SetControlModeCamera,
    SetControlModeLight,

    OpenSceneBrowser,
    ResetWorldCamera,
    ResetActiveControl,
    XyzControl(XyzControlEvent),

    NextGlyphStroke,
    PreviousGlyphStroke,

    OpenMenu(MenuKind),
    CloseMenu,
    MenuUp,
    MenuDown,
    MenuLeft,
    MenuRight,
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

    RotateWorldPositiveX,
    RotateWorldNegativeX,
    RotateWorldPositiveY,
    RotateWorldNegativeY,
    RotateWorldPositiveZ,
    RotateWorldNegativeZ,
    MoveWorldOriginLeft,
    MoveWorldOriginRight,
    MoveWorldOriginUp,
    MoveWorldOriginDown,
    ResetWorldAxes,

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
}

pub fn menu_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc => Some(AppCommand::CloseMenu),
        KeyCode::Left => Some(AppCommand::MenuLeft),
        KeyCode::Right => Some(AppCommand::MenuRight),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(AppCommand::MenuUp),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(AppCommand::MenuDown),
        KeyCode::Enter => Some(AppCommand::MenuSelect),
        _ => None,
    }
}

pub fn scene_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppCommand::Quit),
        KeyCode::Tab => Some(AppCommand::ToggleControlMode),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(AppCommand::OpenMenu(MenuKind::Control)),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(AppCommand::ResetActiveControl),

        KeyCode::Char('m') | KeyCode::Char('M') => Some(AppCommand::OpenMenu(MenuKind::Scenes)),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(AppCommand::OpenMenu(MenuKind::Glyphs)),
        KeyCode::Char('p') | KeyCode::Char('P') => Some(AppCommand::OpenMenu(MenuKind::Physics)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(AppCommand::OpenMenu(MenuKind::Debug)),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            Some(AppCommand::OpenMenu(MenuKind::Help))
        }

        _ => None,
    }
}

pub fn camera_mode_command_for_key(key: KeyCode) -> Option<AppCommand> {
    match key {
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
