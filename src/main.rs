mod canvas;
mod geometry2d;
mod math;
mod projection;

use std::{
    io::{self, Write, stdout},
    time::Duration,
};

use canvas::Canvas;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use geometry2d::Point2;
use math::Vec3;
use projection::ObliqueProjector;

const CANVAS_WIDTH: usize = 80;
const CANVAS_HEIGHT: usize = 28;
const SCENE_COUNT: usize = 4;

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;

        execute!(stdout(), EnterAlternateScreen, Hide, Clear(ClearType::All),)?;

        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show, LeaveAlternateScreen,);

        let _ = disable_raw_mode();
    }
}

fn draw_axes(canvas: &mut Canvas, projector: &ObliqueProjector, include_negative_z: bool) {
    let origin = projector.project(Vec3::zero());

    let positive_x = projector.project(Vec3::new(4.0, 0.0, 0.0));

    let positive_y = projector.project(Vec3::new(0.0, 3.0, 0.0));

    let positive_z = projector.project(Vec3::new(0.0, 0.0, 4.0));

    canvas.draw_arrow_auto(origin, positive_x, '>');
    canvas.draw_arrow_auto(origin, positive_y, '^');
    canvas.draw_arrow_auto(origin, positive_z, 'v');

    canvas.draw_text(Point2::new(positive_x.x + 2, positive_x.y), "+X");

    canvas.draw_text(Point2::new(positive_y.x + 2, positive_y.y), "+Y");

    canvas.draw_text(Point2::new(positive_z.x + 2, positive_z.y), "+Z");

    if include_negative_z {
        let negative_z = projector.project(Vec3::new(0.0, 0.0, -4.0));

        canvas.draw_arrow_auto(origin, negative_z, '^');

        canvas.draw_text(Point2::new(negative_z.x - 4, negative_z.y), "-Z");
    }

    canvas.set(origin, 'O');
}

fn scene_axes(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, false);

    canvas.draw_text(Point2::new(2, 1), "Scene 1: 3D Cartesian axes");

    canvas.draw_text(Point2::new(2, 24), "Origin O = (0, 0, 0)");
}

fn scene_arbitrary_vector(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, false);

    let origin = Vec3::zero();
    let vector = Vec3::new(2.0, 1.0, 3.0);

    let origin_2d = projector.project(origin);
    let vector_2d = projector.project(vector);

    canvas.draw_arrow_auto(origin_2d, vector_2d, '*');

    canvas.draw_text(Point2::new(vector_2d.x + 2, vector_2d.y), "V(2,1,3)");

    let normalized = vector.normalized();

    canvas.draw_text(Point2::new(2, 1), "Scene 2: arbitrary Vec3");

    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "length={:.3} normalized=({:.3}, {:.3}, {:.3})",
            vector.length(),
            normalized.x,
            normalized.y,
            normalized.z,
        ),
    );
}

fn scene_cross_positive_z(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, true);

    let origin = Vec3::zero();

    // Both vectors lie in the XY plane.
    let vector_a = Vec3::new(3.0, 1.0, 0.0);
    let vector_b = Vec3::new(1.0, 2.0, 0.0);

    // A x B = (0, 0, 5), which points along +Z.
    let cross = vector_a.cross(vector_b);
    let displayed_cross = cross.normalized() * 3.0;

    let origin_2d = projector.project(origin);
    let vector_a_2d = projector.project(vector_a);
    let vector_b_2d = projector.project(vector_b);
    let cross_2d = projector.project(displayed_cross);

    canvas.draw_arrow_auto(origin_2d, vector_a_2d, 'A');
    canvas.draw_arrow_auto(origin_2d, vector_b_2d, 'B');
    canvas.draw_arrow_auto(origin_2d, cross_2d, 'N');

    canvas.draw_text(Point2::new(vector_a_2d.x + 2, vector_a_2d.y), "A");

    canvas.draw_text(Point2::new(vector_b_2d.x + 2, vector_b_2d.y), "B");

    canvas.draw_text(Point2::new(cross_2d.x + 2, cross_2d.y), "A x B");

    canvas.draw_text(Point2::new(2, 1), "Scene 3: A x B points along +Z");

    canvas.draw_text(
        Point2::new(2, 24),
        &format!("A x B = ({:.1}, {:.1}, {:.1})", cross.x, cross.y, cross.z,),
    );

    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "(A x B) dot A = {:.1}    (A x B) dot B = {:.1}",
            cross.dot(vector_a),
            cross.dot(vector_b),
        ),
    );
}

fn scene_cross_negative_z(canvas: &mut Canvas, projector: &ObliqueProjector) {
    draw_axes(canvas, projector, true);

    let origin = Vec3::zero();

    let vector_a = Vec3::new(3.0, 1.0, 0.0);
    let vector_b = Vec3::new(1.0, 2.0, 0.0);

    // Reversing the order reverses the normal:
    //
    // B x A = -(A x B) = (0, 0, -5)
    let cross = vector_b.cross(vector_a);
    let displayed_cross = cross.normalized() * 3.0;

    let origin_2d = projector.project(origin);
    let vector_a_2d = projector.project(vector_a);
    let vector_b_2d = projector.project(vector_b);
    let cross_2d = projector.project(displayed_cross);

    canvas.draw_arrow_auto(origin_2d, vector_a_2d, 'A');
    canvas.draw_arrow_auto(origin_2d, vector_b_2d, 'B');
    canvas.draw_arrow_auto(origin_2d, cross_2d, 'N');

    canvas.draw_text(Point2::new(vector_a_2d.x + 2, vector_a_2d.y), "A");

    canvas.draw_text(Point2::new(vector_b_2d.x + 2, vector_b_2d.y), "B");

    canvas.draw_text(Point2::new(cross_2d.x - 7, cross_2d.y), "B x A");

    canvas.draw_text(Point2::new(2, 1), "Scene 4: B x A points along -Z");

    canvas.draw_text(
        Point2::new(2, 24),
        &format!("B x A = ({:.1}, {:.1}, {:.1})", cross.x, cross.y, cross.z,),
    );

    canvas.draw_text(
        Point2::new(2, 25),
        "Changing operand order reverses the cross product.",
    );
}

fn render_scene(scene_index: usize) -> io::Result<()> {
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);

    let projector = ObliqueProjector::new(Point2::new(34, 14));

    match scene_index {
        0 => scene_axes(&mut canvas, &projector),
        1 => scene_arbitrary_vector(&mut canvas, &projector),
        2 => scene_cross_positive_z(&mut canvas, &projector),
        3 => scene_cross_negative_z(&mut canvas, &projector),
        _ => unreachable!("invalid scene index"),
    }

    canvas.draw_text(
        Point2::new(2, 27),
        &format!(
            "[Scene {}/{}] Space/Right: next  Left: previous  Q/Esc: quit",
            scene_index + 1,
            SCENE_COUNT,
        ),
    );

    let mut output = stdout();

    execute!(output, MoveTo(0, 0), Clear(ClearType::All),)?;

    write!(output, "{}", canvas.render())?;
    output.flush()
}

fn main() -> io::Result<()> {
    let _terminal = TerminalGuard::enter()?;

    let mut scene_index = 0;
    render_scene(scene_index)?;

    loop {
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char(' ') | KeyCode::Right | KeyCode::Enter => {
                scene_index = (scene_index + 1) % SCENE_COUNT;

                render_scene(scene_index)?;
            }

            KeyCode::Left => {
                scene_index = if scene_index == 0 {
                    SCENE_COUNT - 1
                } else {
                    scene_index - 1
                };

                render_scene(scene_index)?;
            }

            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,

            _ => {}
        }
    }

    Ok(())
}
