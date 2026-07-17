use super::{EditorCommand, EditorTransformCommand};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorEntry<T> {
    pub target: T,
    pub visible: Option<bool>,
    pub gizmo_visible: bool,
}

impl<T> EditorEntry<T> {
    pub fn new(target: T, visible: Option<bool>) -> Self {
        Self {
            target,
            visible,
            gizmo_visible: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditorSession<T> {
    entries: Vec<EditorEntry<T>>,
    inspected_target: Option<T>,
    active_target: T,
    selected_entry: usize,
    objects_panel_open: bool,
}

impl<T> EditorSession<T>
where
    T: Clone + PartialEq,
{
    pub fn new(entries: Vec<EditorEntry<T>>, active_target: T) -> Self {
        Self {
            entries,
            inspected_target: None,
            active_target,
            selected_entry: 0,
            objects_panel_open: false,
        }
    }

    pub fn entries(&self) -> &[EditorEntry<T>] {
        &self.entries
    }

    pub fn inspected_target(&self) -> Option<&T> {
        self.inspected_target.as_ref()
    }

    pub fn active_target(&self) -> &T {
        &self.active_target
    }

    pub const fn selected_entry(&self) -> usize {
        self.selected_entry
    }

    pub const fn objects_panel_open(&self) -> bool {
        self.objects_panel_open
    }

    pub fn replace_entries(&mut self, mut entries: Vec<EditorEntry<T>>, fallback_active: T) {
        let previous_inspected = self.inspected_target.clone();
        let previous_active = self.active_target.clone();
        let previous_gizmos = self
            .entries
            .iter()
            .map(|entry| (entry.target.clone(), entry.gizmo_visible))
            .collect::<Vec<_>>();

        for entry in &mut entries {
            if let Some((_, visible)) = previous_gizmos
                .iter()
                .find(|(target, _)| target == &entry.target)
            {
                entry.gizmo_visible = *visible;
            }
        }

        self.entries = entries;
        self.selected_entry = self
            .selected_entry
            .min(self.entries.len().saturating_sub(1));
        self.inspected_target = previous_inspected.filter(|target| self.contains_target(target));
        self.active_target = if self.contains_target(&previous_active) {
            previous_active
        } else {
            fallback_active
        };
    }

    pub fn is_active(&self, target: &T) -> bool {
        &self.active_target == target
    }

    pub fn visibility(&self, target: &T) -> Option<bool> {
        self.entries
            .iter()
            .find(|entry| &entry.target == target)
            .and_then(|entry| entry.visible)
    }

    pub fn gizmo_visible(&self, target: &T) -> bool {
        self.entries
            .iter()
            .find(|entry| &entry.target == target)
            .map(|entry| entry.gizmo_visible)
            .unwrap_or(false)
    }

    pub fn request_transform(
        &self,
        command: EditorTransformCommand<T>,
    ) -> Option<EditorTransformCommand<T>> {
        if !command.is_valid() || !self.contains_target(command.target()) {
            return None;
        }
        Some(command)
    }

    pub fn apply(&mut self, command: EditorCommand<T>) -> bool {
        match command {
            EditorCommand::OpenObjectsPanel => {
                self.objects_panel_open = true;
                true
            }
            EditorCommand::CloseObjectsPanel => {
                self.objects_panel_open = false;
                true
            }
            EditorCommand::SelectIndex(index) => {
                self.selected_entry = index.min(self.entries.len().saturating_sub(1));
                true
            }
            EditorCommand::MoveSelectionUp => {
                if self.entries.is_empty() {
                    self.selected_entry = 0;
                } else if self.selected_entry == 0 {
                    self.selected_entry = self.entries.len() - 1;
                } else {
                    self.selected_entry -= 1;
                }
                true
            }
            EditorCommand::MoveSelectionDown => {
                if self.entries.is_empty() {
                    self.selected_entry = 0;
                } else {
                    self.selected_entry = (self.selected_entry + 1) % self.entries.len();
                }
                true
            }
            EditorCommand::InspectSelected => {
                self.inspected_target = self
                    .entries
                    .get(self.selected_entry)
                    .map(|entry| entry.target.clone());
                self.inspected_target.is_some()
            }
            EditorCommand::Inspect(target) => {
                if !self.contains_target(&target) {
                    return false;
                }
                self.inspected_target = Some(target);
                true
            }
            EditorCommand::Activate(target) => {
                if !self.contains_target(&target) {
                    return false;
                }
                self.inspected_target = Some(target.clone());
                self.active_target = target;
                true
            }
            EditorCommand::ActivateInspected => {
                let Some(target) = self.inspected_target.clone() else {
                    return false;
                };
                self.active_target = target;
                true
            }
            EditorCommand::ToggleGizmo(target) => {
                let Some(entry) = self.entries.iter_mut().find(|entry| entry.target == target)
                else {
                    return false;
                };
                entry.gizmo_visible = !entry.gizmo_visible;
                true
            }
            EditorCommand::SetVisibility { target, visible } => {
                let Some(entry) = self.entries.iter_mut().find(|entry| entry.target == target)
                else {
                    return false;
                };
                let Some(current) = entry.visible.as_mut() else {
                    return false;
                };
                *current = visible;
                true
            }
            EditorCommand::ToggleVisibility(target) => {
                let Some(entry) = self.entries.iter_mut().find(|entry| entry.target == target)
                else {
                    return false;
                };
                let Some(visible) = entry.visible.as_mut() else {
                    return false;
                };
                *visible = !*visible;
                true
            }
        }
    }

    fn contains_target(&self, target: &T) -> bool {
        self.entries.iter().any(|entry| &entry.target == target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> EditorSession<&'static str> {
        EditorSession::new(
            vec![
                EditorEntry::new("camera", None),
                EditorEntry::new("earth", Some(true)),
            ],
            "camera",
        )
    }

    #[test]
    fn activate_command_updates_inspected_and_active_targets() {
        let mut session = session();

        assert!(session.apply(EditorCommand::Activate("earth")));
        assert_eq!(session.inspected_target(), Some(&"earth"));
        assert_eq!(session.active_target(), &"earth");
    }

    #[test]
    fn replacing_entries_preserves_valid_state_and_gizmos() {
        let mut session = session();
        session.apply(EditorCommand::Activate("earth"));
        session.apply(EditorCommand::ToggleGizmo("earth"));

        session.replace_entries(
            vec![
                EditorEntry::new("camera", None),
                EditorEntry::new("earth", Some(false)),
            ],
            "camera",
        );

        assert_eq!(session.inspected_target(), Some(&"earth"));
        assert_eq!(session.active_target(), &"earth");
        assert!(!session.gizmo_visible(&"earth"));
    }
    #[test]
    fn visibility_commands_update_only_entries_with_visibility() {
        let mut session = session();

        assert!(session.apply(EditorCommand::ToggleVisibility("earth")));
        assert_eq!(session.visibility(&"earth"), Some(false));

        assert!(session.apply(EditorCommand::SetVisibility {
            target: "earth",
            visible: true,
        }));
        assert_eq!(session.visibility(&"earth"), Some(true));

        assert!(!session.apply(EditorCommand::ToggleVisibility("camera")));
        assert_eq!(session.visibility(&"camera"), None);
    }

    #[test]
    fn transform_requests_validate_target_and_values() {
        let session = session();

        assert!(
            session
                .request_transform(EditorTransformCommand::Translate {
                    target: "earth",
                    delta: [1.0, 0.0, 0.0],
                })
                .is_some()
        );
        assert!(
            session
                .request_transform(EditorTransformCommand::Reset { target: "missing" })
                .is_none()
        );
        assert!(
            session
                .request_transform(EditorTransformCommand::ScaleUniform {
                    target: "earth",
                    factor: -1.0,
                })
                .is_none()
        );
    }
}
