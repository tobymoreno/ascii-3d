use crate::geometry2d::Point2;

pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<char>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![' '; width * height],
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(' ');
    }

    pub fn set(&mut self, point: Point2, character: char) {
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
