//! The Face (design §IV): the UI half that paints appearance from truth.
//!
//! At Milestone 2, the Face reads an immutable [`Snapshot`] published by the
//! reader thread (via `ArcSwap`) and paints each cell's background and glyph
//! in `banquo-mono` (Iosevka) at the cell's exact grid coordinate (guarantee
//! #3). It captures keystrokes, encodes them as PTY bytes, and writes them to
//! the PTY. On resize it computes new `(cols, rows)` via [`CellMetrics`] and
//! drives `SessionHandle::resize`.

use std::io::Write;

use eframe::{App, CreationContext};
use egui::{Color32, Event, FontFamily, FontId, Key, Modifiers, Rect, Vec2};
use std::sync::Arc;

use crate::core::session::SessionHandle;
use crate::core::snapshot::{Rgb, Snapshot};
use crate::fonts::{build_fonts, FontSource, BANQUO_MONO, EMBEDDED_IOSEVKA};
use crate::metrics::CellMetrics;

/// The framebuffer clear color: fully transparent (M1 carry-forward).
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// The flat field (Layer 0/1 of §V, collapsed for M2): warm near-black tint.
const FLAT_FIELD: Color32 = Color32::from_rgba_premultiplied(16, 14, 19, 235);

/// Default background (matches the flat field's RGB).
const DEFAULT_BG: Color32 = Color32::from_rgb(16, 14, 19);

/// Cursor color — a visible block.
const CURSOR_COLOR: Color32 = Color32::from_rgba_premultiplied(235, 232, 226, 180);

/// Grid padding in logical pixels.
const GRID_PADDING: f32 = 4.0;

/// The fixed font size for the terminal grid (guarantee #4: font size is a
/// setting, not a function of window size).
const MONO_SIZE: f32 = 16.0;

/// Convert a Banquo [`Rgb`] to an egui [`Color32`].
fn rgb_to_color32(rgb: Rgb) -> Color32 {
    Color32::from_rgb(rgb.r, rgb.g, rgb.b)
}

/// The monospace font for the grid.
fn mono_font() -> FontId {
    FontId::new(MONO_SIZE, FontFamily::Name(BANQUO_MONO.into()))
}

// ---------------------------------------------------------------------------
// Keystroke encoding (T-109)
// ---------------------------------------------------------------------------

/// Encode an egui key event into PTY bytes.
///
/// Pure function — no side effects. Returns `Some(bytes)` for recognized keys,
/// `None` for unhandled keys. Handles printable text, control sequences
/// (Enter, Backspace, Tab, Esc, arrows), and Ctrl-letter combinations.
pub fn encode_key(key: Key, modifiers: Modifiers, text: &str) -> Option<Vec<u8>> {
    // Ctrl-letter combinations
    if modifiers.ctrl && !modifiers.alt {
        if let Some(ch) = key_to_letter(key) {
            let ctrl_byte = (ch.to_ascii_uppercase() as u8)
                .wrapping_sub(b'A')
                .wrapping_add(1);
            return Some(vec![ctrl_byte]);
        }
    }

    // Special keys (no modifier requirement)
    match key {
        Key::Enter => return Some(b"\r".to_vec()),
        Key::Backspace => return Some(vec![0x7f]),
        Key::Tab => return Some(b"\t".to_vec()),
        Key::Escape => return Some(vec![0x1b]),
        Key::ArrowUp => return Some(b"\x1b[A".to_vec()),
        Key::ArrowDown => return Some(b"\x1b[B".to_vec()),
        Key::ArrowRight => return Some(b"\x1b[C".to_vec()),
        Key::ArrowLeft => return Some(b"\x1b[D".to_vec()),
        Key::Home => return Some(b"\x1b[H".to_vec()),
        Key::End => return Some(b"\x1b[F".to_vec()),
        Key::Delete => return Some(b"\x1b[3~".to_vec()),
        Key::PageUp => return Some(b"\x1b[5~".to_vec()),
        Key::PageDown => return Some(b"\x1b[6~".to_vec()),
        _ => {}
    }

    // Printable text (not ctrl-modified)
    if !text.is_empty() && !modifiers.ctrl {
        return Some(text.as_bytes().to_vec());
    }

    None
}

/// Map an egui Key to its letter character (for Ctrl-letter encoding).
fn key_to_letter(key: Key) -> Option<char> {
    match key {
        Key::A => Some('A'),
        Key::B => Some('B'),
        Key::C => Some('C'),
        Key::D => Some('D'),
        Key::E => Some('E'),
        Key::F => Some('F'),
        Key::G => Some('G'),
        Key::H => Some('H'),
        Key::I => Some('I'),
        Key::J => Some('J'),
        Key::K => Some('K'),
        Key::L => Some('L'),
        Key::M => Some('M'),
        Key::N => Some('N'),
        Key::O => Some('O'),
        Key::P => Some('P'),
        Key::Q => Some('Q'),
        Key::R => Some('R'),
        Key::S => Some('S'),
        Key::T => Some('T'),
        Key::U => Some('U'),
        Key::V => Some('V'),
        Key::W => Some('W'),
        Key::X => Some('X'),
        Key::Y => Some('Y'),
        Key::Z => Some('Z'),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// BanquoApp (T-108, T-110)
// ---------------------------------------------------------------------------

/// Banquo's application state for Milestone 2.
pub struct BanquoApp {
    /// Which face backs the monospace family (guarantee #6).
    #[allow(dead_code)] // Exposed for honest reporting; used at startup.
    font_source: FontSource,
    /// The terminal session handle (snapshot reader, PTY writer, resize).
    session: SessionHandle,
    /// Cell metrics derived from the monospace font.
    cell_metrics: Option<CellMetrics>,
    /// Last-sent grid size — only send a resize when this changes.
    last_grid_size: Option<(usize, usize)>,
}

impl BanquoApp {
    /// Construct the app with the session handle and install fonts.
    pub fn new(cc: &CreationContext<'_>, session: SessionHandle) -> Self {
        let (defs, font_source) = build_fonts(EMBEDDED_IOSEVKA);
        cc.egui_ctx.set_fonts(defs);
        eprintln!("banquo: monospace face = {:?}", font_source);
        Self {
            font_source,
            session,
            cell_metrics: None,
            last_grid_size: None,
        }
    }

    /// Which face backs the monospace family (guarantee #6).
    #[allow(dead_code)] // Exposed for honest reporting; not called in M2 render loop.
    pub fn font_source(&self) -> FontSource {
        self.font_source
    }

    /// Lazily compute cell metrics from the egui font system. We do this on
    /// the first frame because font metrics aren't available until after
    /// `set_fonts` + one layout pass.
    fn ensure_metrics(&mut self, ctx: &egui::Context) {
        if self.cell_metrics.is_some() {
            return;
        }

        let font = mono_font();
        ctx.fonts_mut(|fonts| {
            let layout = fonts.layout_no_wrap("M".to_string(), font.clone(), Color32::WHITE);
            if !layout.rows.is_empty() {
                let cell_w = layout.rect.width();
                let cell_h = layout.rows[0].height();
                if cell_w > 0.0 && cell_h > 0.0 {
                    self.cell_metrics = Some(CellMetrics::new(cell_w, cell_h));
                }
            }
        });
    }
}

impl App for BanquoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Ensure we have cell metrics.
        self.ensure_metrics(&ctx);

        let rect = ui.max_rect();
        let painter = ui.painter();

        // Flat field substrate.
        painter.rect_filled(rect, 0.0, FLAT_FIELD);

        // Load the latest snapshot (lock-free, guarantee #2).
        let snapshot: Arc<Snapshot> = self.session.snapshot.load_full();

        if let Some(metrics) = self.cell_metrics {
            // Compute grid size and handle resize (T-110).
            let (cols, rows) = metrics.grid_size(rect.width(), rect.height(), GRID_PADDING);

            if self.last_grid_size != Some((cols, rows)) {
                self.session.resize(cols, rows);
                self.last_grid_size = Some((cols, rows));
            }

            // Compute the centering offset (absorb slack into padding).
            let (offset_x, offset_y) =
                metrics.centering_offset(rect.width(), rect.height(), GRID_PADDING, cols, rows);
            let origin_x = rect.min.x + offset_x;
            let origin_y = rect.min.y + offset_y;

            // Paint each cell (T-108).
            let paint_cols = cols.min(snapshot.cols);
            let paint_rows = rows.min(snapshot.rows);

            for row in 0..paint_rows {
                for col in 0..paint_cols {
                    if let Some(cell) = snapshot.cell(col, row) {
                        let x = origin_x + col as f32 * metrics.cell_w;
                        let y = origin_y + row as f32 * metrics.cell_h;

                        let cell_rect = Rect::from_min_size(
                            egui::pos2(x, y),
                            Vec2::new(metrics.cell_w, metrics.cell_h),
                        );

                        // Background rect (only if non-default to reduce overdraw).
                        let bg = rgb_to_color32(cell.bg);
                        if bg != DEFAULT_BG {
                            painter.rect_filled(cell_rect, 0.0, bg);
                        }

                        // Glyph (skip spaces for performance).
                        if cell.ch != ' ' {
                            let fg = rgb_to_color32(cell.fg);
                            painter.text(
                                egui::pos2(x, y),
                                egui::Align2::LEFT_TOP,
                                cell.ch,
                                mono_font(),
                                fg,
                            );
                        }
                    }
                }
            }

            // Cursor block (T-108).
            if snapshot.cursor_visible
                && snapshot.cursor.col < paint_cols
                && snapshot.cursor.row < paint_rows
            {
                let cx = origin_x + snapshot.cursor.col as f32 * metrics.cell_w;
                let cy = origin_y + snapshot.cursor.row as f32 * metrics.cell_h;
                let cursor_rect = Rect::from_min_size(
                    egui::pos2(cx, cy),
                    Vec2::new(metrics.cell_w, metrics.cell_h),
                );
                painter.rect_filled(cursor_rect, 0.0, CURSOR_COLOR);

                // Paint the character under the cursor in the inverse color.
                if let Some(cell) = snapshot.cell(snapshot.cursor.col, snapshot.cursor.row) {
                    if cell.ch != ' ' {
                        painter.text(
                            egui::pos2(cx, cy),
                            egui::Align2::LEFT_TOP,
                            cell.ch,
                            mono_font(),
                            DEFAULT_BG,
                        );
                    }
                }
            }
        }

        // Process input events (T-109).
        let events: Vec<Event> = ctx.input(|i| i.events.clone());
        for event in &events {
            match event {
                Event::Text(text) => {
                    let bytes = text.as_bytes();
                    let _ = self.session.writer.write_all(bytes);
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Don't double-send printable text that was already handled
                    // by Event::Text. Only encode special/control keys here.
                    let is_special = matches!(
                        key,
                        Key::Enter
                            | Key::Backspace
                            | Key::Tab
                            | Key::Escape
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::ArrowLeft
                            | Key::ArrowRight
                            | Key::Home
                            | Key::End
                            | Key::Delete
                            | Key::PageUp
                            | Key::PageDown
                    );
                    let is_ctrl_letter = modifiers.ctrl && key_to_letter(*key).is_some();

                    if is_special || is_ctrl_letter {
                        if let Some(bytes) = encode_key(*key, *modifiers, "") {
                            let _ = self.session.writer.write_all(&bytes);
                        }
                    }
                }
                _ => {}
            }
        }

        // Keep frames flowing while the terminal is active.
        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        CLEAR_COLOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- T-109 unit tests: keystroke encoding ---

    #[test]
    fn test_encode_printable() {
        assert_eq!(
            encode_key(Key::A, Modifiers::NONE, "a"),
            Some(b"a".to_vec())
        );
        // Multibyte UTF-8
        assert_eq!(
            encode_key(Key::A, Modifiers::NONE, "é"),
            Some("é".as_bytes().to_vec())
        );
    }

    #[test]
    fn test_encode_enter_backspace_tab_esc() {
        assert_eq!(
            encode_key(Key::Enter, Modifiers::NONE, ""),
            Some(b"\r".to_vec())
        );
        assert_eq!(
            encode_key(Key::Backspace, Modifiers::NONE, ""),
            Some(vec![0x7f])
        );
        assert_eq!(
            encode_key(Key::Tab, Modifiers::NONE, ""),
            Some(b"\t".to_vec())
        );
        assert_eq!(
            encode_key(Key::Escape, Modifiers::NONE, ""),
            Some(vec![0x1b])
        );
    }

    #[test]
    fn test_encode_arrows() {
        assert_eq!(
            encode_key(Key::ArrowUp, Modifiers::NONE, ""),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_key(Key::ArrowDown, Modifiers::NONE, ""),
            Some(b"\x1b[B".to_vec())
        );
        assert_eq!(
            encode_key(Key::ArrowRight, Modifiers::NONE, ""),
            Some(b"\x1b[C".to_vec())
        );
        assert_eq!(
            encode_key(Key::ArrowLeft, Modifiers::NONE, ""),
            Some(b"\x1b[D".to_vec())
        );
    }

    #[test]
    fn test_encode_ctrl_c() {
        assert_eq!(
            encode_key(
                Key::C,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                ""
            ),
            Some(vec![0x03])
        );
    }

    #[test]
    fn test_encode_ctrl_letter() {
        // Ctrl-A = 0x01
        assert_eq!(
            encode_key(
                Key::A,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                ""
            ),
            Some(vec![0x01])
        );
        // Ctrl-Z = 0x1a
        assert_eq!(
            encode_key(
                Key::Z,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                ""
            ),
            Some(vec![0x1a])
        );
    }

    #[test]
    fn test_encode_unhandled_none() {
        // F-keys with no text = unhandled
        assert_eq!(encode_key(Key::F1, Modifiers::NONE, ""), None);
    }

    // --- Transparency invariant (carry-forward from M1) ---

    #[test]
    fn test_transparency_invariants() {
        assert_eq!(CLEAR_COLOR, [0.0, 0.0, 0.0, 0.0]);
        assert!(
            FLAT_FIELD.a() >= 230,
            "flat field must be near-opaque (alpha {} of 255)",
            FLAT_FIELD.a()
        );
    }
}
