//! # Canvas
//!
//! A freely arrangeable canvas with a fixed grid cell size. Colors are derived from
//! `ui.visuals()`, so dark, light, and custom egui themes are supported. The grid
//! uses cells of `CELL_SIZE` points and expands as the available area grows.
//! Widgets with `TabCompatibility::CanvasOnly` use the top-left cell as their header,
//! keeping their content aligned with the grid.

pub mod types;
pub mod layout;
pub mod input;
pub mod painter;

// Re-export public types from the canvas submodules.
pub use types::{GridRect, PlacedWidget, DragState, Colors, HEADER_H, SCROLLBAR_THICKNESS};
pub use layout::{PageSlot, CanvasLayout, MaLayout};

use eframe::egui;
use crate::panels::view_bar::CELL_SIZE;

const RESIZE_ZONE: f32 = 10.0;

/// Stores placed widgets and manages scrolling, active pointer operations,
/// and pending widget placements.
pub struct Canvas {
    /// Widgets currently placed on the canvas.
    pub widgets:           Vec<PlacedWidget>,
    /// Number of grid columns currently represented by the canvas.
    pub cols:              usize,
    /// Number of grid rows currently represented by the canvas.
    pub rows:              usize,
    /// Current horizontal and vertical scroll offset in points.
    pub scroll_offset:     egui::Vec2,
    /// Grid region awaiting confirmation before a widget is placed.
    pub pending_placement: Option<GridRect>,
    pub(crate) drag:       DragState,
    next_id:               u32,
}

impl Canvas {
    /// Creates an empty canvas with its initial grid dimensions.
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            cols: 20,
            rows: 12,
            scroll_offset: egui::Vec2::ZERO,
            pending_placement: None,
            drag: DragState::Idle,
            next_id: 0,
        }
    }

    /// Places a widget in the requested grid region if that region is unoccupied.
    pub fn place_widget(&mut self, content: crate::panels::Tab, region: GridRect) {
        let occupied = self.widgets.iter()
            .any(|w| w.overlaps(region.col, region.row, region.col_span, region.row_span));
        if !occupied {
            self.widgets.push(PlacedWidget {
                id: self.next_id,
                content,
                col: region.col,
                row: region.row,
                col_span: region.col_span,
                row_span: region.row_span,
            });
            self.next_id += 1;
        }
    }

    /// Places a widget at the first unoccupied region matching the requested span.
    ///
    /// Returns `true` if a free region was found and the widget was placed.
    pub fn place_widget_auto(&mut self, content: crate::panels::Tab, col_span: usize, row_span: usize) -> bool {
        let cs = col_span.max(1);
        let rs = row_span.max(1);
        let max_search_cols = (self.cols + 10).max(30);
        let max_search_rows = (self.rows + 10).max(20);
        for r in 0..max_search_rows {
            for c in 0..max_search_cols {
                if !self.widgets.iter().any(|w| w.overlaps(c, r, cs, rs)) {
                    self.place_widget(content, GridRect { col: c, row: r, col_span: cs, row_span: rs });
                    return true;
                }
            }
        }
        false
    }

    // ── Fixed-cell geometry and scroll offset ────────────────────────

    /// Returns the grid origin after applying the current scroll offset.
    pub fn origin(&self, canvas: egui::Rect) -> egui::Pos2 {
        canvas.min - self.scroll_offset
    }

    /// Converts a screen position to non-negative grid coordinates.
    pub fn snap(&self, pos: egui::Pos2, canvas: egui::Rect) -> (usize, usize) {
        let rel = pos - self.origin(canvas);
        let col = ((rel.x / CELL_SIZE).floor() as isize).max(0) as usize;
        let row = ((rel.y / CELL_SIZE).floor() as isize).max(0) as usize;
        (col, row)
    }

    /// Converts a grid region into a screen-space rectangle.
    pub fn grid_to_rect(col: usize, row: usize, cs: usize, rs: usize, origin: egui::Pos2) -> egui::Rect {
        let min = origin + egui::vec2(col as f32 * CELL_SIZE, row as f32 * CELL_SIZE);
        egui::Rect::from_min_size(min, egui::vec2(cs as f32 * CELL_SIZE, rs as f32 * CELL_SIZE))
    }

    /// Converts a grid region into a screen-space rectangle using this canvas origin.
    pub fn to_rect(&self, col: usize, row: usize, cs: usize, rs: usize, canvas: egui::Rect) -> egui::Rect {
        Self::grid_to_rect(col, row, cs, rs, self.origin(canvas))
    }

    /// Returns the screen-space rectangle occupied by a placed widget.
    pub fn wrect(&self, w: &PlacedWidget, canvas: egui::Rect) -> egui::Rect {
        self.to_rect(w.col, w.row, w.col_span, w.row_span, canvas)
    }

    /// Returns the total scrollable content size in points.
    pub fn total_content_size(&self, canvas: egui::Rect) -> (f32, f32) {
        let max_widget_col = self.widgets.iter().map(|w| w.col + w.col_span).max().unwrap_or(0);
        let max_widget_row = self.widgets.iter().map(|w| w.row + w.row_span).max().unwrap_or(0);
        let content_w = ((max_widget_col + 4) as f32 * CELL_SIZE)
            .max(self.scroll_offset.x + canvas.width());
        let content_h = ((max_widget_row + 4) as f32 * CELL_SIZE)
            .max(self.scroll_offset.y + canvas.height());
        (content_w, content_h)
    }

    // ── Main UI ───────────────────────────────────────────────────

    /// Handles input and renders the canvas, widgets, and interaction overlays.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let canvas = ui.available_rect_before_wrap();
        let visible_cols = ((canvas.width() / CELL_SIZE).ceil() as usize).max(1);
        let visible_rows = ((canvas.height() / CELL_SIZE).ceil() as usize).max(1);

        let max_widget_col = self.widgets.iter().map(|w| w.col + w.col_span).max().unwrap_or(0);
        let max_widget_row = self.widgets.iter().map(|w| w.row + w.row_span).max().unwrap_or(0);

        let scrolled_cols = (((self.scroll_offset.x + canvas.width()) / CELL_SIZE).ceil() as usize).max(1);
        let scrolled_rows = (((self.scroll_offset.y + canvas.height()) / CELL_SIZE).ceil() as usize).max(1);

        self.cols = max_widget_col.max(scrolled_cols).max(visible_cols);
        self.rows = max_widget_row.max(scrolled_rows).max(visible_rows);

        let c = Colors::from_ui(ui);

        // 1. Handle pointer events, scrolling, and panning.
        self.handle_pointer_events(ui, canvas);

        // 2. Draw the canvas background and widget frames.
        let hover_pos = ui.ctx().input(|i| i.pointer.latest_pos()).unwrap_or(egui::Pos2::ZERO);
        self.paint_canvas(ui, canvas, &c, hover_pos);

        // 3. Render widget contents in child UIs.
        self.render_widget_contents(ui, canvas);

        // 4. Draw overlays and previews above the widgets.
        self.paint_overlays(ui, canvas, &c, hover_pos);

        if !matches!(self.drag, DragState::Idle) {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_initial_origin() {
        let canvas = Canvas::new();
        assert_eq!(canvas.scroll_offset, egui::Vec2::ZERO);

        let viewport = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(800.0, 600.0));
        assert_eq!(canvas.origin(viewport), egui::pos2(100.0, 50.0));

        let (c, r) = canvas.snap(egui::pos2(100.0, 50.0), viewport);
        assert_eq!((c, r), (0, 0));
    }

    #[test]
    fn test_canvas_scroll_offset_and_snap() {
        let mut canvas = Canvas::new();
        let viewport = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));

        // Scroll by 2 cells horizontally, 3 cells vertically
        canvas.scroll_offset = egui::vec2(2.0 * CELL_SIZE, 3.0 * CELL_SIZE);
        assert_eq!(canvas.origin(viewport), egui::pos2(-2.0 * CELL_SIZE, -3.0 * CELL_SIZE));

        // Top-left corner of the screen is now at grid (2, 3)
        let (c, r) = canvas.snap(egui::pos2(0.0, 0.0), viewport);
        assert_eq!((c, r), (2, 3));

        // Arbitrary position far down-right
        let (c, r) = canvas.snap(egui::pos2(480.0, 480.0), viewport);
        assert_eq!((c, r), (12, 13));
    }

    #[test]
    fn test_canvas_clamp_negative() {
        let canvas = Canvas::new();
        let viewport = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(800.0, 600.0));

        // Positions above or to the left of the origin clamp to 0
        let (c, r) = canvas.snap(egui::pos2(50.0, 50.0), viewport);
        assert_eq!((c, r), (0, 0));
    }
}
