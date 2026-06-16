//! The immutable snapshot — the only truth type the Face ever sees.
//!
//! A `Snapshot` is a complete, consistent picture of the terminal grid at one
//! instant: every cell's character, colors, and attributes, plus the cursor
//! position and visibility. The reader thread builds one after each PTY read
//! burst and publishes it via `ArcSwap`; the Face loads the latest and paints
//! it. Because the snapshot is immutable and behind an `Arc`, the Face never
//! holds a lock on the live grid (design §IV, guarantee #2).
//!
//! **Boundary rule (ADR-003/004):** no `alacritty_*` type appears here. The
//! translation happens in [`super::term`]; downstream code imports only this
//! module.

/// An RGB color triple. Banquo's own color type — the translation from
/// `vte::ansi::Color` happens in [`super::term::resolve_color`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl Default for Rgb {
    fn default() -> Self {
        // Warm off-white — the default foreground glyph color.
        Self::new(235, 232, 226)
    }
}

/// Per-cell text attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellAttrs {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

/// A single terminal cell: one character, its colors, and its attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed in this cell. Space for empty/default.
    pub ch: char,
    /// Foreground color.
    pub fg: Rgb,
    /// Background color.
    pub bg: Rgb,
    /// Text attributes (bold, italic, underline, inverse).
    pub attrs: CellAttrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Rgb::default(),
            bg: Rgb::new(0, 0, 0),
            attrs: CellAttrs::default(),
        }
    }
}

/// The cursor's position in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPos {
    pub col: usize,
    pub row: usize,
}

/// An immutable snapshot of the terminal grid at one instant.
///
/// Built by [`super::term::BanquoTerm::build_snapshot`] on the reader thread,
/// published via `ArcSwap`, consumed by the Face. Contains the full cell grid
/// (row-major), cursor, and visibility.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Number of columns in the grid.
    pub cols: usize,
    /// Number of rows in the grid.
    pub rows: usize,
    /// Row-major cell storage: `cells[row * cols + col]`.
    cells: Vec<Cell>,
    /// Cursor position (col, row).
    pub cursor: CursorPos,
    /// Whether the cursor is visible.
    pub cursor_visible: bool,
}

impl Snapshot {
    /// Create a blank snapshot of the given dimensions with default (space) cells.
    pub fn empty(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); cols * rows],
            cursor: CursorPos::default(),
            cursor_visible: true,
        }
    }

    /// Create a snapshot from pre-built cell data.
    pub fn new(
        cols: usize,
        rows: usize,
        cells: Vec<Cell>,
        cursor: CursorPos,
        cursor_visible: bool,
    ) -> Self {
        debug_assert_eq!(cells.len(), cols * rows, "cell count must match grid dims");
        Self {
            cols,
            rows,
            cells,
            cursor,
            cursor_visible,
        }
    }

    /// Access the cell at `(col, row)`. Returns `None` for out-of-bounds
    /// coordinates (no panic — the Face should never crash on a stale snapshot
    /// that disagrees with its layout).
    pub fn cell(&self, col: usize, row: usize) -> Option<&Cell> {
        if col < self.cols && row < self.rows {
            Some(&self.cells[row * self.cols + col])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_empty_dims() {
        let snap = Snapshot::empty(80, 24);
        assert_eq!(snap.cols, 80);
        assert_eq!(snap.rows, 24);
    }

    #[test]
    fn test_snapshot_empty_cells_are_blank() {
        let snap = Snapshot::empty(80, 24);
        for row in 0..24 {
            for col in 0..80 {
                let cell = snap.cell(col, row).expect("in-bounds cell must exist");
                assert_eq!(cell.ch, ' ', "default cell must be a space");
                assert_eq!(cell.attrs, CellAttrs::default());
            }
        }
    }

    #[test]
    fn test_snapshot_cell_out_of_bounds() {
        let snap = Snapshot::empty(80, 24);
        // Exact boundary
        assert!(snap.cell(80, 0).is_none());
        assert!(snap.cell(0, 24).is_none());
        // Way out of bounds
        assert!(snap.cell(9999, 9999).is_none());
    }

    #[test]
    fn test_snapshot_cursor_default() {
        let snap = Snapshot::empty(80, 24);
        assert_eq!(snap.cursor, CursorPos { col: 0, row: 0 });
        assert!(snap.cursor_visible);
    }
}
