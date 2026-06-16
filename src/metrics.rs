//! Cell metrics — the geometry contract between the grid and the Face.
//!
//! Font size is a *setting*, not a function of window size (guarantee #4).
//! `CellMetrics` computes `cell_w` and `cell_h` from the monospace font's
//! metrics at the fixed size, then the pure `grid_size` function derives
//! `(cols, rows)` from the available pixel area via floor division. Leftover
//! pixels are absorbed into padding — cells are never stretched (guarantee #3).

/// Fixed cell dimensions and the grid-size calculator.
#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    /// Cell width in logical pixels (advance width of 'M' in `banquo-mono`).
    pub cell_w: f32,
    /// Cell height in logical pixels (row height of `banquo-mono`).
    pub cell_h: f32,
}

impl CellMetrics {
    /// Create metrics from known cell dimensions.
    pub const fn new(cell_w: f32, cell_h: f32) -> Self {
        Self { cell_w, cell_h }
    }

    /// Compute the grid size (cols, rows) that fits in the given available area
    /// with the given padding on each side.
    ///
    /// `cols = floor((avail_w − 2·pad) / cell_w)`, clamped to ≥ 1.
    /// `rows = floor((avail_h − 2·pad) / cell_h)`, clamped to ≥ 1.
    ///
    /// The remainder (slack) is absorbed into padding — cells are never
    /// stretched to fill (guarantees #3 and #4).
    pub fn grid_size(&self, avail_w: f32, avail_h: f32, padding: f32) -> (usize, usize) {
        let usable_w = (avail_w - 2.0 * padding).max(0.0);
        let usable_h = (avail_h - 2.0 * padding).max(0.0);

        let cols = (usable_w / self.cell_w).floor().max(1.0) as usize;
        let rows = (usable_h / self.cell_h).floor().max(1.0) as usize;

        (cols, rows)
    }

    /// Compute the padding needed to center the grid within the available area.
    /// This is half the slack (leftover pixels after floor division) plus the
    /// base padding.
    pub fn centering_offset(
        &self,
        avail_w: f32,
        avail_h: f32,
        padding: f32,
        cols: usize,
        rows: usize,
    ) -> (f32, f32) {
        let grid_w = cols as f32 * self.cell_w;
        let grid_h = rows as f32 * self.cell_h;
        let offset_x = padding + (avail_w - 2.0 * padding - grid_w) / 2.0;
        let offset_y = padding + (avail_h - 2.0 * padding - grid_h) / 2.0;
        (offset_x.max(0.0), offset_y.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_size_floor() {
        let m = CellMetrics::new(10.0, 20.0);
        let (cols, rows) = m.grid_size(800.0, 600.0, 4.0);
        // floor((800 - 8) / 10) = floor(79.2) = 79
        assert_eq!(cols, 79);
        // floor((600 - 8) / 20) = floor(29.6) = 29
        assert_eq!(rows, 29);
    }

    #[test]
    fn test_grid_size_min_one() {
        let m = CellMetrics::new(10.0, 20.0);
        // Available area smaller than one cell
        let (cols, rows) = m.grid_size(5.0, 5.0, 0.0);
        assert_eq!(cols, 1);
        assert_eq!(rows, 1);
    }

    #[test]
    fn test_grid_size_exact() {
        let m = CellMetrics::new(10.0, 20.0);
        // Evenly divisible: (800 - 0) / 10 = 80, (600 - 0) / 20 = 30
        let (cols, rows) = m.grid_size(800.0, 600.0, 0.0);
        assert_eq!(cols, 80);
        assert_eq!(rows, 30);
    }

    #[test]
    fn test_grid_size_with_large_padding() {
        let m = CellMetrics::new(10.0, 20.0);
        // Padding eats most of the space
        let (cols, rows) = m.grid_size(100.0, 100.0, 45.0);
        assert_eq!(cols, 1);
        assert_eq!(rows, 1);
    }

    #[test]
    fn test_grid_size_zero_area() {
        let m = CellMetrics::new(10.0, 20.0);
        let (cols, rows) = m.grid_size(0.0, 0.0, 0.0);
        assert_eq!(cols, 1);
        assert_eq!(rows, 1);
    }
}
