use super::{SceneWorkspace, WorkspaceEvent, WorkspaceResponse};

/// Scene workspace for LoadedA3d-specific tools and controls.
///
/// This starts as a behavior-preserving skeleton. Input still routes through
/// app.rs today, but this module gives us the child workspace that can
/// gradually take over LoadedA3d camera, light, gizmo, and scene-local behavior.
#[derive(Debug, Default)]
pub struct LoadedA3dWorkspace;

impl LoadedA3dWorkspace {
    pub const fn new() -> Self {
        Self
    }
}

impl SceneWorkspace for LoadedA3dWorkspace {
    fn handle_workspace_event(&mut self, _event: WorkspaceEvent) -> WorkspaceResponse {
        WorkspaceResponse::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::AppCommand;

    #[test]
    fn loaded_a3d_workspace_initially_ignores_parent_commands() {
        let mut workspace = LoadedA3dWorkspace::new();

        assert_eq!(
            workspace.handle_workspace_event(WorkspaceEvent::Command(AppCommand::ReloadA3d)),
            WorkspaceResponse::Ignored
        );
    }
}
