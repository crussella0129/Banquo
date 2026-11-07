//! A 3x5 bitmap font, enough for the spike's status lines. Lower-case input is
//! drawn with the upper-case glyphs; unknown characters draw as a box.

/// Glyph rows, top to bottom; bit 2 is the left column.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b110],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '(' => [0b001, 0b010, 0b010, 0b010, 0b001],
        ')' => [0b100, 0b010, 0b010, 0b010, 0b100],
        '_' => [0b000, 0b000, 0b000, 0b000, 0b111],
        ' ' => [0; 5],
        _ => [0b111, 0b101, 0b101, 0b101, 0b111],
    }
}

/// An RGBA8 canvas.
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn fill(&mut self, rgba: [u8; 4]) {
        for px in self.pixels.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&rgba);
        }
    }

    /// Fills a rectangle, clipped to the canvas.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let i = (yy as usize * self.width as usize + xx as usize) * 4;
                self.pixels[i..i + 4].copy_from_slice(&rgba);
            }
        }
    }

    /// Draws `text` at (x, y) with `scale`-pixel dots; returns the width drawn.
    pub fn text(&mut self, x: i32, y: i32, scale: i32, text: &str, rgba: [u8; 4]) -> i32 {
        let mut cx = x;
        for c in text.chars() {
            let rows = glyph(c);
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..3 {
                    if bits & (0b100 >> col) != 0 {
                        self.rect(cx + col * scale, y + row as i32 * scale, scale, scale, rgba);
                    }
                }
            }
            cx += 4 * scale;
        }
        cx - x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_draws_pixels_and_advances() {
        let mut c = Canvas::new(64, 16);
        let w = c.text(0, 0, 1, "steam: off", [255, 255, 255, 255]);
        assert_eq!(w, 40);
        assert!(c.pixels.as_chunks::<4>().0.iter().any(|p| p[0] == 255));
    }

    #[test]
    fn rect_clips() {
        let mut c = Canvas::new(4, 4);
        c.rect(-2, -2, 100, 100, [1, 2, 3, 4]);
        assert!(c
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [1, 2, 3, 4]));
    }
}
