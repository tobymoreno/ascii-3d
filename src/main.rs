mod canvas;
mod geometry2d;
mod math;
mod mesh;
mod mesh_renderer;
mod obj;
mod projection;

use std::{
    io::{self, Write, stdout},
    path::Path,
    time::{Duration, Instant},
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
use mesh::Mesh;
use mesh_renderer::{MeshTransform, draw_wireframe};
use obj::load_obj;
use projection::ObliqueProjector;

const CANVAS_WIDTH: usize = 80;
const CANVAS_HEIGHT: usize = 28;
const SCENE_COUNT: usize = 8;

const FULL_ROTATION_DEGREES: f32 = 360.0;
const ROTATION_SPEED_DEGREES_PER_SECOND: f32 = 30.0;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / 60);

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

    let vector_a = Vec3::new(3.0, 1.0, 0.0);
    let vector_b = Vec3::new(1.0, 2.0, 0.0);

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

#[derive(Clone, Copy)]
enum RotationAxis {
    X,
    Y,
    Z,
}

impl RotationAxis {
    fn name(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    fn rotate(self, vector: Vec3, angle_radians: f32) -> Vec3 {
        match self {
            Self::X => vector.rotate_x(angle_radians),
            Self::Y => vector.rotate_y(angle_radians),
            Self::Z => vector.rotate_z(angle_radians),
        }
    }
}

fn scene_rotation(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    axis: RotationAxis,
    angle_degrees: f32,
) {
    let origin = Vec3::zero();

    let base_x = Vec3::new(4.0, 0.0, 0.0);
    let base_y = Vec3::new(0.0, 3.0, 0.0);
    let base_z = Vec3::new(0.0, 0.0, 4.0);

    let angle_radians = angle_degrees.to_radians();

    let rotated_x = axis.rotate(base_x, angle_radians);
    let rotated_y = axis.rotate(base_y, angle_radians);
    let rotated_z = axis.rotate(base_z, angle_radians);

    let origin_2d = projector.project(origin);
    let x_2d = projector.project(rotated_x);
    let y_2d = projector.project(rotated_y);
    let z_2d = projector.project(rotated_z);

    canvas.draw_arrow_auto(origin_2d, x_2d, '>');
    canvas.draw_arrow_auto(origin_2d, y_2d, '^');
    canvas.draw_arrow_auto(origin_2d, z_2d, 'v');

    canvas.set(origin_2d, 'O');

    canvas.draw_text(Point2::new(x_2d.x + 2, x_2d.y), "+X");

    canvas.draw_text(Point2::new(y_2d.x + 2, y_2d.y), "+Y");

    canvas.draw_text(Point2::new(z_2d.x + 2, z_2d.y), "+Z");

    canvas.draw_text(
        Point2::new(2, 1),
        &format!(
            "Rotate Cartesian axes around {}: {:06.1} / {:.1} degrees",
            axis.name(),
            angle_degrees,
            FULL_ROTATION_DEGREES,
        ),
    );

    canvas.draw_text(Point2::new(2, 24), "Origin O = (0, 0, 0)");

    canvas.draw_text(
        Point2::new(2, 25),
        &format!(
            "Rotating around {} leaves the {} axis unchanged.",
            axis.name(),
            axis.name(),
        ),
    );

    canvas.draw_text(Point2::new(2, 26), "The other two axes sweep around it.");
}

fn scene_obj_box(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    mesh: &Mesh,
    angle_degrees: f32,
) -> io::Result<()> {
    draw_axes(canvas, projector, true);

    let angle_radians = angle_degrees.to_radians();

    let transform = MeshTransform {
        rotation_x: angle_radians * 0.7,
        rotation_y: angle_radians,
        rotation_z: angle_radians * 0.35,
        scale: 3.0,
        translation: Vec3::zero(),
    };

    draw_wireframe(canvas, projector, mesh, transform).map_err(io::Error::other)?;

    canvas.draw_text(
        Point2::new(2, 1),
        &format!(
            "Scene 8: rotating OBJ wireframe box  angle={:06.1}",
            angle_degrees,
        ),
    );

    canvas.draw_text(
        Point2::new(2, 24),
        &format!(
            "vertices={}  faces={}  unique edges={}",
            mesh.vertices.len(),
            mesh.faces.len(),
            mesh.unique_edges().len(),
        ),
    );

    canvas.draw_text(Point2::new(2, 25), "Source: assets/box.obj");

    canvas.draw_text(
        Point2::new(2, 26),
        "Centered at origin; largest dimension normalized to 1.0",
    );

    Ok(())
}

fn render_scene(
    scene_index: usize,
    rotation_angle_degrees: f32,
    box_angle_degrees: f32,
    box_mesh: &Mesh,
) -> io::Result<()> {
    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT);

    let projector = ObliqueProjector::new(Point2::new(34, 14));

    match scene_index {
        0 => scene_axes(&mut canvas, &projector),

        1 => scene_arbitrary_vector(&mut canvas, &projector),

        2 => scene_cross_positive_z(&mut canvas, &projector),

        3 => scene_cross_negative_z(&mut canvas, &projector),

        4 => scene_rotation(
            &mut canvas,
            &projector,
            RotationAxis::X,
            rotation_angle_degrees,
        ),

        5 => scene_rotation(
            &mut canvas,
            &projector,
            RotationAxis::Y,
            rotation_angle_degrees,
        ),

        6 => scene_rotation(
            &mut canvas,
            &projector,
            RotationAxis::Z,
            rotation_angle_degrees,
        ),

        7 => scene_obj_box(&mut canvas, &projector, box_mesh, box_angle_degrees)?,

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

    execute!(output, MoveTo(0, 0))?;

    write!(output, "{}", canvas.render())?;
    output.flush()
}

fn is_rotation_scene(scene_index: usize) -> bool {
    matches!(scene_index, 4..=6)
}

fn is_box_scene(scene_index: usize) -> bool {
    scene_index == 7
}

fn main() -> io::Result<()> {
    let box_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("box.obj");

    let mut box_mesh = load_obj(&box_path).map_err(io::Error::other)?;

    if !box_mesh.normalize_to_size(1.0) {
        return Err(io::Error::other(format!(
            "could not normalize OBJ mesh: {}",
            box_path.display(),
        )));
    }

    let _terminal = TerminalGuard::enter()?;

    let mut scene_index = 0;
    let mut rotation_angle_degrees = 0.0_f32;
    let mut box_angle_degrees = 0.0_f32;
    let mut previous_time = Instant::now();

    render_scene(
        scene_index,
        rotation_angle_degrees,
        box_angle_degrees,
        &box_mesh,
    )?;

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(previous_time);
        previous_time = now;

        let mut redraw = false;

        if is_rotation_scene(scene_index) {
            rotation_angle_degrees += elapsed.as_secs_f32() * ROTATION_SPEED_DEGREES_PER_SECOND;

            rotation_angle_degrees %= FULL_ROTATION_DEGREES;

            redraw = true;
        }

        if is_box_scene(scene_index) {
            box_angle_degrees += elapsed.as_secs_f32() * ROTATION_SPEED_DEGREES_PER_SECOND;

            box_angle_degrees %= 360.0;
            redraw = true;
        }

        if redraw {
            render_scene(
                scene_index,
                rotation_angle_degrees,
                box_angle_degrees,
                &box_mesh,
            )?;
        }

        if !event::poll(FRAME_DURATION)? {
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

                rotation_angle_degrees = 0.0;
                box_angle_degrees = 0.0;
                previous_time = Instant::now();

                render_scene(
                    scene_index,
                    rotation_angle_degrees,
                    box_angle_degrees,
                    &box_mesh,
                )?;
            }

            KeyCode::Left => {
                scene_index = if scene_index == 0 {
                    SCENE_COUNT - 1
                } else {
                    scene_index - 1
                };

                rotation_angle_degrees = 0.0;
                box_angle_degrees = 0.0;
                previous_time = Instant::now();

                render_scene(
                    scene_index,
                    rotation_angle_degrees,
                    box_angle_degrees,
                    &box_mesh,
                )?;
            }

            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,

            _ => {}
        }
    }

    Ok(())
}
