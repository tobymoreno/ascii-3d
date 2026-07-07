mod event;
mod response;

pub use event::WorkspaceEvent;
pub use response::WorkspaceResponse;

/// Boundary for scene-specific workspace behavior.
///
/// The app shell owns the event loop and active-scene selection. A scene
/// workspace owns scene-local tools such as gizmos, camera helpers, and
/// scene-specific control handling.
pub trait SceneWorkspace {
    fn handle_workspace_event(&mut self, event: WorkspaceEvent) -> WorkspaceResponse;
}

#[cfg(test)]
mod tests {
    use super::WorkspaceResponse;

    #[test]
    fn handled_helper_maps_bool_to_response() {
        assert_eq!(WorkspaceResponse::handled(true), WorkspaceResponse::Handled);
        assert_eq!(
            WorkspaceResponse::handled(false),
            WorkspaceResponse::Ignored
        );
    }

    #[test]
    fn ignored_is_not_handled() {
        assert!(!WorkspaceResponse::Ignored.is_handled());
        assert!(WorkspaceResponse::Handled.is_handled());
        assert!(WorkspaceResponse::RequestReloadA3d.is_handled());
    }
}
