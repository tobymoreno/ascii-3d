use crate::geometry2d::Point2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRect {
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
}

impl ClipRect {
    fn contains(self, point: Point2) -> bool {
        let right = self.x + self.width as i32;
        let bottom = self.y + self.height as i32;

        point.x >= self.x && point.x < right && point.y >= self.y && point.y < bottom
    }
}

pub struct Canvas {
    width: usize,
    height: usize,
    clip_rect: Option<ClipRect>,
    origin_offset: Point2,
    cells: Vec<char>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            clip_rect: None,
            origin_offset: Point2::new(0, 0),
            cells: vec![' '; width * height],
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(' ');
    }

    pub fn with_clip_rect<R>(
        &mut self,
        clip_rect: ClipRect,
        draw: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous_clip_rect = self.clip_rect;

        self.clip_rect = Some(clip_rect);
        let result = draw(self);
        self.clip_rect = previous_clip_rect;

        result
    }

    pub fn with_viewport<R>(&mut self, viewport: ClipRect, draw: impl FnOnce(&mut Self) -> R) -> R {
        let previous_clip_rect = self.clip_rect;
        let previous_origin_offset = self.origin_offset;

        self.clip_rect = Some(viewport);
        self.origin_offset = Point2::new(
            previous_origin_offset.x + viewport.x,
            previous_origin_offset.y + viewport.y,
        );

        let result = draw(self);

        self.origin_offset = previous_origin_offset;
        self.clip_rect = previous_clip_rect;

        result
    }

    pub fn set(&mut self, point: Point2, character: char) {
        let point = Point2::new(
            point.x + self.origin_offset.x,
            point.y + self.origin_offset.y,
        );

        if let Some(clip_rect) = self.clip_rect {
            if !clip_rect.contains(point) {
                return;
            }
        }

        if point.x < 0 || point.y < 0 {
            return;
        }

        let x = point.x as usize;
        let y = point.y as usize;

        if x >= self.width || y >= self.height {
            return;
        }

        self.cells[y * self.width + x] = character;
    }

    pub fn draw_text(&mut self, start: Point2, text: &str) {
        for (offset, character) in text.chars().enumerate() {
            self.set(Point2::new(start.x + offset as i32, start.y), character);
        }
    }

    pub fn draw_line(&mut self, start: Point2, end: Point2, character: char) {
        let mut x0 = start.x;
        let mut y0 = start.y;
        let x1 = end.x;
        let y1 = end.y;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };

        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut error = dx + dy;

        loop {
            self.set(Point2::new(x0, y0), character);

            if x0 == x1 && y0 == y1 {
                break;
            }

            let doubled_error = 2 * error;

            if doubled_error >= dy {
                error += dy;
                x0 += sx;
            }

            if doubled_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_line_auto(&mut self, start: Point2, end: Point2) {
        let dx = end.x - start.x;
        let dy = end.y - start.y;

        let character = if dy == 0 {
            '-'
        } else if dx == 0 {
            '|'
        } else if dx.signum() == dy.signum() {
            '\\'
        } else {
            '/'
        };

        self.draw_line(start, end, character);
    }

    pub fn draw_arrow_auto(&mut self, start: Point2, end: Point2, arrow_character: char) {
        self.draw_line_auto(start, end);
        self.set(end, arrow_character);
    }

    pub fn render(&self) -> String {
        let mut output = String::with_capacity((self.width + 2) * self.height);

        for row in self.cells.chunks(self.width) {
            for character in row {
                output.push(*character);
            }

            // In raw terminal mode, '\n' may move down without returning
            // to column zero. Emit CRLF so every rendered row starts at
            // the left edge on macOS, Linux, and Windows terminals.
            output.push('\r');
            output.push('\n');
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::{Canvas, ClipRect};
    use crate::geometry2d::Point2;

    #[test]
    fn clip_rect_prevents_drawing_outside_region() {
        let mut canvas = Canvas::new(8, 4);

        canvas.with_clip_rect(
            ClipRect {
                x: 2,
                y: 1,
                width: 3,
                height: 2,
            },
            |canvas| {
                canvas.set(Point2::new(1, 1), 'A');
                canvas.set(Point2::new(2, 1), 'B');
                canvas.set(Point2::new(4, 2), 'C');
                canvas.set(Point2::new(5, 2), 'D');
            },
        );

        let rendered = canvas.render();

        assert!(!rendered.contains('A'));
        assert!(rendered.contains('B'));
        assert!(rendered.contains('C'));
        assert!(!rendered.contains('D'));
    }
}
