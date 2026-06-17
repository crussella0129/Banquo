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
use egui::{Color32, Event, FontFamily, FontId, Key, Modifiers, Rect, Vec2, Pos2};
use std::sync::Arc;
use std::time::{Instant, Duration};

use crate::core::session::SessionHandle;
use crate::core::snapshot::{Rgb, Snapshot};
use crate::fonts::{build_fonts, FontSource, BANQUO_MONO};
use crate::metrics::CellMetrics;

/// The framebuffer clear color: fully transparent (M1 carry-forward).
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// The flat field (Layer 0/1 of §V, collapsed for M2): warm near-black tint.
const FLAT_FIELD: Color32 = Color32::from_rgba_premultiplied(16, 14, 19, 180);

/// Default background (matches the flat field's RGB).
const DEFAULT_BG: Color32 = Color32::from_rgb(0, 0, 0);

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
    /// The terminal sessions (one per tab).
    sessions: Vec<SessionHandle>,
    /// The index of the currently active tab.
    active_tab: usize,
    /// Cell metrics derived from the monospace font.
    cell_metrics: Option<CellMetrics>,
    /// Last-sent grid size — only send a resize when this changes.
    last_grid_size: Option<(usize, usize)>,
    /// Whether the app is using native OS decorations (true) or custom chrome (false).
    native_decorations: bool,
    /// Tracking mouse for auto-collapse tabs.
    last_mouse_pos: Option<Pos2>,
    last_mouse_move_time: Instant,
    /// The loaded configuration.
    config: crate::config::BanquoConfig,
}

impl BanquoApp {
    /// Construct the app with the session handle and install fonts.
    pub fn new(cc: &CreationContext<'_>, session: SessionHandle, native_decorations: bool, config: crate::config::BanquoConfig) -> Self {
        let (defs, font_source) = build_fonts(&config);
        cc.egui_ctx.set_fonts(defs);
        eprintln!("banquo: monospace face = {:?}", font_source);
        Self {
            font_source,
            sessions: vec![session],
            active_tab: 0,
            cell_metrics: None,
            last_grid_size: None,
            native_decorations,
            last_mouse_pos: None,
            last_mouse_move_time: Instant::now(),
            config,
        }
    }

    /// Which face backs the monospace family (guarantee #6).
    #[allow(dead_code)] // Exposed for honest reporting; not called in M2 render loop.
    pub fn font_source(&self) -> FontSource {
        self.font_source.clone()
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

    fn get_squircle_path(rect: egui::Rect, radius: f32, corner_style: &str, top_only: bool) -> Vec<egui::Pos2> {
        let n = if corner_style == "g2" { 3.5 } else { 5.0 };
        let steps = 16;
        let mut points = Vec::with_capacity(steps * 4);

        let max_h = if top_only { rect.height() } else { rect.height() / 2.0 };
        let r = radius.min(rect.width() / 2.0).min(max_h);
        
        let mut quadrant = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = (i as f32 / steps as f32) * std::f32::consts::FRAC_PI_2;
            let x = t.cos().max(0.0).powf(2.0 / n);
            let y = t.sin().max(0.0).powf(2.0 / n);
            quadrant.push(egui::vec2(x, y));
        }

        // Top-Left
        for v in &quadrant {
            points.push(egui::pos2(rect.min.x + r - r * v.x, rect.min.y + r - r * v.y));
        }

        // Top-Right
        for v in &quadrant {
            points.push(egui::pos2(rect.max.x - r + r * v.y, rect.min.y + r - r * v.x));
        }

        if top_only {
            points.push(egui::pos2(rect.max.x, rect.max.y));
            points.push(egui::pos2(rect.min.x, rect.max.y));
        } else {
            // Bottom-Right
            for v in &quadrant {
                points.push(egui::pos2(rect.max.x - r + r * v.x, rect.max.y - r + r * v.y));
            }

            // Bottom-Left
            for v in &quadrant {
                points.push(egui::pos2(rect.min.x + r - r * v.y, rect.max.y - r + r * v.x));
            }
        }
        points
    }
}
impl App for BanquoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Ensure we have cell metrics.
        self.ensure_metrics(&ctx);

        let window_cfg = &self.config.window;
        let edge_style = window_cfg.edge_style.as_deref().unwrap_or("flat");
        let corner_style = window_cfg.corner_style.as_deref().unwrap_or("square");
        let radius = window_cfg.radius.unwrap_or(8.0);
        let inset = window_cfg.inset.unwrap_or(0.0);

        let mut rect = ui.max_rect();
        let painter = ui.painter();
        
        // Background clear (fully transparent)
        painter.rect_filled(rect, 0.0, Color32::TRANSPARENT);

        // Inset the drawing rect
        rect = rect.shrink(inset);

        let mut content_rect = rect;
        // Pad the content area so terminal grid doesn't collide with the rounded borders
        let corner_padding = (radius / 2.0).max(8.0);
        content_rect = content_rect.shrink(corner_padding);

        // Draw shape with slight inset to hide anti-aliasing halo under the stroke
        let bg_rect = rect.shrink(0.5);
        let bg_radius = (radius - 0.5).max(0.0);
        
        if corner_style == "square" || radius <= 0.0 {
            painter.rect_filled(bg_rect, 0.0, FLAT_FIELD);
        } else if corner_style == "g1" {
            let rounding = egui::CornerRadius::same(bg_radius as u8);
            painter.rect_filled(bg_rect, rounding, FLAT_FIELD);
        } else {
            // G2 or G3: Superellipse corners using helper
            let points = Self::get_squircle_path(bg_rect, bg_radius, corner_style, false);
            let shape = egui::epaint::PathShape::convex_polygon(points, FLAT_FIELD, egui::Stroke::NONE);
            painter.add(shape);
        }

        // Apply Edge Styles
        let rounding = egui::CornerRadius::same(if corner_style == "square" { 0 } else { radius as u8 });
        let stroke_kind = egui::StrokeKind::Inside;
        
        let mut stroke_rect = |draw_rect: Rect, stroke: egui::Stroke| {
            if corner_style == "square" || radius <= 0.0 || corner_style == "g1" {
                painter.rect_stroke(draw_rect, rounding, stroke, stroke_kind);
            } else {
                // Emulate StrokeKind::Inside by shrinking by half the stroke width
                let path_rect = draw_rect.shrink(stroke.width / 2.0);
                // The radius needs to shrink by the same amount so the curve stays parallel
                let path_radius = (radius - (stroke.width / 2.0)).max(0.0);
                let points = Self::get_squircle_path(path_rect, path_radius, corner_style, false);
                let shape = egui::epaint::PathShape {
                    points,
                    closed: true,
                    fill: Color32::TRANSPARENT,
                    stroke: stroke.into(),
                };
                painter.add(shape);
            }
        };

        if edge_style == "rounded" {
            stroke_rect(rect, egui::Stroke::new(1.0, Color32::from_white_alpha(40)));
        } else if edge_style == "beveled" {
            // Top and left highlight
            let inset_rect = rect.shrink(1.0);
            stroke_rect(rect, egui::Stroke::new(1.0, Color32::from_white_alpha(60)));
            stroke_rect(inset_rect, egui::Stroke::new(1.0, Color32::from_black_alpha(150)));
        } else if edge_style == "3d" {
            // Chunky CRT bezel effect
            stroke_rect(rect, egui::Stroke::new(2.0, Color32::from_white_alpha(30)));
            stroke_rect(rect.shrink(2.0), egui::Stroke::new(4.0, Color32::from_black_alpha(120)));
            stroke_rect(rect.shrink(6.0), egui::Stroke::new(1.0, Color32::from_white_alpha(20)));
        }

        // --- IDLE DETECTION ---
        let hover_pos = ctx.input(|i| i.pointer.hover_pos());
        if let Some(pos) = hover_pos {
            if self.last_mouse_pos != Some(pos) {
                self.last_mouse_move_time = Instant::now();
                self.last_mouse_pos = Some(pos);
            }
        }
        
        let time_since_move = Instant::now().duration_since(self.last_mouse_move_time);
        let pointer_y = hover_pos.map(|p| p.y).unwrap_or(f32::MAX);
        
        // Show tabs if mouse is within top 40px AND moved within the last 3 seconds
        let show_tabs = pointer_y <= 40.0 && time_since_move < Duration::from_secs(3);

        // --- OVERLAY DRAWING ---
        if show_tabs {
            let title_bar_height = 32.0;
            let title_bar_rect = Rect::from_min_size(
                rect.min,
                Vec2::new(rect.width(), title_bar_height),
            );

            // Draw over the grid using an egui::Area
            egui::Area::new("tab_bar_overlay".into())
                .fixed_pos(rect.min)
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    // Paint background for tabs area with more transparency
                    let bg_color = Color32::from_rgba_unmultiplied(30, 28, 35, 160);
                    
                    // Shrink slightly to hide AA halo under the stroke
                    let tab_bg_rect = title_bar_rect.shrink(0.5);
                    let tab_bg_radius = (radius - 0.5).max(0.0);

                    if corner_style == "square" || radius <= 0.0 {
                        ui.painter().rect_filled(tab_bg_rect, 0.0, bg_color);
                    } else if corner_style == "g1" {
                        // For G1, use custom rounding (top corners only)
                        let rounding = egui::CornerRadius {
                            nw: tab_bg_radius as u8,
                            ne: tab_bg_radius as u8,
                            sw: 0,
                            se: 0,
                        };
                        ui.painter().rect_filled(tab_bg_rect, rounding, bg_color);
                    } else {
                        // For G2/G3, use helper with top_only=true
                        let points = Self::get_squircle_path(tab_bg_rect, tab_bg_radius, corner_style, true);
                        let shape = egui::epaint::PathShape::convex_polygon(points, bg_color, egui::Stroke::NONE);
                        ui.painter().add(shape);
                    }

                    if !self.native_decorations {
                        // Drag to move
                        let title_bar_response = ui.interact(title_bar_rect, ui.id().with("title_bar"), egui::Sense::click_and_drag());
                        if title_bar_response.dragged() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }

                        // Close button (top right)
                        let close_button_size = 32.0;
                        let close_rect = Rect::from_min_size(
                            egui::pos2(rect.max.x - close_button_size, rect.min.y),
                            Vec2::new(close_button_size, close_button_size),
                        );
                        
                        let close_response = ui.interact(close_rect, ui.id().with("close_btn"), egui::Sense::click());
                        
                        // Draw close button (a simple X)
                        let cross_color = if close_response.hovered() {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(128)
                        };
                        
                        let stroke = egui::Stroke::new(1.5, cross_color);
                        let p1 = close_rect.center() - Vec2::new(4.0, 4.0);
                        let p2 = close_rect.center() + Vec2::new(4.0, 4.0);
                        ui.painter().line_segment([p1, p2], stroke);
                        let p3 = close_rect.center() - Vec2::new(4.0, -4.0);
                        let p4 = close_rect.center() + Vec2::new(4.0, -4.0);
                        ui.painter().line_segment([p3, p4], stroke);

                        if close_response.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }

                    // Render Tabs
                    let mut tab_x = rect.min.x + 8.0;
                    let tab_y = rect.min.y + 6.0;
                    let tab_h = 24.0;
                    
                    for i in 0..self.sessions.len() {
                        let tab_w = 80.0;
                        let tab_rect = Rect::from_min_size(
                            egui::pos2(tab_x, tab_y),
                            Vec2::new(tab_w, tab_h)
                        );
                        
                        let tab_resp = ui.interact(tab_rect, ui.id().with(format!("tab_{}", i)), egui::Sense::click());
                        if tab_resp.clicked() {
                            self.active_tab = i;
                        }
                        
                        let bg_color = if i == self.active_tab {
                            Color32::from_rgba_premultiplied(80, 80, 90, 255)
                        } else if tab_resp.hovered() {
                            Color32::from_rgba_premultiplied(50, 50, 60, 255)
                        } else {
                            Color32::from_rgba_unmultiplied(40, 40, 65, 90)
                        };
                        
                        ui.painter().rect_filled(tab_rect, 4.0, bg_color);
                        
                        ui.painter().text(
                            tab_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Tab {}", i + 1),
                            FontId::new(14.0, FontFamily::Name(crate::fonts::BANQUO_MONO.into())),
                            if i == self.active_tab { Color32::WHITE } else { Color32::GRAY },
                        );
                        
                        tab_x += tab_w + 4.0;
                    }
                    
                    // Add Tab Button
                    let add_rect = Rect::from_min_size(
                        egui::pos2(tab_x, tab_y),
                        Vec2::new(24.0, tab_h)
                    );
                    let add_resp = ui.interact(add_rect, ui.id().with("add_tab_btn"), egui::Sense::click());
                    if add_resp.clicked() {
                        if let Some((cols, rows)) = self.last_grid_size {
                            if let Ok(new_session) = crate::core::session::spawn(cols, rows) {
                                self.sessions.push(new_session);
                                self.active_tab = self.sessions.len() - 1;
                            }
                        }
                    }
                    
                    ui.painter().text(
                        add_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        FontId::new(16.0, FontFamily::Name(crate::fonts::BANQUO_MONO.into())),
                        if add_resp.hovered() { Color32::WHITE } else { Color32::GRAY },
                    );
                });
        }

        // Resize borders (invisible edges) MUST ALWAYS be active, even when tabs are hidden, 
        // so you can resize the window at any time.
        if !self.native_decorations {
            let border = 6.0;
            let edges: [(egui::Align2, Vec2, egui::CursorIcon, egui::ViewportCommand); 4] = [
                (egui::Align2::LEFT_TOP, Vec2::new(border, rect.height()), egui::CursorIcon::ResizeHorizontal, egui::ViewportCommand::BeginResize(egui::ResizeDirection::West)),
                (egui::Align2::RIGHT_TOP, Vec2::new(border, rect.height()), egui::CursorIcon::ResizeHorizontal, egui::ViewportCommand::BeginResize(egui::ResizeDirection::East)),
                (egui::Align2::LEFT_TOP, Vec2::new(rect.width(), border), egui::CursorIcon::ResizeVertical, egui::ViewportCommand::BeginResize(egui::ResizeDirection::North)),
                (egui::Align2::LEFT_BOTTOM, Vec2::new(rect.width(), border), egui::CursorIcon::ResizeVertical, egui::ViewportCommand::BeginResize(egui::ResizeDirection::South)),
            ];

            for (align, size, cursor, cmd) in edges {
                let edge_rect = align.align_size_within_rect(size, rect);
                let edge_id = ui.id().with(format!("edge_{:?}", cmd));
                let edge_response = ui.interact(edge_rect, edge_id, egui::Sense::click_and_drag());
                if edge_response.hovered() || edge_response.dragged() {
                    ctx.set_cursor_icon(cursor);
                }
                if edge_response.dragged() {
                    ctx.send_viewport_cmd(cmd);
                }
            }
        }

        // Load the latest snapshot (lock-free, guarantee #2).
        let snapshot: Arc<Snapshot> = self.sessions[self.active_tab].snapshot.load_full();

        if let Some(metrics) = self.cell_metrics {
            // Compute grid size and handle resize (T-110).
            let (cols, rows) = metrics.grid_size(content_rect.width(), content_rect.height(), GRID_PADDING);

            if self.last_grid_size != Some((cols, rows)) {
                for session in &mut self.sessions {
                    session.resize(cols, rows);
                }
                self.last_grid_size = Some((cols, rows));
            }

            // Compute the centering offset (absorb slack into padding).
            let (offset_x, offset_y) =
                metrics.centering_offset(content_rect.width(), content_rect.height(), GRID_PADDING, cols, rows);
            let origin_x = content_rect.min.x + offset_x;
            let origin_y = content_rect.min.y + offset_y;

            // Paint each cell (T-108).
            let paint_cols = cols.min(snapshot.cols);
            let paint_rows = rows.min(snapshot.rows);

            let is_auto = self.config.grid.mode.as_deref() == Some("auto");

            for row in 0..paint_rows {
                let mut current_x = origin_x;
                for col in 0..paint_cols {
                    if let Some(cell) = snapshot.cell(col, row) {
                        let cell_w = if is_auto {
                            let galley = painter.layout_no_wrap(cell.ch.to_string(), mono_font(), Color32::WHITE);
                            galley.rect.width().max(1.0)
                        } else {
                            metrics.cell_w
                        };
                        let x = current_x;
                        let y = origin_y + row as f32 * metrics.cell_h;

                        let cell_rect = Rect::from_min_size(
                            egui::pos2(x, y),
                            Vec2::new(cell_w, metrics.cell_h),
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
                        
                        current_x += cell_w;
                    }
                }
            }

            // Apply window blur once if enabled
            crate::os::apply_window_effects(&self.config, _frame);

            // Cursor block (T-108).
            if snapshot.cursor_visible
                && snapshot.cursor.col < paint_cols
                && snapshot.cursor.row < paint_rows
            {
                let mut cx = origin_x;
                if is_auto {
                    for col in 0..snapshot.cursor.col {
                        let ch = snapshot.cell(col, snapshot.cursor.row).map(|c| c.ch).unwrap_or(' ');
                        let galley = painter.layout_no_wrap(ch.to_string(), mono_font(), Color32::WHITE);
                        cx += galley.rect.width().max(1.0);
                    }
                } else {
                    cx += snapshot.cursor.col as f32 * metrics.cell_w;
                }
                
                let cy = origin_y + snapshot.cursor.row as f32 * metrics.cell_h;
                
                let cursor_ch = snapshot.cell(snapshot.cursor.col, snapshot.cursor.row).map(|c| c.ch).unwrap_or(' ');
                let cursor_w = if is_auto {
                    let galley = painter.layout_no_wrap(cursor_ch.to_string(), mono_font(), Color32::WHITE);
                    galley.rect.width().max(4.0) // minimum width for cursor on spaces
                } else {
                    metrics.cell_w
                };
                
                let cursor_rect = Rect::from_min_size(
                    egui::pos2(cx, cy),
                    Vec2::new(cursor_w, metrics.cell_h),
                );
                painter.rect_filled(cursor_rect, 0.0, CURSOR_COLOR);

                // Paint the character under the cursor in the inverse color.
                if cursor_ch != ' ' {
                    painter.text(
                        egui::pos2(cx, cy),
                        egui::Align2::LEFT_TOP,
                        cursor_ch,
                        mono_font(),
                        DEFAULT_BG,
                    );
                }
            }
        }

        // Process input events (T-109).
        let events: Vec<Event> = ctx.input(|i| i.events.clone());
        for event in &events {
            match event {
                Event::Text(text) => {
                    let bytes = text.as_bytes();
                    if let Ok(mut writer) = self.sessions[self.active_tab].writer.lock() {
                        let _ = writer.write_all(bytes);
                        let _ = writer.flush();
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Check for tab management shortcuts
                    if modifiers.ctrl && modifiers.shift {
                        if *key == Key::T {
                            // Spawn new tab
                            if let Some((cols, rows)) = self.last_grid_size {
                                if let Ok(new_session) = crate::core::session::spawn(cols, rows) {
                                    self.sessions.push(new_session);
                                    self.active_tab = self.sessions.len() - 1;
                                }
                            }
                            continue;
                        } else if *key == Key::W {
                            // Close current tab
                            self.sessions.remove(self.active_tab);
                            if self.sessions.is_empty() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                return;
                            }
                            if self.active_tab >= self.sessions.len() {
                                self.active_tab = self.sessions.len() - 1;
                            }
                            continue;
                        }
                    }

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
                            let mut writer = self.sessions[self.active_tab].writer.lock().unwrap();
                            let _ = writer.write_all(&bytes);
                            let _ = writer.flush();
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
