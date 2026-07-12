use super::Frame;

pub fn draw_line_overlay(frame: &mut Frame, a: (i32, i32, f32), b: (i32, i32, f32), ch: char) {
    let mut x0 = a.0;
    let mut y0 = a.1;
    let x1 = b.0;
    let y1 = b.1;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let t = if dx.abs() + dy.abs() > 0 {
            ((x0 - a.0).abs() + (y0 - a.1).abs()) as f32
                / ((x1 - a.0).abs() + (y1 - a.1).abs()).max(1) as f32
        } else {
            0.0
        };

        let z = a.2 * (1.0 - t) + b.2 * t;
        frame.set_overlay(x0, y0, ch);

        if x0 == x1 && y0 == y1 {
            let _ = z;
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            err += dy;
            x0 += sx;
        }

        if e2 <= dx {
            err += dx;
            y0 += sy;
        }

        let _ = z;
    }
}
