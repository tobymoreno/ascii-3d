use crate::{
    canvas::Canvas,
    geometry2d::Point2,
    input::{AppCommand, KeyBinding},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    Scenes,
    Camera,
    World,
    Glyphs,
    Physics,
    Debug,
    Help,
}

impl MenuKind {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Scenes => "Scenes",
            Self::Camera => "Camera",
            Self::World => "World",
            Self::Glyphs => "Glyphs",
            Self::Physics => "Physics",
            Self::Debug => "Debug",
            Self::Help => "Help",
        }
    }

    pub const fn hotkey(self) -> &'static str {
        match self {
            Self::Scenes => "m",
            Self::Camera => "c",
            Self::World => "w",
            Self::Glyphs => "g",
            Self::Physics => "f",
            Self::Debug => "d",
            Self::Help => "h/?",
        }
    }

    pub fn items(self) -> &'static [MenuItem] {
        match self {
            Self::Scenes => SCENE_ITEMS,
            Self::Camera => CAMERA_ITEMS,
            Self::World => WORLD_ITEMS,
            Self::Glyphs => GLYPH_ITEMS,
            Self::Physics => PHYSICS_ITEMS,
            Self::Debug => DEBUG_ITEMS,
            Self::Help => HELP_ITEMS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    pub label: &'static str,
    pub command: AppCommand,
    pub placeholder: bool,
}

impl MenuItem {
    pub const fn real(label: &'static str, command: AppCommand) -> Self {
        Self {
            label,
            command,
            placeholder: false,
        }
    }

    pub const fn placeholder(label: &'static str, command: AppCommand) -> Self {
        Self {
            label,
            command,
            placeholder: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuState {
    kind: MenuKind,
    selected_index: usize,
}

impl MenuState {
    pub const fn new(kind: MenuKind) -> Self {
        Self {
            kind,
            selected_index: 0,
        }
    }

    pub const fn kind(&self) -> MenuKind {
        self.kind
    }

    pub const fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_command(&self) -> AppCommand {
        self.kind.items()[self.selected_index].command
    }

    pub fn move_up(&mut self) {
        let item_count = self.kind.items().len();

        if item_count == 0 {
            return;
        }

        self.selected_index = if self.selected_index == 0 {
            item_count - 1
        } else {
            self.selected_index - 1
        };
    }

    pub fn move_down(&mut self) {
        let item_count = self.kind.items().len();

        if item_count == 0 {
            return;
        }

        self.selected_index = (self.selected_index + 1) % item_count;
    }

    pub fn bindings() -> &'static [KeyBinding] {
        MENU_BINDINGS
    }
}

const SCENE_ITEMS: &[MenuItem] = &[
    MenuItem::real("Next scene", AppCommand::NextScene),
    MenuItem::real("Previous scene", AppCommand::PreviousScene),
];

const CAMERA_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Toggle camera debug", AppCommand::ToggleCameraDebug),
    MenuItem::real("Reset camera", AppCommand::ResetCamera),
    MenuItem::placeholder("Toggle near-plane debug", AppCommand::ToggleNearPlaneDebug),
];

const WORLD_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Toggle world axes", AppCommand::ToggleWorldAxes),
    MenuItem::placeholder("Toggle world grid", AppCommand::ToggleWorldGrid),
];

const GLYPH_ITEMS: &[MenuItem] = &[
    MenuItem::real("Next glyph stroke", AppCommand::NextGlyphStroke),
    MenuItem::real("Previous glyph stroke", AppCommand::PreviousGlyphStroke),
    MenuItem::placeholder("Next glyph", AppCommand::NextGlyph),
    MenuItem::placeholder("Previous glyph", AppCommand::PreviousGlyph),
    MenuItem::placeholder("Select glyph...", AppCommand::SelectGlyph),
];

const PHYSICS_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Pause/play simulation", AppCommand::ToggleSimulationPause),
    MenuItem::placeholder("Step simulation", AppCommand::StepSimulation),
];

const DEBUG_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Toggle depth view", AppCommand::ToggleDepthView),
    MenuItem::placeholder("Toggle projection debug", AppCommand::ToggleProjectionDebug),
];

const HELP_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder("Scene mode: arrows change scene", AppCommand::CloseMenu),
    MenuItem::placeholder("Camera mode: WASD/QE move camera", AppCommand::CloseMenu),
    MenuItem::placeholder("Menus: j/k or arrows, Enter, Esc", AppCommand::CloseMenu),
];

const MENU_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "j / Down",
        label: "Menu down",
        command: AppCommand::MenuDown,
    },
    KeyBinding {
        key: "k / Up",
        label: "Menu up",
        command: AppCommand::MenuUp,
    },
    KeyBinding {
        key: "Enter",
        label: "Select menu item",
        command: AppCommand::MenuSelect,
    },
    KeyBinding {
        key: "Esc",
        label: "Close menu",
        command: AppCommand::CloseMenu,
    },
];

pub fn draw_menu(canvas: &mut Canvas, state: &MenuState, origin: Point2) {
    let items = state.kind.items();
    let width = 44_i32;
    let height = items.len() as i32 + 4;
    let left = origin.x;
    let top = origin.y;
    let right = left + width - 1;
    let bottom = top + height - 1;

    canvas.draw_line(Point2::new(left, top), Point2::new(right, top), '=');
    canvas.draw_line(Point2::new(left, bottom), Point2::new(right, bottom), '=');
    canvas.draw_line(Point2::new(left, top), Point2::new(left, bottom), '|');
    canvas.draw_line(Point2::new(right, top), Point2::new(right, bottom), '|');

    canvas.set(Point2::new(left, top), '+');
    canvas.set(Point2::new(right, top), '+');
    canvas.set(Point2::new(left, bottom), '+');
    canvas.set(Point2::new(right, bottom), '+');

    canvas.draw_text(
        Point2::new(left + 2, top + 1),
        &format!("{} menu [{}]", state.kind.title(), state.kind.hotkey()),
    );

    for (index, item) in items.iter().enumerate() {
        let selector = if index == state.selected_index() {
            ">"
        } else {
            " "
        };
        let placeholder = if item.placeholder {
            " (placeholder)"
        } else {
            ""
        };

        canvas.draw_text(
            Point2::new(left + 2, top + 2 + index as i32),
            &format!("{selector} {}{}", item.label, placeholder),
        );
    }
}
