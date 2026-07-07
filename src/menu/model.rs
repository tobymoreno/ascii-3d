use crate::{
    canvas::Canvas,
    geometry2d::Point2,
    input::{AppCommand, KeyBinding},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    File,
    Scenes,
    Control,
    Glyphs,
    Physics,
    Debug,
    Help,
}

impl MenuKind {
    pub const fn title(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Scenes => "Scenes",
            Self::Control => "Control",
            Self::Glyphs => "Glyphs",
            Self::Physics => "Physics",
            Self::Debug => "Debug",
            Self::Help => "Help",
        }
    }

    pub const fn hotkey(self) -> &'static str {
        match self {
            Self::File => "f",
            Self::Scenes => "m",
            Self::Control => "c",
            Self::Glyphs => "g",
            Self::Physics => "p",
            Self::Debug => "d",
            Self::Help => "h/?",
        }
    }

    pub fn items(self) -> &'static [MenuItem] {
        match self {
            Self::File => FILE_ITEMS,
            Self::Scenes => SCENE_ITEMS,
            Self::Control => CONTROL_ITEMS,
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

    pub fn with_selected(kind: MenuKind, selected_index: usize) -> Self {
        let item_count = kind.items().len();

        Self {
            kind,
            selected_index: if item_count == 0 {
                0
            } else {
                selected_index.min(item_count - 1)
            },
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

const FILE_ITEMS: &[MenuItem] = &[
    MenuItem::real("Load .a3d...", AppCommand::OpenA3dFilePicker),
    MenuItem::real("Reload current .a3d", AppCommand::ReloadA3d),
    MenuItem::real("Exit", AppCommand::Quit),
];

const SCENE_ITEMS: &[MenuItem] = &[
    MenuItem::real("Next scene", AppCommand::NextScene),
    MenuItem::real("Previous scene", AppCommand::PreviousScene),
];

const CONTROL_ITEMS: &[MenuItem] = &[
    MenuItem::real("World mode", AppCommand::SetControlModeScene),
    MenuItem::real("Camera mode", AppCommand::SetControlModeCamera),
    MenuItem::real("Light mode", AppCommand::SetControlModeLight),
    MenuItem::real("Rotate world +X  [x]", AppCommand::RotateWorldPositiveX),
    MenuItem::real("Rotate world -X  [X]", AppCommand::RotateWorldNegativeX),
    MenuItem::real("Rotate world +Y  [y]", AppCommand::RotateWorldPositiveY),
    MenuItem::real("Rotate world -Y  [Y]", AppCommand::RotateWorldNegativeY),
    MenuItem::real("Rotate world +Z  [z]", AppCommand::RotateWorldPositiveZ),
    MenuItem::real("Rotate world -Z  [Z]", AppCommand::RotateWorldNegativeZ),
    MenuItem::real(
        "Move origin -X  [Ctrl/Shift+Left]",
        AppCommand::MoveWorldOriginLeft,
    ),
    MenuItem::real(
        "Move origin +X  [Ctrl/Shift+Right]",
        AppCommand::MoveWorldOriginRight,
    ),
    MenuItem::real(
        "Move origin +Y  [Ctrl/Shift+Up]",
        AppCommand::MoveWorldOriginUp,
    ),
    MenuItem::real(
        "Move origin -Y  [Ctrl/Shift+Down]",
        AppCommand::MoveWorldOriginDown,
    ),
    MenuItem::real("Reset world axes/origin", AppCommand::ResetWorldAxes),
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
    MenuItem::real("Toggle debug console", AppCommand::ToggleDebugConsole),
    MenuItem::placeholder("Toggle depth view", AppCommand::ToggleDepthView),
    MenuItem::placeholder("Toggle projection debug", AppCommand::ToggleProjectionDebug),
    MenuItem::real("Toggle frame timing", AppCommand::ToggleFrameTiming),
    MenuItem::real(
        "Show OS graphics overlay",
        AppCommand::ShowOsGraphicsOverlay,
    ),
];

const HELP_ITEMS: &[MenuItem] = &[
    MenuItem::placeholder(
        "World axes: x/X y/Y z/Z rotate; Ctrl+arrows move origin",
        AppCommand::CloseMenu,
    ),
    MenuItem::placeholder("Camera mode: WASD/QE move camera", AppCommand::CloseMenu),
    MenuItem::placeholder("Light mode: WASD/QE move light", AppCommand::CloseMenu),
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
