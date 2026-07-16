use ascii_3d::{
    a3d::{AssetRef, BehaviorConfig, LoadedWorld, SceneObject, load_a3d_project},
    editor_ui::{
        DEBUG_MENU_ID, EditorAction, EditorEvent, EditorKeyRepeatGate, FILE_BROWSE_SCENES_ID,
        FILE_EXIT_ID, FILE_MENU_ID, FILE_OPEN_ID, FILE_RELOAD_ID, FILE_SAVE_AS_ID, FILE_SAVE_ID,
        MenuBarState, MenuDefinition, MenuEntry, ObjectHierarchyState, PropertiesState,
        WorkspaceKeymap, WorkspaceMenu, draw_menu_bar, draw_menu_popup, draw_object_hierarchy,
        draw_properties_panel, menu_action_count,
    },
    math::{Mat4, Vec3},
    mesh::Mesh,
    render::{Frame, GeoJsonMapAsset, RenderScene, apply_render_behaviors_to_scene},
    scene::{
        AxisDocument, BehaviorDocument, DisplayDocument, GroupDocument, MeshPrepareDocument,
        NodeDocument, ObjectDocument, ObjectKindDocument, SceneDocument, TransformDocument,
        load_scene_document, save_scene_document, scene_document_to_render_scene,
        set_scene_document_visibility,
    },
    viewer::{
        CAMERA_HELPER_PATH, MIN_VIEW_SCENE_HEIGHT, MIN_VIEW_SCENE_WIDTH, OBJECTS_MENU_ID,
        SCENE_ORIGIN_HELPER_PATH, ViewerInput, ViewerInspectorState, ViewerState, ViewerViewport,
        collect_scene_objects_with_helpers, draw_render_scene, editor_items, handle_camera_key,
        handle_scene_object_transform_key, handle_scene_origin_key, load_scene_maps,
        load_scene_meshes, property_rows, reset_scene_object_transform,
        toggle_scene_object_visibility, viewer_menu_definitions,
    },
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use std::{
    collections::HashMap,
    env, fs, io,
    io::stdout,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

fn unsupported_terminal_reason() -> Option<&'static str> {
    let term_program = env::var("TERM_PROGRAM").unwrap_or_default();
    if term_program.eq_ignore_ascii_case("mintty") {
        return Some("mintty does not reliably restore this full-screen TUI");
    }

    None
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

type AppTerminal = Terminal<CrosstermBackend<io::Stdout>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScenePickerMode {
    Open,
    BrowseBuiltIn,
}

#[derive(Clone, Debug)]
struct ScenePickerEntry {
    label: String,
    path: PathBuf,
    is_dir: bool,
}

#[derive(Clone, Debug)]
struct ScenePickerState {
    mode: ScenePickerMode,
    current_dir: PathBuf,
    entries: Vec<ScenePickerEntry>,
    selected: usize,
    error: Option<String>,
}

impl ScenePickerState {
    fn open(start_dir: PathBuf) -> Self {
        let mut state = Self {
            mode: ScenePickerMode::Open,
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            error: None,
        };
        state.refresh_directory();
        state
    }

    fn browse_built_in() -> Self {
        let mut entries = collect_built_in_scenes();
        entries.sort_by(|left, right| left.label.cmp(&right.label));
        Self {
            mode: ScenePickerMode::BrowseBuiltIn,
            current_dir: PathBuf::from("assets"),
            entries,
            selected: 0,
            error: None,
        }
    }

    fn title(&self) -> &'static str {
        match self.mode {
            ScenePickerMode::Open => "Open Scene",
            ScenePickerMode::BrowseBuiltIn => "Built-in Scenes",
        }
    }

    fn move_up(&mut self) {
        if self.entries.is_empty() {
            self.selected = 0;
        } else if self.selected == 0 {
            self.selected = self.entries.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.entries.is_empty() {
            self.selected = 0;
        } else {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }

    fn parent(&mut self) {
        if self.mode != ScenePickerMode::Open {
            return;
        }
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh_directory();
        }
    }

    fn activate(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.selected)?.clone();
        if entry.is_dir {
            self.current_dir = entry.path;
            self.refresh_directory();
            None
        } else {
            Some(entry.path)
        }
    }

    fn refresh_directory(&mut self) {
        self.entries.clear();
        self.selected = 0;
        self.error = None;

        let read_dir = match fs::read_dir(&self.current_dir) {
            Ok(read_dir) => read_dir,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            let is_dir = path.is_dir();
            if !is_dir && !is_supported_scene_path(&path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            self.entries.push(ScenePickerEntry {
                label: if is_dir {
                    format!("[{name}]")
                } else {
                    name.to_string()
                },
                path,
                is_dir,
            });
        }

        self.entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
        });
    }
}

fn run_viewer(
    mut scene_path: PathBuf,
    mut document: SceneDocument,
    mut scene: RenderScene,
    mut meshes: HashMap<String, Arc<ascii_3d::mesh::Mesh>>,
    mut maps: HashMap<String, GeoJsonMapAsset>,
    save_enabled: bool,
) -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = AppTerminal::new(backend)?;
    let mut state = ViewerState::default();
    let mut inspector = ViewerInspectorState::default();
    inspector.active_object_path = Some(CAMERA_HELPER_PATH.to_string());
    inspector.active_xyz_target_path = CAMERA_HELPER_PATH.to_string();
    let mut object_entries = collect_scene_objects_with_helpers(&scene, state.show_axes);
    let mut hierarchy_items = editor_items(&object_entries);
    let menu_definitions = viewer_menu_definitions(save_enabled);
    let mut menu_bar = MenuBarState::default();
    let keymap = WorkspaceKeymap::default();
    let mut editor_key_repeat = EditorKeyRepeatGate::default();
    let mut hierarchy = ObjectHierarchyState::default();
    let mut properties = PropertiesState::default();
    let mut save_status: Option<String> = None;
    let mut scene_picker: Option<ScenePickerState> = None;
    let mut frame = Frame::new(MIN_VIEW_SCENE_WIDTH, MIN_VIEW_SCENE_HEIGHT);
    let target_frame = Duration::from_millis(33);
    let mut previous_frame_start = Instant::now();

    loop {
        let frame_start = Instant::now();
        let delta = frame_start.duration_since(previous_frame_start);
        previous_frame_start = frame_start;

        apply_render_behaviors_to_scene(&mut scene, delta.as_secs_f32());

        state.frame_time_ms = delta.as_secs_f32() * 1000.0;
        state.fps = if delta.as_secs_f32() > 0.0 {
            1.0 / delta.as_secs_f32()
        } else {
            0.0
        };

        terminal.draw(|ui| {
            let shell = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(ui.area());

            let scene_area = shell[1];
            let scene_block = Block::default()
                .borders(Borders::ALL)
                .title(" viewport ");
            let viewport_area = scene_block.inner(scene_area);

            let render_width = (viewport_area.width as usize).max(MIN_VIEW_SCENE_WIDTH);
            let render_height = (viewport_area.height as usize).max(MIN_VIEW_SCENE_HEIGHT);

            if frame.width() != render_width || frame.height() != render_height {
                frame = Frame::new(render_width, render_height);
            }

            let viewport = ViewerViewport::terminal(render_width, render_height);
            draw_render_scene(&mut frame, viewport, &scene, &meshes, &maps, &state);

            let rendered = frame.render().replace('\r', "");
            let active_object = object_entries
                .iter()
                .find(|entry| entry.path == inspector.active_xyz_target_path)
                .map(|entry| entry.name.as_str())
                .unwrap_or("Camera");
            let footer = save_status.clone().unwrap_or_else(|| {
                format!(
                    "XYZ target: {active_object} | arrows move | PgUp/PgDn z | x/y/z rotate | runtime only"
                )
            });

            draw_menu_bar(
                ui,
                shell[0],
                &menu_definitions,
                &menu_bar,
                &format!("fps {:>5.1}", state.fps),
            );
            ui.render_widget(scene_block, scene_area);
            ui.render_widget(Paragraph::new(rendered), viewport_area);
            ui.render_widget(Paragraph::new(footer), shell[2]);

            if inspector.file_open {
                draw_menu_popup(
                    ui,
                    centered_rect(56, 12, ui.area()),
                    menu_definition(&menu_definitions, FILE_MENU_ID),
                    inspector.selected_file_item,
                    Some(&scene_path.display().to_string()),
                );
            }

            if inspector.debug_open {
                draw_menu_popup(
                    ui,
                    centered_rect(56, 7, ui.area()),
                    menu_definition(&menu_definitions, DEBUG_MENU_ID),
                    inspector.selected_debug_item,
                    None,
                );
            }

            if inspector.save_as_open {
                draw_save_as_popup(
                    ui,
                    centered_rect(76, 7, ui.area()),
                    &inspector.save_as_path,
                );
            }

            if inspector.confirm_exit {
                draw_exit_confirm_popup(ui, centered_rect(52, 7, ui.area()));
            }

            if let Some(picker) = scene_picker.as_ref() {
                draw_scene_picker(ui, centered_rect(78, 24, ui.area()), picker);
            }

            if hierarchy.is_open() {
                draw_object_hierarchy(
                    ui,
                    centered_rect(
                        58,
                        (hierarchy_items.len() as u16 + 4).clamp(6, 24),
                        ui.area(),
                    ),
                    &hierarchy_items,
                    &hierarchy,
                    "Objects",
                );
            }

            if properties.is_open() {
                let rows = properties
                    .target()
                    .map(|target| {
                        property_rows(
                            &scene,
                            target,
                            state.show_axes,
                            Some(inspector.active_xyz_target_path.as_str()),
                        )
                    })
                    .unwrap_or_else(|| vec![]);
                let object_name = properties
                    .target()
                    .and_then(|target| {
                        hierarchy_items
                            .iter()
                            .find(|item| item.target.key == target.key)
                            .map(|item| item.label.as_str())
                    })
                    .unwrap_or("Object");

                draw_properties_panel(
                    ui,
                    centered_rect(
                        72,
                        (rows.len() as u16 + 4).clamp(8, 28),
                        ui.area(),
                    ),
                    object_name,
                    &rows,
                    &properties,
                );
            }
        })?;

        while event::poll(Duration::from_millis(0))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            let editor_owns_input = scene_picker.is_some()
                || inspector.confirm_exit
                || inspector.save_as_open
                || inspector.file_open
                || inspector.debug_open
                || properties.is_open()
                || hierarchy.is_open()
                || menu_bar.focused();
            if editor_owns_input {
                if !editor_key_repeat.accept(key, Instant::now()) {
                    continue;
                }
            } else {
                editor_key_repeat.reset();
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    continue;
                }
            }

            if scene_picker.is_some() {
                let mut picker = scene_picker.take().expect("scene picker must be present");
                let mut keep_picker = true;

                match key.code {
                    KeyCode::Esc => keep_picker = false,
                    KeyCode::Up | KeyCode::Char('k') => picker.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => picker.move_down(),
                    KeyCode::Backspace | KeyCode::Left => picker.parent(),
                    KeyCode::Enter => {
                        if let Some(selected_path) = picker.activate() {
                            match load_viewer_scene(&selected_path) {
                                Ok((new_document, new_scene, new_meshes, new_maps, _)) => {
                                    scene_path = selected_path;
                                    document = new_document;
                                    scene = new_scene;
                                    meshes = new_meshes;
                                    maps = new_maps;
                                    state = ViewerState::default();
                                    inspector.active_object_path =
                                        Some(CAMERA_HELPER_PATH.to_string());
                                    inspector.active_xyz_target_path =
                                        CAMERA_HELPER_PATH.to_string();
                                    object_entries =
                                        collect_scene_objects_with_helpers(&scene, state.show_axes);
                                    hierarchy_items = editor_items(&object_entries);
                                    hierarchy.replace_items(&hierarchy_items);
                                    properties.close();
                                    save_status = Some(format!("Opened {}", scene_path.display()));
                                    keep_picker = false;
                                    menu_bar.blur();
                                }
                                Err(error) => {
                                    picker.error = Some(format!("Load failed: {error}"));
                                }
                            }
                        }
                    }
                    _ => {}
                }

                if keep_picker {
                    scene_picker = Some(picker);
                }
                continue;
            }

            if inspector.confirm_exit {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(()),
                    KeyCode::Esc
                    | KeyCode::Char('n')
                    | KeyCode::Char('N')
                    | KeyCode::Char('c')
                    | KeyCode::Char('C') => inspector.close_exit_confirm(),
                    _ => {}
                }
                continue;
            }

            if inspector.save_as_open {
                match key.code {
                    KeyCode::Esc => inspector.close_save_as(),
                    KeyCode::Enter => {
                        if !save_enabled {
                            save_status = Some(
                                "A3D is read-only in view-scene; edit/save it in ascii-3d"
                                    .to_string(),
                            );
                            inspector.close_save_as();
                            continue;
                        }

                        let requested_path = inspector.save_as_path.trim();

                        if requested_path.is_empty() {
                            save_status = Some("Save As path must not be empty".to_string());
                        } else {
                            let new_path = PathBuf::from(requested_path);

                            match save_scene_document(&new_path, &document) {
                                Ok(()) => {
                                    scene_path = new_path;
                                    save_status =
                                        Some(format!("Saved as {}", scene_path.display()));
                                    inspector.close_save_as();
                                }
                                Err(error) => {
                                    save_status = Some(format!("Save As failed: {error}"));
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        inspector.save_as_path.pop();
                    }
                    KeyCode::Char(character) => {
                        inspector.save_as_path.push(character);
                    }
                    _ => {}
                }

                continue;
            }

            if inspector.file_open {
                let actions = menu_actions(&menu_definitions, FILE_MENU_ID);
                match key.code {
                    KeyCode::Esc => inspector.close_popup(),
                    KeyCode::Up | KeyCode::Char('k') => inspector.move_file_up(actions.len()),
                    KeyCode::Down | KeyCode::Char('j') => inspector.move_file_down(actions.len()),
                    KeyCode::Enter => {
                        let Some((action_id, _, enabled)) =
                            actions.get(inspector.selected_file_item)
                        else {
                            continue;
                        };
                        if !*enabled {
                            save_status =
                                Some(format!("{} is not available in view-scene", action_id));
                            inspector.close_popup();
                            continue;
                        }
                        match action_id.as_str() {
                            FILE_OPEN_ID => {
                                let start_dir = scene_path
                                    .parent()
                                    .map(Path::to_path_buf)
                                    .unwrap_or_else(|| PathBuf::from("."));
                                inspector.close_popup();
                                scene_picker = Some(ScenePickerState::open(start_dir));
                            }
                            FILE_RELOAD_ID => {
                                match load_viewer_scene(&scene_path) {
                                    Ok((new_document, new_scene, new_meshes, new_maps, _)) => {
                                        document = new_document;
                                        scene = new_scene;
                                        meshes = new_meshes;
                                        maps = new_maps;
                                        object_entries = collect_scene_objects_with_helpers(
                                            &scene,
                                            state.show_axes,
                                        );
                                        hierarchy_items = editor_items(&object_entries);
                                        hierarchy.replace_items(&hierarchy_items);
                                        save_status =
                                            Some(format!("Reloaded {}", scene_path.display()));
                                    }
                                    Err(error) => {
                                        save_status = Some(format!("Reload failed: {error}"))
                                    }
                                }
                                inspector.close_popup();
                            }
                            FILE_SAVE_ID => {
                                match save_scene_document(&scene_path, &document) {
                                    Ok(()) => {
                                        save_status =
                                            Some(format!("Saved {}", scene_path.display()))
                                    }
                                    Err(error) => {
                                        save_status = Some(format!("Save failed: {error}"))
                                    }
                                }
                                inspector.close_popup();
                            }
                            FILE_SAVE_AS_ID => {
                                inspector.open_save_as(scene_path.display().to_string())
                            }
                            FILE_BROWSE_SCENES_ID => {
                                inspector.close_popup();
                                scene_picker = Some(ScenePickerState::browse_built_in());
                            }
                            FILE_EXIT_ID => inspector.open_exit_confirm(),
                            _ => inspector.close_popup(),
                        }
                    }
                    _ => {}
                }
                continue;
            }

            if inspector.debug_open {
                match key.code {
                    KeyCode::Esc => inspector.close_popup(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        inspector.selected_debug_item =
                            inspector.selected_debug_item.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let count =
                            menu_action_count(menu_definition(&menu_definitions, DEBUG_MENU_ID));
                        if count > 0 {
                            inspector.selected_debug_item =
                                (inspector.selected_debug_item + 1) % count;
                        }
                    }
                    KeyCode::Enter => {
                        save_status = Some("Debug actions are provided by ascii-3d".to_string());
                        inspector.close_popup();
                    }
                    _ => {}
                }
                continue;
            }

            if key.kind == KeyEventKind::Press {
                if let Some(menu) = keymap.menu_for_event(key) {
                    match menu {
                        WorkspaceMenu::File => {
                            menu_bar.focus_menu(FILE_MENU_ID, &menu_definitions);
                            inspector.file_open = true;
                            inspector.selected_file_item = 0;
                        }
                        WorkspaceMenu::Objects => {
                            menu_bar.focus_menu(OBJECTS_MENU_ID, &menu_definitions);
                            hierarchy.open(&hierarchy_items);
                        }
                        WorkspaceMenu::View => {
                            menu_bar.focus_menu("view", &menu_definitions);
                        }
                        WorkspaceMenu::Debug => {
                            menu_bar.focus_menu(DEBUG_MENU_ID, &menu_definitions);
                            inspector.debug_open = true;
                            inspector.selected_debug_item = 0;
                        }
                        WorkspaceMenu::Help => {
                            menu_bar.focus_menu("help", &menu_definitions);
                        }
                    }
                    continue;
                }
            }

            if properties.is_open() {
                let rows = properties
                    .target()
                    .map(|target| {
                        property_rows(
                            &scene,
                            target,
                            state.show_axes,
                            Some(inspector.active_xyz_target_path.as_str()),
                        )
                    })
                    .unwrap_or_default();

                if let Some(editor_event) = properties.handle_key(key.code, &rows) {
                    match editor_event {
                        EditorEvent::CloseRequested => {
                            hierarchy.open(&hierarchy_items);
                        }
                        EditorEvent::ActionRequested { target, action, .. } => match action {
                            EditorAction::ActivateControlTarget => {
                                inspector.active_object_path = Some(target.path.clone());
                                inspector.active_xyz_target_path = target.path.clone();
                                let label = hierarchy_items
                                    .iter()
                                    .find(|item| item.target.key == target.key)
                                    .map(|item| item.label.as_str())
                                    .unwrap_or("object");
                                save_status = Some(format!("XYZ target activated: {label}"));
                            }
                            EditorAction::ToggleVisibility => {
                                if let Some(visible) =
                                    toggle_scene_object_visibility(&mut scene, &target.path)
                                {
                                    set_scene_document_visibility(
                                        &mut document,
                                        &target.path,
                                        visible,
                                    );
                                    object_entries =
                                        collect_scene_objects_with_helpers(&scene, state.show_axes);
                                    hierarchy_items = editor_items(&object_entries);
                                    hierarchy.replace_items(&hierarchy_items);
                                    save_status = Some("Unsaved visibility change".to_string());
                                }
                            }
                            EditorAction::ResetTransform => {
                                let reset = if target.path == CAMERA_HELPER_PATH {
                                    state = ViewerState::default();
                                    true
                                } else if target.path == SCENE_ORIGIN_HELPER_PATH {
                                    state.origin_x = 0.0;
                                    state.origin_y = 0.0;
                                    state.origin_z = 0.0;
                                    state.rotation_x_degrees = 0.0;
                                    state.rotation_y_degrees = 0.0;
                                    state.rotation_z_degrees = 0.0;
                                    state.zoom = 1.0;
                                    true
                                } else {
                                    reset_scene_object_transform(&mut scene, &target.path)
                                };

                                if reset {
                                    save_status = Some(format!("Reset transform: {}", target.id));
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }

                continue;
            }

            if hierarchy.is_open() {
                if let Some(editor_event) = hierarchy.handle_key(key.code, &hierarchy_items) {
                    match editor_event {
                        EditorEvent::InspectRequested { target, .. } => {
                            inspector.active_object_path = Some(target.path.clone());
                            hierarchy.close();
                            properties.open(target);
                        }
                        EditorEvent::CloseRequested => menu_bar.focus(),
                        _ => {}
                    }
                }
                continue;
            }

            if menu_bar.focused() {
                if let Some(EditorEvent::MenuOpened { menu_id }) =
                    menu_bar.handle_key(key.code, &menu_definitions)
                {
                    if menu_id.0 == FILE_MENU_ID {
                        inspector.file_open = true;
                        inspector.selected_file_item = 0;
                    } else if menu_id.0 == OBJECTS_MENU_ID {
                        hierarchy.open(&hierarchy_items);
                    } else if menu_id.0 == DEBUG_MENU_ID {
                        inspector.debug_open = true;
                        inspector.selected_debug_item = 0;
                    }
                }
                continue;
            }

            match key.code {
                KeyCode::Tab => menu_bar.focus(),
                _ => {
                    let path = inspector.active_xyz_target_path.as_str();

                    if path == CAMERA_HELPER_PATH || path == SCENE_ORIGIN_HELPER_PATH {
                        let axes_before = state.show_axes;

                        let result = if path == CAMERA_HELPER_PATH {
                            handle_camera_key(key.code, &mut state)
                        } else {
                            handle_scene_origin_key(key.code, &mut state)
                        };
                        if result == ViewerInput::Quit {
                            inspector.open_exit_confirm();
                            continue;
                        }

                        if state.show_axes != axes_before {
                            object_entries =
                                collect_scene_objects_with_helpers(&scene, state.show_axes);
                            hierarchy_items = editor_items(&object_entries);
                            hierarchy.replace_items(&hierarchy_items);
                        }
                    } else {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                            inspector.open_exit_confirm();
                            continue;
                        }

                        if handle_scene_object_transform_key(&mut scene, path, key.code) {
                            let active_label = object_entries
                                .iter()
                                .find(|entry| entry.path == path)
                                .map(|entry| entry.name.as_str())
                                .unwrap_or("object");

                            save_status = Some(format!("Runtime XYZ change: {active_label}"));
                        }
                    }
                }
            }
        }

        let elapsed = frame_start.elapsed();

        if elapsed < target_frame {
            std::thread::sleep(target_frame - elapsed);
        }
    }
}

fn menu_definition<'a>(definitions: &'a [MenuDefinition], id: &str) -> &'a MenuDefinition {
    definitions
        .iter()
        .find(|definition| definition.id.0 == id)
        .expect("shared menu definition must exist")
}

fn menu_actions(definitions: &[MenuDefinition], id: &str) -> Vec<(String, String, bool)> {
    menu_definition(definitions, id)
        .entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Action {
                id, label, enabled, ..
            } => Some((id.clone(), label.clone(), *enabled)),
            MenuEntry::Separator => None,
        })
        .collect()
}

fn is_supported_scene_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.eq_ignore_ascii_case("scene.a3d")
        || name.to_ascii_lowercase().ends_with(".scene.json")
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("a3d"))
}

fn is_canonical_built_in_scene_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    name.eq_ignore_ascii_case("scene.a3d") || name.to_ascii_lowercase().ends_with(".scene.json")
}

fn collect_built_in_scenes() -> Vec<ScenePickerEntry> {
    fn visit(root: &Path, entries: &mut Vec<ScenePickerEntry>) {
        let Ok(children) = fs::read_dir(root) else {
            return;
        };
        for child in children.flatten() {
            let path = child.path();
            if path.is_dir() {
                visit(&path, entries);
            } else if is_canonical_built_in_scene_path(&path) {
                let label = path
                    .strip_prefix("assets")
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                entries.push(ScenePickerEntry {
                    label,
                    path,
                    is_dir: false,
                });
            }
        }
    }

    let mut entries = Vec::new();
    visit(Path::new("assets/a3d"), &mut entries);
    visit(Path::new("assets/scenes"), &mut entries);
    entries
}

fn draw_scene_picker(frame: &mut ratatui::Frame<'_>, area: Rect, picker: &ScenePickerState) {
    let mut items = picker
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let selector = if index == picker.selected { ">" } else { " " };
            ListItem::new(format!("{selector} {}", entry.label))
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        items.push(ListItem::new("  No scenes found"));
    }
    if let Some(error) = picker.error.as_deref() {
        items.push(ListItem::new(""));
        items.push(ListItem::new(format!("  {error}")));
    }

    let help = match picker.mode {
        ScenePickerMode::Open => "Enter open/load  Backspace/Left parent  Esc cancel",
        ScenePickerMode::BrowseBuiltIn => "Enter load  Esc cancel",
    };
    let popup = List::new(items).block(
        Block::default()
            .title(format!(
                " {}  {}  {} ",
                picker.title(),
                picker.current_dir.display(),
                help
            ))
            .borders(Borders::ALL),
    );
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn draw_exit_confirm_popup(frame: &mut ratatui::Frame<'_>, area: Rect) {
    let popup = Paragraph::new("Exit view-scene?\n\nEnter/Y = exit    Esc/N = cancel").block(
        Block::default()
            .title(" Confirm exit ")
            .borders(Borders::ALL),
    );
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn draw_save_as_popup(frame: &mut ratatui::Frame<'_>, area: Rect, path: &str) {
    let popup = Paragraph::new(format!(
        "Path:
> {path}

Type path  Backspace delete  Enter save  Esc cancel"
    ))
    .block(
        Block::default()
            .title(" Save Scene As ")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn a3d_transform_document(object: &SceneObject) -> TransformDocument {
    TransformDocument {
        position: object.transform.position,
        rotation_degrees: object.transform.rotation_degrees,
        scale: object.transform.scale,
    }
}

fn a3d_behavior_documents(object: &SceneObject) -> Vec<BehaviorDocument> {
    let mut documents = Vec::new();

    for behavior in &object.behaviors {
        let BehaviorConfig::Rotate {
            axis,
            degrees_per_second,
        } = behavior
        else {
            continue;
        };

        for (component, render_axis) in [
            (axis[0], AxisDocument::X),
            (axis[1], AxisDocument::Y),
            (axis[2], AxisDocument::Z),
        ] {
            if component.abs() <= f32::EPSILON {
                continue;
            }

            documents.push(BehaviorDocument::Spin {
                axis: render_axis,
                degrees_per_second: degrees_per_second * component,
                enabled: true,
            });
        }
    }

    documents
}

fn direct_parent_id(id: &str) -> Option<&str> {
    id.rsplit_once('/').map(|(parent, _)| parent)
}

fn a3d_mesh_prepare_document(object: &SceneObject) -> MeshPrepareDocument {
    let simplify = object
        .render
        .ascii_simplify
        .as_ref()
        .filter(|config| config.enabled);

    MeshPrepareDocument {
        normalize_to_size: Some(1.0),
        grid_size: simplify.and_then(|config| {
            (config.grid_size.is_finite() && config.grid_size > 0.0).then_some(config.grid_size)
        }),
        target_vertices: simplify
            .and_then(|config| config.target_vertices.filter(|value| *value > 0)),
        cache: simplify.is_some_and(|config| config.cache),
    }
}

fn a3d_generated_mesh_prepare_document() -> MeshPrepareDocument {
    MeshPrepareDocument {
        normalize_to_size: None,
        grid_size: None,
        target_vertices: None,
        cache: false,
    }
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
struct GlyphTransformDocument {
    #[serde(default)]
    translation: [f32; 3],
    #[serde(default)]
    rotation_degrees: [f32; 3],
    #[serde(default = "unit_scale")]
    scale: [f32; 3],
}

impl Default for GlyphTransformDocument {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: unit_scale(),
        }
    }
}

const fn unit_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

impl GlyphTransformDocument {
    fn matrix(self) -> Mat4 {
        Mat4::translation(
            self.translation[0],
            self.translation[1],
            self.translation[2],
        ) * Mat4::rotation_x(self.rotation_degrees[0].to_radians())
            * Mat4::rotation_y(self.rotation_degrees[1].to_radians())
            * Mat4::rotation_z(self.rotation_degrees[2].to_radians())
            * Mat4::scale(self.scale[0], self.scale[1], self.scale[2])
    }
}

#[derive(Debug, serde::Deserialize)]
struct WordAssetDocument {
    #[serde(default)]
    children: Vec<WordGlyphDocument>,
}

#[derive(Debug, serde::Deserialize)]
struct WordGlyphDocument {
    glyph_asset: String,
    #[serde(default)]
    local_transform: GlyphTransformDocument,
}

#[derive(Debug, serde::Deserialize)]
struct GlyphAssetDocument {
    #[serde(default)]
    paths: Vec<GlyphPathDocument>,
    #[serde(default)]
    sampling: GlyphSamplingDocument,
}

#[derive(Debug, Default, serde::Deserialize)]
struct GlyphPathDocument {
    #[serde(default)]
    segments: Vec<GlyphSegmentDocument>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GlyphSegmentDocument {
    Line {
        from: [f32; 3],
        to: [f32; 3],
    },
    CubicBezier {
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
        p3: [f32; 3],
    },
}

#[derive(Debug, serde::Deserialize)]
struct GlyphSamplingDocument {
    #[serde(default = "default_curve_segments")]
    default_segments_per_curve: usize,
}

impl Default for GlyphSamplingDocument {
    fn default() -> Self {
        Self {
            default_segments_per_curve: default_curve_segments(),
        }
    }
}

const fn default_curve_segments() -> usize {
    16
}

fn read_json_document<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    let source = fs::read_to_string(path)?;
    serde_json::from_str(&source).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {}: {error}", path.display()),
        )
    })
}

fn resolve_nested_asset_path(owner_path: &Path, asset_path: &str) -> PathBuf {
    let path = Path::new(asset_path);
    if path.is_absolute() || path.exists() {
        path.to_path_buf()
    } else {
        owner_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn array_vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn cubic_point(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let one_minus_t = 1.0 - t;
    p0 * (one_minus_t * one_minus_t * one_minus_t)
        + p1 * (3.0 * one_minus_t * one_minus_t * t)
        + p2 * (3.0 * one_minus_t * t * t)
        + p3 * (t * t * t)
}

fn append_mesh_line(vertices: &mut Vec<Vec3>, faces: &mut Vec<Vec<usize>>, from: Vec3, to: Vec3) {
    let start = vertices.len();
    vertices.push(from);
    vertices.push(to);
    faces.push(vec![start, start + 1]);
}

fn append_glyph_mesh(
    glyph: &GlyphAssetDocument,
    transform: Mat4,
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Vec<usize>>,
) {
    for path in &glyph.paths {
        for segment in &path.segments {
            match segment {
                GlyphSegmentDocument::Line { from, to } => append_mesh_line(
                    vertices,
                    faces,
                    transform.transform_point(array_vec3(*from)),
                    transform.transform_point(array_vec3(*to)),
                ),
                GlyphSegmentDocument::CubicBezier { p0, p1, p2, p3 } => {
                    let p0 = array_vec3(*p0);
                    let p1 = array_vec3(*p1);
                    let p2 = array_vec3(*p2);
                    let p3 = array_vec3(*p3);
                    let segments = glyph.sampling.default_segments_per_curve.max(1);
                    let mut previous = cubic_point(p0, p1, p2, p3, 0.0);
                    for index in 1..=segments {
                        let t = index as f32 / segments as f32;
                        let current = cubic_point(p0, p1, p2, p3, t);
                        append_mesh_line(
                            vertices,
                            faces,
                            transform.transform_point(previous),
                            transform.transform_point(current),
                        );
                        previous = current;
                    }
                }
            }
        }
    }
}

fn load_a3d_stroke_mesh(object: &SceneObject) -> io::Result<Mesh> {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    match &object.asset {
        AssetRef::Word { path } => {
            let word_path = Path::new(path);
            let word: WordAssetDocument = read_json_document(word_path)?;
            for child in word.children {
                let glyph_path = resolve_nested_asset_path(word_path, &child.glyph_asset);
                let glyph: GlyphAssetDocument = read_json_document(&glyph_path)?;
                append_glyph_mesh(
                    &glyph,
                    child.local_transform.matrix(),
                    &mut vertices,
                    &mut faces,
                );
            }
        }
        AssetRef::Glyph { path, .. } => {
            let glyph: GlyphAssetDocument = read_json_document(Path::new(path))?;
            append_glyph_mesh(&glyph, Mat4::identity(), &mut vertices, &mut faces);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a word or glyph object", object.id),
            ));
        }
    }

    Ok(Mesh::new(vertices, faces))
}

fn a3d_leaf_document(
    object: &SceneObject,
    generated_meshes: &mut HashMap<String, Arc<Mesh>>,
) -> io::Result<Option<ObjectDocument>> {
    let short_id = object
        .id
        .rsplit_once('/')
        .map(|(_, id)| id)
        .unwrap_or(object.id.as_str())
        .to_string();

    let object_kind = match &object.asset {
        AssetRef::Mesh { path } => ObjectKindDocument::Mesh {
            asset: path.clone(),
            backface_cull: object.render.backface_cull,
            prepare: a3d_mesh_prepare_document(object),
        },
        AssetRef::GeoJsonMap { path, radius_scale } => ObjectKindDocument::GeoJsonMap {
            asset: path.clone(),
            radius_scale: *radius_scale,
        },
        AssetRef::Word { .. } | AssetRef::Glyph { .. } => {
            let asset = format!("@a3d-strokes/{}", object.id);
            generated_meshes.insert(asset.clone(), Arc::new(load_a3d_stroke_mesh(object)?));
            ObjectKindDocument::Mesh {
                asset,
                backface_cull: false,
                prepare: a3d_generated_mesh_prepare_document(),
            }
        }
        AssetRef::Group { .. } => return Ok(None),
    };

    Ok(Some(ObjectDocument {
        id: short_id.clone(),
        name: short_id,
        transform: a3d_transform_document(object),
        visible: object.render.visible,
        behaviors: a3d_behavior_documents(object),
        object: object_kind,
    }))
}

fn a3d_group_document(
    group: &SceneObject,
    world: &LoadedWorld,
    generated_meshes: &mut HashMap<String, Arc<Mesh>>,
) -> io::Result<GroupDocument> {
    let mut children = Vec::new();
    for object in world
        .objects
        .iter()
        .filter(|object| direct_parent_id(&object.id) == Some(group.id.as_str()))
    {
        if matches!(&object.asset, AssetRef::Group { .. }) {
            children.push(NodeDocument::Group(a3d_group_document(
                object,
                world,
                generated_meshes,
            )?));
        } else if let Some(document) = a3d_leaf_document(object, generated_meshes)? {
            children.push(NodeDocument::Object(document));
        }
    }

    let short_id = group
        .id
        .rsplit_once('/')
        .map(|(_, id)| id)
        .unwrap_or(group.id.as_str())
        .to_string();

    Ok(GroupDocument {
        id: short_id.clone(),
        name: short_id,
        transform: a3d_transform_document(group),
        visible: group.render.visible,
        editor_composite: group.editor_composite,
        behaviors: a3d_behavior_documents(group),
        children,
    })
}

fn a3d_world_to_scene_document(
    world: &LoadedWorld,
) -> io::Result<(SceneDocument, HashMap<String, Arc<Mesh>>)> {
    let mut groups = Vec::new();
    let mut generated_meshes = HashMap::new();

    for object in world
        .objects
        .iter()
        .filter(|object| !object.id.contains('/'))
    {
        if matches!(&object.asset, AssetRef::Group { .. }) {
            groups.push(a3d_group_document(object, world, &mut generated_meshes)?);
            continue;
        }

        if let Some(document) = a3d_leaf_document(object, &mut generated_meshes)? {
            groups.push(GroupDocument {
                id: object.id.clone(),
                name: object.id.clone(),
                transform: TransformDocument::default(),
                visible: object.render.visible,
                editor_composite: object.editor_composite,
                behaviors: Vec::new(),
                children: vec![NodeDocument::Object(document)],
            });
        }
    }

    Ok((
        SceneDocument {
            name: world.title.clone(),
            mesh_asset: String::new(),
            display: DisplayDocument {
                world_scale: 4.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: Vec::new(),
            groups,
        },
        generated_meshes,
    ))
}

fn load_viewer_scene(
    scene_path: &Path,
) -> io::Result<(
    SceneDocument,
    RenderScene,
    HashMap<String, Arc<ascii_3d::mesh::Mesh>>,
    HashMap<String, GeoJsonMapAsset>,
    bool,
)> {
    let is_a3d = scene_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("a3d"));
    let (document, generated_meshes) = if is_a3d {
        let project = load_a3d_project(scene_path)?;
        let world = project
            .into_world()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        a3d_world_to_scene_document(&world)?
    } else {
        (load_scene_document(scene_path)?, HashMap::new())
    };
    let scene = scene_document_to_render_scene(document.clone());
    let mut meshes = load_scene_meshes(scene_path, &scene)?;
    meshes.extend(generated_meshes);
    let maps = load_scene_maps(scene_path, &scene)?;
    Ok((document, scene, meshes, maps, !is_a3d))
}

fn main() -> io::Result<()> {
    if let Some(reason) = unsupported_terminal_reason() {
        eprintln!("view-scene cannot start in this terminal: {reason}.");
        eprintln!("Use Windows Terminal, Command Prompt, or PowerShell instead.");
        return Ok(());
    }

    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/scenes/km_logo_quads.scene.json".to_string());

    let scene_path = PathBuf::from(path);
    let (document, scene, meshes, maps, save_enabled) = load_viewer_scene(&scene_path)?;

    run_viewer(scene_path, document, scene, meshes, maps, save_enabled)
}
