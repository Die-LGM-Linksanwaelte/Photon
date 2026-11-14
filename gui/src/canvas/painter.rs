//! # Canvas rendering
//!
//! Contains drawing logic for the background grid, widgets, headers, close buttons,
//! selection and drag previews, and interactive scrollbars.

use eframe::egui;
use super::{Canvas, Colors, DragState, PlacedWidget, CELL_SIZE, HEADER_H, SCROLLBAR_THICKNESS};

impl Canvas {
    // ── Main drawing passes ───────────────────────────────────────

    /// Draws the canvas background and placed widget frames.
    pub fn paint_canvas(&self, ui: &egui::Ui, canvas: egui::Rect, c: &Colors, hover_pos: egui::Pos2) {
        let painter = ui.painter_at(canvas);
        let rounding = ui.visuals().window_rounding;
        let font = egui::FontId::proportional(13.0);

        self.draw_background_grid(&painter, canvas, c);

        for widget in &self.widgets {
            self.draw_widget(&painter, widget, canvas, c, hover_pos, &font, rounding);
        }
    }

    /// Draws interaction overlays, previews, and scrollbars above the widgets.
    pub fn paint_overlays(&self, ui: &egui::Ui, canvas: egui::Rect, c: &Colors, hover_pos: egui::Pos2) {
        let painter = ui.painter_at(canvas);
        let rounding = ui.visuals().window_rounding;

        self.draw_selection(&painter, canvas, c, rounding);
        self.draw_move_ghost(&painter, canvas, c, rounding);
        self.draw_resize_ghost(&painter, canvas, c, rounding);
        self.draw_scrollbars(&painter, canvas, c, hover_pos);
    }

    // ── Background and grid ───────────────────────────────────────

    /// Draws the visible background and grid lines using the fixed cell size.
    pub fn draw_background_grid(&self, painter: &egui::Painter, canvas: egui::Rect, c: &Colors) {
        painter.rect_filled(canvas, 0.0, c.bg);
        let origin = self.origin(canvas);

        let start_col = ((canvas.min.x - origin.x) / CELL_SIZE).floor().max(0.0) as usize;
        let end_col = ((canvas.max.x - origin.x) / CELL_SIZE).ceil().max(0.0) as usize;
        let start_row = ((canvas.min.y - origin.y) / CELL_SIZE).floor().max(0.0) as usize;
        let end_row = ((canvas.max.y - origin.y) / CELL_SIZE).ceil().max(0.0) as usize;

        // Draw vertical grid lines.
        for col in start_col..=end_col {
            let x = origin.x + col as f32 * CELL_SIZE;
            if x >= canvas.min.x && x <= canvas.max.x {
                let stroke = if col == 0 {
                    egui::Stroke::new(1.5, c.border)
                } else {
                    egui::Stroke::new(0.5, c.grid)
                };
                painter.line_segment(
                    [egui::pos2(x, canvas.min.y), egui::pos2(x, canvas.max.y)],
                    stroke,
                );
            }
        }

        // Draw horizontal grid lines.
        for row in start_row..=end_row {
            let y = origin.y + row as f32 * CELL_SIZE;
            if y >= canvas.min.y && y <= canvas.max.y {
                let stroke = if row == 0 {
                    egui::Stroke::new(1.5, c.border)
                } else {
                    egui::Stroke::new(0.5, c.grid)
                };
                painter.line_segment(
                    [egui::pos2(canvas.min.x, y), egui::pos2(canvas.max.x, y)],
                    stroke,
                );
            }
        }
    }

    // ── Overlays ─────────────────────────────────────────────────

    /// Draws the active selection rectangle or a pending placement preview.
    pub fn draw_selection(
        &self,
        painter: &egui::Painter,
        canvas: egui::Rect,
        c: &Colors,
        rounding: egui::Rounding,
    ) {
        if let DragState::Selecting { start, current } = self.drag {
            let raw = egui::Rect::from_two_pos(start, current);
            painter.rect_filled(raw, 0.0, c.sel_fill);
            painter.rect_stroke(raw, 0.0, egui::Stroke::new(1.0, c.sel_stroke));
            let (c0, r0) = self.snap(start.min(current), canvas);
            let (c1, r1) = self.snap(start.max(current), canvas);
            let snapped = self.to_rect(c0, r0, (c1 + 1).saturating_sub(c0).max(1), (r1 + 1).saturating_sub(r0).max(1), canvas);
            painter.rect_stroke(snapped, rounding, egui::Stroke::new(2.0, c.ghost_stroke));
        } else if let Some(region) = &self.pending_placement {
            let snapped = self.to_rect(region.col, region.row, region.col_span, region.row_span, canvas);
            painter.rect_filled(snapped, rounding, c.sel_fill);
            painter.rect_stroke(snapped, rounding, egui::Stroke::new(2.0, c.ghost_stroke));
            painter.text(
                snapped.center(),
                egui::Align2::CENTER_CENTER,
                format!("{}×{}", region.col_span, region.row_span),
                egui::FontId::proportional(14.0),
                c.ghost_stroke,
            );
        }
    }

    /// Draws the translucent preview shown while moving a widget.
    pub fn draw_move_ghost(
        &self,
        painter: &egui::Painter,
        canvas: egui::Rect,
        c: &Colors,
        rounding: egui::Rounding,
    ) {
        if let DragState::Moving { id, offset, current } = self.drag {
            if let Some(w) = self.widgets.iter().find(|w| w.id == id) {
                let (nc, nr) = self.snap(current - offset + egui::vec2(CELL_SIZE * 0.5, CELL_SIZE * 0.5), canvas);
                let g = self.to_rect(nc, nr, w.col_span, w.row_span, canvas);
                painter.rect_filled(g, rounding, c.ghost_fill);
                painter.rect_stroke(g, rounding, egui::Stroke::new(2.0, c.ghost_stroke));
            }
        }
    }

    /// Draws the outline and size preview while resizing a widget.
    pub fn draw_resize_ghost(
        &self,
        painter: &egui::Painter,
        canvas: egui::Rect,
        c: &Colors,
        rounding: egui::Rounding,
    ) {
        if let DragState::Resizing { id, resize_right, resize_bottom, current } = self.drag {
            if let Some(w) = self.widgets.iter().find(|w| w.id == id) {
                let (sc, sr) = self.snap(current, canvas);
                let new_cs = if resize_right {
                    ((sc + 1).saturating_sub(w.col)).max(1)
                } else {
                    w.col_span
                };
                let new_rs = if resize_bottom {
                    ((sr + 1).saturating_sub(w.row)).max(1)
                } else {
                    w.row_span
                };
                let g = self.to_rect(w.col, w.row, new_cs, new_rs, canvas);
                painter.rect_filled(g, rounding, c.ghost_fill);
                painter.rect_stroke(g, rounding, egui::Stroke::new(2.5, c.ghost_stroke));
                painter.text(
                    g.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}×{}", new_cs, new_rs),
                    egui::FontId::proportional(14.0),
                    c.ghost_stroke,
                );
            }
        }
    }

    // ── Scrollbars ───────────────────────────────────────────────

    /// Draws interactive scrollbars along the right and bottom edges of the canvas.
    pub fn draw_scrollbars(&self, painter: &egui::Painter, canvas: egui::Rect, c: &Colors, hover_pos: egui::Pos2) {
        let track_bg = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 45);
        let thumb_normal = c.text.linear_multiply(0.35);
        let thumb_hover = c.border_act;

        if let Some(thumb) = self.scrollbar_x_thumb(canvas) {
            let track = self.scrollbar_x_track(canvas);
            painter.rect_filled(track, egui::Rounding::same(3.0), track_bg);
            let is_hovered = thumb.contains(hover_pos) || matches!(self.drag, DragState::DraggingScrollbarX { .. });
            painter.rect_filled(
                thumb.shrink(1.0),
                egui::Rounding::same(3.0),
                if is_hovered { thumb_hover } else { thumb_normal },
            );
        }

        if let Some(thumb) = self.scrollbar_y_thumb(canvas) {
            let track = self.scrollbar_y_track(canvas);
            painter.rect_filled(track, egui::Rounding::same(3.0), track_bg);
            let is_hovered = thumb.contains(hover_pos) || matches!(self.drag, DragState::DraggingScrollbarY { .. });
            painter.rect_filled(
                thumb.shrink(1.0),
                egui::Rounding::same(3.0),
                if is_hovered { thumb_hover } else { thumb_normal },
            );
        }
    }

    // ── Widget rendering ─────────────────────────────────────────

    /// Draws the frame, background, and header of a placed widget.
    pub fn draw_widget(
        &self,
        painter: &egui::Painter,
        widget: &PlacedWidget,
        canvas: egui::Rect,
        c: &Colors,
        hover_pos: egui::Pos2,
        font: &egui::FontId,
        rounding: egui::Rounding,
    ) {
        let wr = self.wrect(widget, canvas);
        if !wr.intersects(canvas) {
            return;
        }
        let is_moving = matches!(self.drag, DragState::Moving { id, .. } if id == widget.id);
        let is_resizing = matches!(self.drag, DragState::Resizing { id, .. } if id == widget.id);
        let hdr = widget.header_rect(wr);
        let close_btn_rect = widget.close_btn_rect(hdr);

        // Draw the widget body.
        painter.rect_filled(wr, rounding, c.widget_bg);
        painter.rect_stroke(
            wr,
            rounding,
            egui::Stroke::new(1.0, if is_resizing { c.border_act } else { c.border }),
        );

        if widget.is_corner_header() {
            self.draw_corner_header(painter, widget, hdr, close_btn_rect, c, hover_pos, is_moving);
        } else {
            self.draw_classic_header(painter, widget, wr, hdr, close_btn_rect, c, hover_pos, font, is_moving, rounding);
        }
    }

    /// Draws the compact 1-by-1 corner header for `TabCompatibility::CanvasOnly` widgets.
    pub fn draw_corner_header(
        &self,
        painter: &egui::Painter,
        widget: &PlacedWidget,
        hdr: egui::Rect,
        close_btn_rect: egui::Rect,
        c: &Colors,
        hover_pos: egui::Pos2,
        is_moving: bool,
    ) {
        // Draw the header in the top-left grid cell.
        painter.rect_filled(
            hdr.shrink(1.0),
            egui::Rounding::same(3.0),
            if is_moving { c.hdr_active } else { c.hdr_bg },
        );
        painter.rect_stroke(
            hdr.shrink(1.0),
            egui::Rounding::same(3.0),
            egui::Stroke::new(1.0, c.border),
        );

        // Draw the title and icon within the header cell.
        painter.text(
            hdr.center_top() + egui::vec2(0.0, 5.0),
            egui::Align2::CENTER_TOP,
            "⠿",
            egui::FontId::proportional(12.0),
            c.text,
        );
        painter.text(
            hdr.center_bottom() + egui::vec2(0.0, -5.0),
            egui::Align2::CENTER_BOTTOM,
            widget.content.title(),
            egui::FontId::proportional(10.0),
            c.text,
        );

        // Draw the close button in the top-right corner.
        self.draw_close_button(painter, close_btn_rect, hover_pos, c, 9.0, 2.0);
    }

    /// Draws the classic horizontal header across the full widget width.
    pub fn draw_classic_header(
        &self,
        painter: &egui::Painter,
        widget: &PlacedWidget,
        wr: egui::Rect,
        hdr: egui::Rect,
        close_btn_rect: egui::Rect,
        c: &Colors,
        hover_pos: egui::Pos2,
        font: &egui::FontId,
        is_moving: bool,
        rounding: egui::Rounding,
    ) {
        painter.rect_filled(
            hdr,
            egui::Rounding { nw: rounding.nw, ne: rounding.ne, sw: 0.0, se: 0.0 },
            if is_moving { c.hdr_active } else { c.hdr_bg },
        );

        painter.text(
            hdr.min + egui::vec2(8.0, 5.0),
            egui::Align2::LEFT_TOP,
            format!("⠿  {}", widget.content.title()),
            font.clone(),
            c.text,
        );

        // Draw the close button.
        self.draw_close_button(painter, close_btn_rect, hover_pos, c, 11.0, 3.0);

        painter.line_segment(
            [egui::pos2(wr.min.x, wr.min.y + HEADER_H), egui::pos2(wr.max.x, wr.min.y + HEADER_H)],
            egui::Stroke::new(1.0, c.border),
        );
    }

    /// Draws the interactive close button with a hover highlight.
    pub fn draw_close_button(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        hover_pos: egui::Pos2,
        c: &Colors,
        font_size: f32,
        corner_rounding: f32,
    ) {
        let is_hovered = rect.contains(hover_pos);
        if is_hovered {
            painter.rect_filled(
                rect,
                egui::Rounding::same(corner_rounding),
                egui::Color32::from_rgba_unmultiplied(220, 50, 50, 200),
            );
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                egui::FontId::proportional(font_size),
                egui::Color32::WHITE,
            );
        } else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                egui::FontId::proportional(font_size),
                c.text.linear_multiply(0.6),
            );
        }
    }

    // ── Widget contents (child UIs) ───────────────────────────────

    /// Renders each placed widget's egui child UI within its content area.
    pub fn render_widget_contents(&mut self, ui: &mut egui::Ui, canvas: egui::Rect) {
        let origin = self.origin(canvas);
        for widget in &mut self.widgets {
            let wr = Self::grid_to_rect(widget.col, widget.row, widget.col_span, widget.row_span, origin);
            if !wr.intersects(canvas) {
                continue;
            }
            let content_rect = widget.content_rect(wr);
            if content_rect.width() > 5.0 && content_rect.height() > 5.0 {
                let mut child = ui.child_ui(
                    content_rect,
                    egui::Layout::top_down(egui::Align::LEFT),
                    None,
                );
                child.set_clip_rect(content_rect.intersect(canvas));
                widget.content.ui(&mut child);
            }
        }
    }

    // ── Scrollbar geometry ────────────────────────────────────────

    /// Returns the track rectangle for the horizontal scrollbar.
    pub fn scrollbar_x_track(&self, canvas: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(canvas.min.x, canvas.max.y - SCROLLBAR_THICKNESS),
            egui::vec2(canvas.width() - SCROLLBAR_THICKNESS, SCROLLBAR_THICKNESS),
        )
    }

    /// Returns the track rectangle for the vertical scrollbar.
    pub fn scrollbar_y_track(&self, canvas: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(canvas.max.x - SCROLLBAR_THICKNESS, canvas.min.y),
            egui::vec2(SCROLLBAR_THICKNESS, canvas.height() - SCROLLBAR_THICKNESS),
        )
    }

    /// Returns the horizontal scrollbar thumb rectangle when scrolling is available.
    pub fn scrollbar_x_thumb(&self, canvas: egui::Rect) -> Option<egui::Rect> {
        let (content_w, _) = self.total_content_size(canvas);
        if content_w <= canvas.width() && self.scroll_offset.x <= 0.0 {
            return None;
        }
        let track = self.scrollbar_x_track(canvas);
        let max_scroll = (content_w - canvas.width()).max(1.0);
        let thumb_w = (canvas.width() / content_w * track.width()).clamp(24.0, track.width());
        let ratio = (self.scroll_offset.x / max_scroll).clamp(0.0, 1.0);
        let x = track.min.x + ratio * (track.width() - thumb_w);
        Some(egui::Rect::from_min_size(egui::pos2(x, track.min.y), egui::vec2(thumb_w, track.height())))
    }

    /// Returns the vertical scrollbar thumb rectangle when scrolling is available.
    pub fn scrollbar_y_thumb(&self, canvas: egui::Rect) -> Option<egui::Rect> {
        let (_, content_h) = self.total_content_size(canvas);
        if content_h <= canvas.height() && self.scroll_offset.y <= 0.0 {
            return None;
        }
        let track = self.scrollbar_y_track(canvas);
        let max_scroll = (content_h - canvas.height()).max(1.0);
        let thumb_h = (canvas.height() / content_h * track.height()).clamp(24.0, track.height());
        let ratio = (self.scroll_offset.y / max_scroll).clamp(0.0, 1.0);
        let y = track.min.y + ratio * (track.height() - thumb_h);
        Some(egui::Rect::from_min_size(egui::pos2(track.min.x, y), egui::vec2(track.width(), thumb_h)))
    }
}
