pub struct Frame {
    width: usize,
    height: usize,
    cells: Vec<char>,
    depth: Vec<f32>,
}

impl Frame {
    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![' '; width * height],
            depth: vec![f32::INFINITY; width * height],
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(' ');
        self.depth.fill(f32::INFINITY);
    }

    pub fn set(&mut self, x: i32, y: i32, z: f32, ch: char) {
        if x < 0 || y < 0 {
            return;
        }

        let x = x as usize;
        let y = y as usize;

        if x >= self.width || y >= self.height {
            return;
        }

        let index = y * self.width + x;

        if z < self.depth[index] {
            self.depth[index] = z;
            self.cells[index] = ch;
        }
    }

    pub fn set_overlay(&mut self, x: i32, y: i32, ch: char) {
        if x < 0 || y < 0 {
            return;
        }

        let x = x as usize;
        let y = y as usize;

        if x >= self.width || y >= self.height {
            return;
        }

        self.cells[y * self.width + x] = ch;
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str) {
        if y >= self.height {
            return;
        }

        for (offset, ch) in text.chars().enumerate() {
            let x = x + offset;
            if x >= self.width {
                break;
            }

            self.cells[y * self.width + x] = ch;
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::with_capacity((self.width + 2) * self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                out.push(self.cells[y * self.width + x]);
            }
            out.push('\r');
            out.push('\n');
        }

        out
    }
}
