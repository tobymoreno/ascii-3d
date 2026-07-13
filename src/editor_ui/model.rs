#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EditorTargetKey(pub String);

impl EditorTargetKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorTargetKind {
    Camera,
    SceneOrigin,
    Light,
    WorldAxes,
    Group,
    Mesh,
    QuadGroup,
    GeoJsonMap,
    SphereGuide,
    Other,
}

impl EditorTargetKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::SceneOrigin => "origin",
            Self::Light => "light",
            Self::WorldAxes => "axes",
            Self::Group => "group",
            Self::Mesh => "mesh",
            Self::QuadGroup => "quads",
            Self::GeoJsonMap => "map",
            Self::SphereGuide => "guide",
            Self::Other => "object",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EditorTarget {
    pub key: EditorTargetKey,
    pub id: String,
    pub path: String,
    pub kind: EditorTargetKind,
}

impl EditorTarget {
    pub fn new(
        key: impl Into<String>,
        id: impl Into<String>,
        path: impl Into<String>,
        kind: EditorTargetKind,
    ) -> Self {
        Self {
            key: EditorTargetKey::new(key),
            id: id.into(),
            path: path.into(),
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditorCapabilities(u16);

impl EditorCapabilities {
    pub const VISIBILITY: Self = Self(1 << 0);
    pub const TRANSLATE: Self = Self(1 << 1);
    pub const ROTATE: Self = Self(1 << 2);
    pub const SCALE: Self = Self(1 << 3);
    pub const DOLLY: Self = Self(1 << 4);
    pub const RESET: Self = Self(1 << 5);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, capability: Self) -> bool {
        self.0 & capability.0 == capability.0
    }

    pub const fn union(self, capability: Self) -> Self {
        Self(self.0 | capability.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorItem {
    pub target: EditorTarget,
    pub label: String,
    pub depth: usize,
    pub visible: Option<bool>,
    pub has_children: bool,
    pub capabilities: EditorCapabilities,
}

impl EditorItem {
    pub fn display_label(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let visibility = match self.visible {
            Some(true) => "[on] ",
            Some(false) => "[off]",
            None => "     ",
        };
        format!(
            "{indent}{visibility} {} ({})",
            self.label,
            self.target.kind.label()
        )
    }
}
