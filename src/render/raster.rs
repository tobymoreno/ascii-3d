use super::Frame;

fn edge(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (p.0 - a.0) * (b.1 - a.1) - (p.1 - a.1) * (b.0 - a.0)
}

pub fn fill_triangle(
    frame: &mut Frame,
    p0: (i32, i32, f32),
    p1: (i32, i32, f32),
    p2: (i32, i32, f32),
    ch: char,
) {
    let min_x = p0.0.min(p1.0).min(p2.0).max(0);
    let max_x = p0.0.max(p1.0).max(p2.0);
    let min_y = p0.1.min(p1.1).min(p2.1).max(0);
    let max_y = p0.1.max(p1.1).max(p2.1);

    let a = (p0.0 as f32, p0.1 as f32);
    let b = (p1.0 as f32, p1.1 as f32);
    let c = (p2.0 as f32, p2.1 as f32);
    let area = edge(a, b, c);

    if area.abs() <= f32::EPSILON {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = (x as f32 + 0.5, y as f32 + 0.5);

            let w0 = edge(b, c, point);
            let w1 = edge(c, a, point);
            let w2 = edge(a, b, point);

            let inside_positive = w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0;
            let inside_negative = w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0;

            if !inside_positive && !inside_negative {
                continue;
            }

            let inv_area = 1.0 / area;
            let alpha = w0 * inv_area;
            let beta = w1 * inv_area;
            let gamma = w2 * inv_area;
            let depth = p0.2 * alpha + p1.2 * beta + p2.2 * gamma;

            frame.set(x, y, depth, ch);
        }
    }
}

pub fn draw_line(frame: &mut Frame, a: (i32, i32, f32), b: (i32, i32, f32), ch: char) {
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = a.0;
    let mut y = a.1;

    loop {
        let t = if dx.abs() > (-dy).abs() {
            if dx == 0 {
                0.0
            } else {
                (x - a.0) as f32 / (b.0 - a.0) as f32
            }
        } else if dy == 0 {
            0.0
        } else {
            (y - a.1) as f32 / (b.1 - a.1) as f32
        };

        let depth = a.2 + (b.2 - a.2) * t.clamp(0.0, 1.0);
        frame.set_overlay(x, y, ch);

        if x == b.0 && y == b.1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            err += dx;
            y += sy;
        }

        let _ = depth;
    }
}

#[cfg(test)]
mod tests {
    use super::{draw_line, fill_triangle};
    use crate::render::Frame;

    #[test]
    fn fill_triangle_marks_frame() {
        let mut frame = Frame::new(8, 6);

        fill_triangle(&mut frame, (1, 1, 0.0), (5, 1, 0.0), (1, 4, 0.0), '#');

        assert!(frame.render().contains('#'));
    }

    #[test]
    fn draw_line_marks_frame() {
        let mut frame = Frame::new(8, 6);

        draw_line(&mut frame, (0, 0, 0.0), (5, 0, 0.0), '-');

        assert!(frame.render().contains('-'));
    }
}
