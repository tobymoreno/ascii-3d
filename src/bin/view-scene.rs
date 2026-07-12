use ascii_3d::{
    render::{apply_render_behaviors_to_scene, Frame, GeoJsonMapAsset, MeshAsset, RenderScene},
    scene::{
        load_scene_document, save_scene_document, scene_document_to_render_scene,
        set_scene_document_visibility, SceneDocument,
    },
    viewer::{
        collect_scene_objects_with_helpers, draw_render_scene, handle_key,
        handle_scene_object_transform_key, load_scene_maps, load_scene_meshes,
        scene_helper_property_lines, scene_object_property_lines, toggle_scene_object_visibility,
        ViewerInput, ViewerInspectorState, ViewerState, ViewerViewport, CAMERA_HELPER_PATH,
        MIN_VIEW_SCENE_HEIGHT, MIN_VIEW_SCENE_WIDTH, SCENE_ORIGIN_HELPER_PATH, VIEWER_MENU_TITLES,
        WORLD_AXES_HELPER_PATH,
    },
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
    Terminal,
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
) -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = AppTerminal::new(backend)?;
    let mut state = ViewerState::default();
    let mut inspector = ViewerInspectorState::default();
    inspector.active_object_path = Some(CAMERA_HELPER_PATH.to_string());
    inspector.active_xyz_target_path = CAMERA_HELPER_PATH.to_string();
    let mut object_entries = collect_scene_objects_with_helpers(&scene, state.show_axes);
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
                inspector.selected_menu,
                inspector.menu_focused || inspector.file_open || inspector.objects_open,
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

            if inspector.objects_open {
                draw_objects_popup(
                    ui,
                    centered_rect(
                        58,
                        (object_entries.len() as u16 + 4).clamp(6, 24),
                        ui.area(),
                    ),
                    &object_entries,
                    inspector.selected_object,
                );
            }

            if inspector.properties_open {
                let property_lines = inspector
                    .active_object_path
                    .as_deref()
                    .and_then(|path| {
                        scene_helper_property_lines(
                            path,
                            state.show_axes,
                            Some(inspector.active_xyz_target_path.as_str()),
                        )
                        .or_else(|| {
                            scene_object_property_lines(
                                &scene,
                                path,
                                Some(inspector.active_xyz_target_path.as_str()),
                            )
                        })
                    })
                    .unwrap_or_else(|| vec!["Object not found".to_string()]);

                draw_properties_popup(
                    ui,
                    centered_rect(
                        72,
                        (property_lines.len() as u16 + 4).clamp(8, 28),
                        ui.area(),
                    ),
                    active_object,
                    &property_lines,
                    inspector.selected_property_item,
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
                            match save_scene_document(&scene_path, &document) {
                                Ok(()) => {
                                    save_status = Some(format!("Saved {}", scene_path.display()));
                                }
                                Err(error) => {
                                    save_status = Some(format!("Save failed: {error}"));
                                }
                            }
                            inspector.close_popup();
                        } else {
                            inspector.open_save_as(scene_path.display().to_string());
                        }
                    }
                    _ => {}
                }

                continue;
            }

            if inspector.properties_open {
                let is_runtime_helper = inspector
                    .active_object_path
                    .as_deref()
                    .is_some_and(|path| path.starts_with("@scene/"));
                let property_action_count = if is_runtime_helper { 1 } else { 2 };

                match key.code {
                    KeyCode::Esc => inspector.close_properties(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        inspector.move_property_up(property_action_count)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        inspector.move_property_down(property_action_count)
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(path) = inspector.active_object_path.clone() {
                            if inspector.selected_property_item == 0 {
                                inspector.active_xyz_target_path = path.clone();
                                let label =
                                    inspector.active_label(&object_entries).unwrap_or("object");
                                save_status = Some(format!("XYZ target activated: {label}"));
                            } else if let Some(visible) =
                                toggle_scene_object_visibility(&mut scene, &path)
                            {
                                set_scene_document_visibility(&mut document, &path, visible);
                                object_entries =
                                    collect_scene_objects_with_helpers(&scene, state.show_axes);
                                save_status = Some("Unsaved visibility change".to_string());
                            }
                        }
                    }
                    _ => {}
                }

                continue;
            }

            if inspector.objects_open {
                match key.code {
                    KeyCode::Esc => inspector.close_popup(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        inspector.move_object_up(object_entries.len())
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        inspector.move_object_down(object_entries.len())
                    }
                    KeyCode::Enter => inspector.activate_selected(&object_entries),
                    _ => {}
                }

                continue;
            }

            if inspector.menu_focused {
                match key.code {
                    KeyCode::Esc | KeyCode::Tab => inspector.menu_focused = false,
                    KeyCode::Left => inspector.move_menu_left(),
                    KeyCode::Right => inspector.move_menu_right(),
                    KeyCode::Enter => inspector.open_selected_menu(object_entries.len()),
                    _ => {}
                }

                continue;
            }

            match key.code {
                KeyCode::Tab => inspector.focus_menu(),
                _ => {
                    let path = inspector.active_xyz_target_path.as_str();

                    if path == CAMERA_HELPER_PATH || path == SCENE_ORIGIN_HELPER_PATH {
                        let axes_before = state.show_axes;

                        if handle_key(key.code, &mut state) == ViewerInput::Quit {
                            return Ok(());
                        }

                        if state.show_axes != axes_before {
                            object_entries =
                                collect_scene_objects_with_helpers(&scene, state.show_axes);
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

fn draw_menu_bar(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    selected_menu: usize,
    focused: bool,
    status: &str,
) {
    let titles = VIEWER_MENU_TITLES
        .iter()
        .map(|title| Line::from(Span::raw(format!(" {title} "))))
        .collect::<Vec<_>>();

    let tabs = Tabs::new(titles)
        .divider(" ")
        .select(selected_menu)
        .highlight_style(if focused {
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });

    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(1)])
        .split(area);

    frame.render_widget(tabs, header[0]);
    frame.render_widget(Paragraph::new(status), header[1]);
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

fn draw_objects_popup(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    entries: &[ascii_3d::viewer::SceneObjectEntry],
    selected: usize,
) {
    let items = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let selector = if index == selected { ">" } else { " " };
            ListItem::new(Line::from(format!("{selector} {}", entry.display_label())))
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .title(" Objects  Enter=select  Esc=close ")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(list, area);
}

fn draw_properties_popup(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    object_name: &str,
    lines: &[String],
    selected_action: usize,
) {
    let mut action_index = 0usize;
    let items = lines
        .iter()
        .map(|line| {
            let is_action = line.starts_with("xyz control:") || line.starts_with("visible:");
            let selected = is_action && action_index == selected_action;
            let prefix = if selected { "> " } else { "  " };

            if is_action {
                action_index += 1;
            }

            let suffix = if line.starts_with("xyz control:") {
                "  [Enter/Space to activate]"
            } else if line.starts_with("visible:") {
                "  [Enter/Space to toggle]"
            } else {
                ""
            };

            let item = ListItem::new(Line::from(format!("{prefix}{line}{suffix}")));
            if selected {
                item.style(Style::default().add_modifier(Modifier::REVERSED))
            } else {
                item
            }
        })
        .collect::<Vec<_>>();

    let popup = List::new(items).block(
        Block::default()
            .title(format!(
                " Properties: {object_name}  Up/Down select  Enter activate  Esc=back "
            ))
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

fn main() -> io::Result<()> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/scenes/km_logo_quads.scene.json".to_string());

    let scene_path = PathBuf::from(path);
    let document = load_scene_document(&scene_path)?;
    let scene = scene_document_to_render_scene(document.clone());
    let meshes = load_scene_meshes(&scene_path, &scene)?;
    let maps = load_scene_maps(&scene_path, &scene)?;

    run_viewer(scene_path, document, scene, meshes, maps)
}
