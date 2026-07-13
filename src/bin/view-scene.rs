use ascii_3d::{
    a3d::{AssetRef, BehaviorConfig, LoadedWorld, SceneObject, load_a3d_project},
    editor_ui::{
        EditorAction, EditorEvent, MenuBarState, ObjectHierarchyState, PropertiesState,
        draw_menu_bar, draw_object_hierarchy, draw_properties_panel,
    },
    render::{Frame, GeoJsonMapAsset, MeshAsset, RenderScene, apply_render_behaviors_to_scene},
    scene::{
        AxisDocument, BehaviorDocument, DisplayDocument, GroupDocument, NodeDocument,
        ObjectDocument, ObjectKindDocument, SceneDocument, TransformDocument, load_scene_document,
        save_scene_document, scene_document_to_render_scene, set_scene_document_visibility,
    },
    viewer::{
        CAMERA_HELPER_PATH, FILE_MENU_ID, MIN_VIEW_SCENE_HEIGHT, MIN_VIEW_SCENE_WIDTH,
        OBJECTS_MENU_ID, SCENE_ORIGIN_HELPER_PATH, ViewerInput, ViewerInspectorState, ViewerState,
        ViewerViewport, collect_scene_objects_with_helpers, draw_render_scene, editor_items,
        handle_camera_key, handle_scene_object_transform_key, handle_scene_origin_key,
        load_scene_maps, load_scene_meshes, property_rows, reset_scene_object_transform,
        toggle_scene_object_visibility, viewer_menu_definitions,
    },
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use std::{
    collections::HashMap,
    env, io,
    io::stdout,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

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

fn run_viewer(
    mut scene_path: PathBuf,
    mut document: SceneDocument,
    mut scene: RenderScene,
    meshes: HashMap<String, MeshAsset>,
    maps: HashMap<String, GeoJsonMapAsset>,
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
    let menu_definitions = viewer_menu_definitions();
    let mut menu_bar = MenuBarState::default();
    let mut hierarchy = ObjectHierarchyState::default();
    let mut properties = PropertiesState::default();
    let mut save_status: Option<String> = None;
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
                draw_file_popup(
                    ui,
                    centered_rect(52, 6, ui.area()),
                    &scene_path,
                    inspector.selected_file_item,
                );
            }

            if inspector.save_as_open {
                draw_save_as_popup(
                    ui,
                    centered_rect(76, 7, ui.area()),
                    &inspector.save_as_path,
                );
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
                match key.code {
                    KeyCode::Esc => inspector.close_popup(),
                    KeyCode::Up | KeyCode::Char('k') => inspector.move_file_up(),
                    KeyCode::Down | KeyCode::Char('j') => inspector.move_file_down(),
                    KeyCode::Enter => {
                        if inspector.selected_file_item == 0 {
                            if save_enabled {
                                match save_scene_document(&scene_path, &document) {
                                    Ok(()) => {
                                        save_status =
                                            Some(format!("Saved {}", scene_path.display()));
                                    }
                                    Err(error) => {
                                        save_status = Some(format!("Save failed: {error}"));
                                    }
                                }
                            } else {
                                save_status = Some(
                                    "A3D is read-only in view-scene; edit/save it in ascii-3d"
                                        .to_string(),
                                );
                            }
                            inspector.close_popup();
                        } else if save_enabled {
                            inspector.open_save_as(scene_path.display().to_string());
                        } else {
                            save_status = Some(
                                "A3D is read-only in view-scene; edit/save it in ascii-3d"
                                    .to_string(),
                            );
                            inspector.close_popup();
                        }
                    }
                    _ => {}
                }

                continue;
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
                            return Ok(());
                        }

                        if state.show_axes != axes_before {
                            object_entries =
                                collect_scene_objects_with_helpers(&scene, state.show_axes);
                            hierarchy_items = editor_items(&object_entries);
                            hierarchy.replace_items(&hierarchy_items);
                        }
                    } else {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                            return Ok(());
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

fn draw_file_popup(frame: &mut ratatui::Frame<'_>, area: Rect, scene_path: &Path, selected: usize) {
    let labels = ["Save", "Save As..."];
    let items = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let selector = if index == selected { ">" } else { " " };
            let item = ListItem::new(Line::from(format!("{selector} {label}")));

            if index == selected {
                item.style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                item
            }
        })
        .chain(std::iter::once(ListItem::new(Line::from(format!(
            "  {}",
            scene_path.display()
        )))))
        .collect::<Vec<_>>();

    let popup = List::new(items).block(
        Block::default()
            .title(" File  Up/Down select  Enter open  Esc close ")
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

fn a3d_group_document(group: &SceneObject, world: &LoadedWorld) -> GroupDocument {
    let children = world
        .objects
        .iter()
        .filter(|object| direct_parent_id(&object.id) == Some(group.id.as_str()))
        .filter_map(|object| match &object.asset {
            AssetRef::Group { .. } => Some(NodeDocument::Group(a3d_group_document(object, world))),
            AssetRef::Mesh { path } => Some(NodeDocument::Object(ObjectDocument {
                id: object
                    .id
                    .rsplit_once('/')
                    .map(|(_, id)| id)
                    .unwrap_or(object.id.as_str())
                    .to_string(),
                name: object
                    .id
                    .rsplit_once('/')
                    .map(|(_, id)| id)
                    .unwrap_or(object.id.as_str())
                    .to_string(),
                transform: a3d_transform_document(object),
                visible: object.render.visible,
                behaviors: a3d_behavior_documents(object),
                object: ObjectKindDocument::Mesh {
                    asset: path.clone(),
                    backface_cull: object.render.backface_cull,
                },
            })),
            AssetRef::GeoJsonMap { path, radius_scale } => {
                Some(NodeDocument::Object(ObjectDocument {
                    id: object
                        .id
                        .rsplit_once('/')
                        .map(|(_, id)| id)
                        .unwrap_or(object.id.as_str())
                        .to_string(),
                    name: object
                        .id
                        .rsplit_once('/')
                        .map(|(_, id)| id)
                        .unwrap_or(object.id.as_str())
                        .to_string(),
                    transform: a3d_transform_document(object),
                    visible: object.render.visible,
                    behaviors: a3d_behavior_documents(object),
                    object: ObjectKindDocument::GeoJsonMap {
                        asset: path.clone(),
                        radius_scale: *radius_scale,
                    },
                }))
            }
            AssetRef::Word { .. } | AssetRef::Glyph { .. } => None,
        })
        .collect();

    GroupDocument {
        id: group
            .id
            .rsplit_once('/')
            .map(|(_, id)| id)
            .unwrap_or(group.id.as_str())
            .to_string(),
        name: group
            .id
            .rsplit_once('/')
            .map(|(_, id)| id)
            .unwrap_or(group.id.as_str())
            .to_string(),
        transform: a3d_transform_document(group),
        visible: group.render.visible,
        editor_composite: group.editor_composite,
        behaviors: a3d_behavior_documents(group),
        children,
    }
}

fn a3d_world_to_scene_document(world: &LoadedWorld) -> SceneDocument {
    let mut groups = world
        .objects
        .iter()
        .filter(|object| !object.id.contains('/'))
        .filter_map(|object| match object.asset {
            AssetRef::Group { .. } => Some(a3d_group_document(object, world)),
            _ => None,
        })
        .collect::<Vec<_>>();

    for object in world
        .objects
        .iter()
        .filter(|object| !object.id.contains('/'))
        .filter(|object| matches!(object.asset, AssetRef::Mesh { .. }))
    {
        let AssetRef::Mesh { path } = &object.asset else {
            continue;
        };

        groups.push(GroupDocument {
            id: object.id.clone(),
            name: object.id.clone(),
            transform: TransformDocument::default(),
            visible: object.render.visible,
            editor_composite: object.editor_composite,
            behaviors: Vec::new(),
            children: vec![NodeDocument::Object(ObjectDocument {
                id: "mesh".to_string(),
                name: object.id.clone(),
                transform: a3d_transform_document(object),
                visible: object.render.visible,
                behaviors: a3d_behavior_documents(object),
                object: ObjectKindDocument::Mesh {
                    asset: path.clone(),
                    backface_cull: object.render.backface_cull,
                },
            })],
        });
    }

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
    }
}

fn main() -> io::Result<()> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/scenes/km_logo_quads.scene.json".to_string());

    let scene_path = PathBuf::from(path);
    let is_a3d = scene_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("a3d"));

    let (document, save_enabled) = if is_a3d {
        let project = load_a3d_project(&scene_path)?;
        let world = project
            .into_world()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        (a3d_world_to_scene_document(&world), false)
    } else {
        (load_scene_document(&scene_path)?, true)
    };

    let scene = scene_document_to_render_scene(document.clone());
    let meshes = load_scene_meshes(&scene_path, &scene)?;
    let maps = load_scene_maps(&scene_path, &scene)?;

    run_viewer(scene_path, document, scene, meshes, maps, save_enabled)
}
