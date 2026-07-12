use crate::{input::AppCommand, xyz_control::XyzControlEvent};

/// Typed event passed from the app shell into the active scene workspace.
///
/// The parent still owns raw terminal input. Workspaces receive already-mapped
/// app commands or axis/origin control events and decide whether the active
/// scene can handle them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceEvent {
    Command(AppCommand),
    XyzControl(XyzControlEvent),
}

impl From<AppCommand> for WorkspaceEvent {
    fn from(command: AppCommand) -> Self {
        Self::Command(command)
    }
}

impl From<XyzControlEvent> for WorkspaceEvent {
    fn from(event: XyzControlEvent) -> Self {
        Self::XyzControl(event)
    }
}
