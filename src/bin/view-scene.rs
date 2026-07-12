use ascii_3d::{
    render::{apply_render_behaviors_to_scene, Frame, GeoJsonMapAsset, MeshAsset, RenderScene},
    viewer::{
        draw_render_scene, handle_key, load_scene_maps, load_scene_meshes, read_scene,
        ViewerInput, ViewerState, ViewerViewport, MIN_VIEW_SCENE_HEIGHT, MIN_VIEW_SCENE_WIDTH,
    },
};
use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::Paragraph,
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
            let render_width = (scene_area.width as usize).max(MIN_VIEW_SCENE_WIDTH);
            let render_height = (scene_area.height as usize).max(MIN_VIEW_SCENE_HEIGHT);

            if frame.width() != render_width || frame.height() != render_height {
                frame = Frame::new(render_width, render_height);
            }

            let viewport = ViewerViewport::terminal(render_width, render_height);
            draw_render_scene(&mut frame, viewport, &scene, &meshes, &maps, &state);

            let rendered = frame.render().replace('\r', "");
            let header = format!(
                "{} | visible {}x{} | render {}x{} | fps {:>5.1}",
                scene.name,
                scene_area.width,
                scene_area.height,
                render_width,
                render_height,
                state.fps,
            );
            let footer =
                "arrows origin | PgUp/PgDn z | +/- zoom | x/y/z rotate | a/A axes | r reset | q quit";

            ui.render_widget(Paragraph::new(header), shell[0]);
            ui.render_widget(Paragraph::new(rendered), scene_area);
            ui.render_widget(Paragraph::new(footer), shell[2]);
        })?;

        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(key.code, &mut state) == ViewerInput::Quit {
                    return Ok(());
                }
            }
        }

        let elapsed = frame_start.elapsed();

        if elapsed < target_frame {
            std::thread::sleep(target_frame - elapsed);
        }
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
