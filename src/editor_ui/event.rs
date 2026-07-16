use super::{EditorTarget, MenuId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventSource {
    Keyboard,
    Menu,
    PropertiesPanel,
    ObjectHierarchy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformSpace {
    Local,
    World,
    View,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorAction {
    ActivateControlTarget,
    ToggleVisibility,
    ToggleTransformGizmo,
    SetVisibility(bool),
    Translate {
        axis: TransformAxis,
        amount: f32,
        space: TransformSpace,
    },
    Rotate {
        axis: TransformAxis,
        degrees: f32,
        space: TransformSpace,
    },
    ScaleUniform {
        factor: f32,
    },
    Dolly {
        amount: f32,
    },
    ResetTransform,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorEvent {
    CloseRequested,
    MenuOpened {
        menu_id: MenuId,
    },
    MenuActionRequested {
        menu_id: MenuId,
        action_id: String,
        source: EventSource,
    },
    SelectionChanged {
        target: EditorTarget,
        source: EventSource,
    },
    InspectRequested {
        target: EditorTarget,
        source: EventSource,
    },
    ActionRequested {
        target: EditorTarget,
        action: EditorAction,
        source: EventSource,
    },
}
