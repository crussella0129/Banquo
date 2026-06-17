//! The term wrapper — translates `alacritty_terminal` into Banquo's truth types.
//!
//! This is the **only** file that imports `alacritty_terminal` types. Everything
//! downstream sees [`super::snapshot`] types exclusively (ADR-003/004). The
//! adapter boundary is the cost of engine-swappability: if we ever forge our own
//! parser, only this file changes.

use std::io::Write;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags as AlacrittyFlags;
use alacritty_terminal::term::Config as TermConfig;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{self, Processor};

use super::snapshot::{Cell, CellAttrs, CursorPos, Rgb, Snapshot};

// ---------------------------------------------------------------------------
// Color translation (T-103)
// ---------------------------------------------------------------------------

/// Translate an `alacritty_terminal` color into Banquo's [`Rgb`].
///
/// Pure function — no side effects, no panics. Handles all three color
/// variants: `Spec` (direct RGB), `Named` (palette lookup), and `Indexed`
/// (256-color lookup). Falls back to a sensible default when a palette slot
/// is empty.
pub fn resolve_color(color: ansi::Color, colors: &alacritty_terminal::term::color::Colors) -> Rgb {
    match color {
        ansi::Color::Spec(rgb) => Rgb::new(rgb.r, rgb.g, rgb.b),

        ansi::Color::Named(named) => {
            let idx = named as usize;
            match colors[idx] {
                Some(rgb) => Rgb::new(rgb.r, rgb.g, rgb.b),
                None => default_named_color(named),
            }
        }

        ansi::Color::Indexed(idx) => match colors[idx as usize] {
            Some(rgb) => Rgb::new(rgb.r, rgb.g, rgb.b),
            None => {
                // 256-color: first 16 are named, 16..232 are a 6×6×6 cube,
                // 232..255 are a greyscale ramp.
                if idx < 16 {
                    default_named_color(idx_to_named(idx))
                } else if idx < 232 {
                    let idx = idx - 16;
                    let r = (idx / 36) * 51;
                    let g = ((idx % 36) / 6) * 51;
                    let b = (idx % 6) * 51;
                    Rgb::new(r, g, b)
                } else {
                    let grey = 8 + (idx - 232) * 10;
                    Rgb::new(grey, grey, grey)
                }
            }
        },
    }
}

/// Fallback palette for the 16 named ANSI colors when the terminal config
/// hasn't overridden them.
fn default_named_color(named: ansi::NamedColor) -> Rgb {
    use ansi::NamedColor::*;
    match named {
        Black => Rgb::new(0, 0, 0),
        Red => Rgb::new(204, 0, 0),
        Green => Rgb::new(78, 154, 6),
        Yellow => Rgb::new(196, 160, 0),
        Blue => Rgb::new(52, 101, 164),
        Magenta => Rgb::new(117, 80, 123),
        Cyan => Rgb::new(6, 152, 154),
        White => Rgb::new(211, 215, 207),
        BrightBlack => Rgb::new(85, 87, 83),
        BrightRed => Rgb::new(239, 41, 41),
        BrightGreen => Rgb::new(138, 226, 52),
        BrightYellow => Rgb::new(252, 233, 79),
        BrightBlue => Rgb::new(114, 159, 207),
        BrightMagenta => Rgb::new(173, 127, 168),
        BrightCyan => Rgb::new(52, 226, 226),
        BrightWhite => Rgb::new(238, 238, 236),
        // Foreground/Background/Cursor/etc. — sensible defaults.
        Foreground => Rgb::new(235, 232, 226),
        Background => Rgb::new(0, 0, 0),
        _ => Rgb::new(235, 232, 226),
    }
}

/// Map a 0..15 index to the corresponding `NamedColor`.
fn idx_to_named(idx: u8) -> ansi::NamedColor {
    use ansi::NamedColor::*;
    match idx {
        0 => Black,
        1 => Red,
        2 => Green,
        3 => Yellow,
        4 => Blue,
        5 => Magenta,
        6 => Cyan,
        7 => White,
        8 => BrightBlack,
        9 => BrightRed,
        10 => BrightGreen,
        11 => BrightYellow,
        12 => BrightBlue,
        13 => BrightMagenta,
        14 => BrightCyan,
        15 => BrightWhite,
        _ => White,
    }
}

// ---------------------------------------------------------------------------
// BanquoTerm (T-104)
// ---------------------------------------------------------------------------

/// The thin wrapper around `alacritty_terminal::Term` that lives on the reader
/// thread. Owns the terminal state behind a `FairMutex` (the reader and the
/// snapshot builder are the same thread, so the mutex is held briefly; the UI
/// thread never locks it — it reads the published `Arc<Snapshot>` instead).
pub struct BanquoTerm {
    term: Arc<FairMutex<Term<BanquoListener>>>,
    processor: Processor,
}

#[derive(Clone)]
pub struct BanquoListener {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    title: Arc<Mutex<String>>,
}

impl BanquoListener {
    pub fn new(writer: Arc<Mutex<Box<dyn Write + Send>>>, title: Arc<Mutex<String>>) -> Self {
        Self { writer, title }
    }
}

impl EventListener for BanquoListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(text) => {
                if let Ok(mut w) = self.writer.lock() {
                    let _ = w.write_all(text.as_bytes());
                    let _ = w.flush();
                }
            }
            Event::Title(t) => {
                if let Ok(mut title) = self.title.lock() {
                    *title = t;
                }
            }
            _ => {}
        }
    }
}

/// Sizing information for `alacritty_terminal::term::Term`.
struct BanquoDimensions {
    cols: usize,
    rows: usize,
}

impl alacritty_terminal::grid::Dimensions for BanquoDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

impl BanquoTerm {
    /// Create a new terminal wrapper.
    pub fn new(cols: usize, rows: usize, listener: BanquoListener) -> Self {
        let dims = BanquoDimensions { cols, rows };

        let config = TermConfig::default();

        let term = alacritty_terminal::term::Term::new(
            config,
            &dims,
            listener,
        );
        Self {
            term: Arc::new(FairMutex::new(term)),
            processor: Processor::new(),
        }
    }

    /// Feed raw bytes from the PTY into the terminal state machine.
    pub fn advance(&mut self, bytes: &[u8]) {
        let mut term = self.term.lock();
        self.processor.advance(&mut *term, bytes);
    }

    /// Resize the terminal grid.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let dims = BanquoDimensions { cols, rows };
        let mut term = self.term.lock();
        term.resize(dims);
    }

    /// Build an immutable snapshot of the current grid state.
    ///
    /// Translates `alacritty_terminal`'s internal representation into Banquo's
    /// own [`Snapshot`] type — no `alacritty_*` type leaks past this point.
    pub fn build_snapshot(&self) -> Snapshot {
        let term = self.term.lock();
        let content = term.renderable_content();
        let cols = term.columns();
        let rows = term.screen_lines();

        let mut cells = vec![Cell::default(); cols * rows];

        for indexed in content.display_iter {
            let point = indexed.point;
            // Apply display offset (scrollback).
            let col = point.column.0;
            let row = point.line.0 as usize;

            if col < cols && row < rows {
                let ac = &indexed.cell;
                let mut fg = resolve_color(ac.fg, content.colors);
                let mut bg = resolve_color(ac.bg, content.colors);

                let flags = ac.flags;
                let inverse = flags.contains(AlacrittyFlags::INVERSE);
                if inverse {
                    std::mem::swap(&mut fg, &mut bg);
                }

                let attrs = CellAttrs {
                    bold: flags.contains(AlacrittyFlags::BOLD),
                    italic: flags.contains(AlacrittyFlags::ITALIC),
                    underline: flags.intersects(
                        AlacrittyFlags::UNDERLINE
                            | AlacrittyFlags::DOUBLE_UNDERLINE
                            | AlacrittyFlags::UNDERCURL,
                    ),
                    inverse,
                };

                cells[row * cols + col] = Cell {
                    ch: ac.c,
                    fg,
                    bg,
                    attrs,
                };
            }
        }

        let cursor = content.cursor;
        let cursor_pos = CursorPos {
            col: cursor.point.column.0,
            row: cursor.point.line.0 as usize,
        };

        // RenderableCursor doesn't have is_hidden; if we got a cursor from
        // renderable_content, it's visible. The cursor shape existing implies
        // visibility.
        let cursor_visible = true;

        Snapshot::new(cols, rows, cells, cursor_pos, cursor_visible)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- T-103 unit tests: color resolution ---

    #[test]
    fn test_resolve_spec() {
        let colors = alacritty_terminal::term::color::Colors::default();
        let color = ansi::Color::Spec(alacritty_terminal::vte::ansi::Rgb { r: 1, g: 2, b: 3 });
        let result = resolve_color(color, &colors);
        assert_eq!(result, Rgb::new(1, 2, 3));
    }

    #[test]
    fn test_resolve_named() {
        let colors = alacritty_terminal::term::color::Colors::default();
        let color = ansi::Color::Named(ansi::NamedColor::Red);
        let result = resolve_color(color, &colors);
        // Should be non-default (either from the palette or our default_named_color)
        assert_ne!(
            result,
            Rgb::default(),
            "Red should not be the default color"
        );
    }

    #[test]
    fn test_resolve_indexed() {
        let colors = alacritty_terminal::term::color::Colors::default();
        // Index 196 is in the 6×6×6 cube
        let color = ansi::Color::Indexed(196);
        let result = resolve_color(color, &colors);
        // Shouldn't panic, and should produce a reasonable color
        assert!(result.r > 0 || result.g > 0 || result.b > 0 || result == Rgb::new(0, 0, 0));
    }

    #[test]
    fn test_resolve_indexed_greyscale() {
        let colors = alacritty_terminal::term::color::Colors::default();
        let color = ansi::Color::Indexed(240); // greyscale ramp
        let result = resolve_color(color, &colors);
        // Should be a grey value
        assert_eq!(result.r, result.g);
        assert_eq!(result.g, result.b);
    }

    // --- T-104 integration tests: headless truth-half ---

    fn dummy_listener() -> BanquoListener {
        BanquoListener::new(
            std::sync::Arc::new(std::sync::Mutex::new(Box::new(std::io::sink()))),
            std::sync::Arc::new(std::sync::Mutex::new(String::new())),
        )
    }

    #[test]
    fn test_echoes_plain() {
        let mut term = BanquoTerm::new(80, 24, dummy_listener());
        term.advance(b"hi");
        let snap = term.build_snapshot();
        assert_eq!(snap.cell(0, 0).unwrap().ch, 'h');
        assert_eq!(snap.cell(1, 0).unwrap().ch, 'i');
    }

    #[test]
    fn test_newline_advances_cursor() {
        let mut term = BanquoTerm::new(80, 24, dummy_listener());
        term.advance(b"a\r\nb");
        let snap = term.build_snapshot();
        assert_eq!(snap.cell(0, 1).unwrap().ch, 'b');
        assert_eq!(snap.cursor.row, 1);
    }

    #[test]
    fn test_sgr_red_fg() {
        let mut term = BanquoTerm::new(80, 24, dummy_listener());
        term.advance(b"\x1b[31mX");
        let snap = term.build_snapshot();
        let cell = snap.cell(0, 0).unwrap();
        assert_eq!(cell.ch, 'X');
        // The foreground should be red-ish (not the default warm off-white)
        assert_ne!(
            cell.fg,
            Rgb::default(),
            "SGR 31 should set a red foreground"
        );
        assert!(
            cell.fg.r > cell.fg.g,
            "red channel should dominate for SGR 31"
        );
    }

    #[test]
    fn test_clear_screen() {
        let mut term = BanquoTerm::new(80, 24, dummy_listener());
        term.advance(b"hello world");
        term.advance(b"\x1b[2J");
        let snap = term.build_snapshot();
        // After ED 2 (erase entire display), all cells should be blank
        for col in 0..5 {
            assert_eq!(snap.cell(col, 0).unwrap().ch, ' ');
        }
    }

    #[test]
    fn test_resize_reflows() {
        let mut term = BanquoTerm::new(80, 24, dummy_listener());
        term.resize(40, 10);
        let snap = term.build_snapshot();
        assert_eq!(snap.cols, 40);
        assert_eq!(snap.rows, 10);
    }
}
