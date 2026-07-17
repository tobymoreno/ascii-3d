#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand<T> {
    OpenObjectsPanel,
    CloseObjectsPanel,
    SelectIndex(usize),
    MoveSelectionUp,
    MoveSelectionDown,
    InspectSelected,
    Inspect(T),
    Activate(T),
    ActivateInspected,
    ToggleGizmo(T),
    SetVisibility { target: T, visible: bool },
    ToggleVisibility(T),
}
