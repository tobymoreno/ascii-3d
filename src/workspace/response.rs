use crate::input::AppCommand;

/// Response returned by a scene workspace after it receives a workspace event.
///
/// This keeps the parent-child boundary explicit: the workspace may handle the
/// input locally, ignore it so the parent can continue routing, or request a
/// parent-owned action such as a debug line or app command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceResponse {
    Ignored,
    Handled,
    RequestAppCommand(AppCommand),
    RequestDebugLine(String),
    RequestReloadA3d,
}

impl WorkspaceResponse {
    pub const fn handled(handled: bool) -> Self {
        if handled {
            Self::Handled
        } else {
            Self::Ignored
        }
    }

    pub const fn is_handled(&self) -> bool {
        !matches!(self, Self::Ignored)
    }
}
