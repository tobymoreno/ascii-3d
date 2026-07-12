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
    execute, terminal,
};
use std::{
    collections::HashMap,
    env, io,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn run_viewer(mut scene: RenderScene, meshes: HashMap<String, MeshAsset>, maps: HashMap<String, GeoJsonMapAsset>) -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    let mut state = ViewerState::default();
    let mut frame = Frame::new(MIN_VIEW_SCENE_WIDTH, MIN_VIEW_SCENE_HEIGHT);
    let target_frame = Duration::from_millis(33);
    let mut previous_frame_start = Instant::now();

    // Clear once when entering the viewer. After this, every frame overwrites
    // the same fixed-size buffer without clearing the terminal, which prevents flicker.
    execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;

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

        let viewport = ViewerViewport::new(frame.width(), frame.height());
        draw_render_scene(&mut frame, viewport, &scene, &meshes, &maps, &state);

        let rendered = frame.render();

        execute!(stdout, cursor::MoveTo(0, 0))?;
        write!(stdout, "{}", rendered)?;
        stdout.flush()?;

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
