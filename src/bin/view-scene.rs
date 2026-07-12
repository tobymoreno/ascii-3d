use ascii_3d::{
    render::{apply_render_behaviors_to_scene, Frame, GeoJsonMapAsset, MeshAsset, RenderScene},
    viewer::{
        collect_scene_objects, draw_render_scene, handle_key, load_scene_maps, load_scene_meshes,
        read_scene, scene_object_property_lines, ViewerInput, ViewerInspectorState, ViewerState,
        ViewerViewport, MIN_VIEW_SCENE_HEIGHT, MIN_VIEW_SCENE_WIDTH, VIEWER_MENU_TITLES,
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
    path::Path,
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
    mut scene: RenderScene,
    meshes: HashMap<String, MeshAsset>,
    maps: HashMap<String, GeoJsonMapAsset>,
) -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = AppTerminal::new(backend)?;
    let mut state = ViewerState::default();
    let mut inspector = ViewerInspectorState::default();
    let object_entries = collect_scene_objects(&scene);
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
            let active_object = inspector.active_label(&object_entries).unwrap_or("none");
            let footer = format!(
                "Tab menu | arrows origin | PgUp/PgDn z | +/- zoom | x/y/z rotate | active: {active_object}"
            );

            draw_menu_bar(
                ui,
                shell[0],
                inspector.selected_menu,
                inspector.menu_focused || inspector.objects_open,
                &format!(
                    "{}  {}x{}  render {}x{}  fps {:>5.1}",
                    scene.name,
                    viewport_area.width,
                    viewport_area.height,
                    render_width,
                    render_height,
                    state.fps,
                ),
            );
            ui.render_widget(scene_block, scene_area);
            ui.render_widget(Paragraph::new(rendered), viewport_area);
            ui.render_widget(Paragraph::new(footer), shell[2]);

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
                    .and_then(|path| scene_object_property_lines(&scene, path))
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
                );
            }
        })?;

        while event::poll(Duration::from_millis(0))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            if inspector.properties_open {
                if key.code == KeyCode::Esc {
                    inspector.close_properties();
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
                _ if handle_key(key.code, &mut state) == ViewerInput::Quit => return Ok(()),
                _ => {}
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
) {
    let text = lines.join("\n");
    let popup = Paragraph::new(text).block(
        Block::default()
            .title(format!(" Properties: {object_name}  Esc=back "))
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

    let scene = read_scene(&path)?;
    let meshes = load_scene_meshes(Path::new(&path), &scene)?;
    let maps = load_scene_maps(Path::new(&path), &scene)?;

    run_viewer(scene, meshes, maps)
}
