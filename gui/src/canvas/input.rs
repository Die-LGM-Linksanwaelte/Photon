//! # Canvas input handling
//!
//! This module implements pointer press, move, and release handling, scrolling,
//! panning, scrollbar dragging, and cursor updates for `Canvas`.

use eframe::egui;
use super::{Canvas, DragState, CELL_SIZE, RESIZE_ZONE};

impl Canvas {
    /// Processes pointer interactions and updates the cursor icon.
    pub(super) fn handle_pointer_events(&mut self, ui: &egui::Ui, canvas: egui::Rect) {
        let (pressed, released, pos, scroll_delta, middle_pressed, secondary_pressed) = ui.ctx().input(|i| (
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.latest_pos(),
            i.smooth_scroll_delta,
            i.pointer.button_pressed(egui::PointerButton::Middle),
            i.pointer.button_pressed(egui::PointerButton::Secondary),
        ));

        let popup_open = self.pending_placement.is_some();
        let pointer_captured = ui.ctx().wants_pointer_input() || ui.ctx().is_pointer_over_area();

        // 1. Handle wheel scrolling, clamping the offset at the top and left edges.
        if let Some(p) = pos {
            if canvas.contains(p) && scroll_delta != egui::Vec2::ZERO && !popup_open {
                let mut new_offset = self.scroll_offset - scroll_delta;
                new_offset.x = new_offset.x.max(0.0);
                new_offset.y = new_offset.y.max(0.0);
                self.scroll_offset = new_offset;
                ui.ctx().request_repaint();
            }
        }

        // 2. Start panning with the middle button or a secondary click on empty space.
        if (middle_pressed || secondary_pressed) && !popup_open && !pointer_captured {
            if let Some(p) = pos {
                if canvas.contains(p) {
                    let in_widget = self.widgets.iter().any(|w| self.wrect(w, canvas).contains(p));
                    if middle_pressed || !in_widget {
                        self.drag = DragState::Panning {
                            start_pos: p,
                            start_offset: self.scroll_offset,
                        };
                    }
                }
            }
        }

        // 3. Handle primary clicks on scrollbars, widgets, or empty canvas space.
        if pressed && !popup_open {
            if let Some(p) = pos {
                if canvas.contains(p) {
                    let over_resize_zone = self.widgets.iter().any(|w| {
                        Self::resize_hit(p, self.wrect(w, canvas)).is_some()
                    });

                    if !pointer_captured || over_resize_zone {
                        self.handle_pointer_press(p, canvas);
                    }
                }
            }
        }

        // 4. Update the active interaction as the pointer moves.
        if let Some(p) = pos {
            self.handle_pointer_move(p, canvas);
        }

        // 5. Finish interactions when a pointer button is released.
        let middle_released = ui.ctx().input(|i| i.pointer.button_released(egui::PointerButton::Middle));
        let secondary_released = ui.ctx().input(|i| i.pointer.button_released(egui::PointerButton::Secondary));
        if (middle_released || secondary_released) && matches!(self.drag, DragState::Panning { .. }) {
            self.drag = DragState::Idle;
        }

        if released {
            if !popup_open {
                self.handle_pointer_release(canvas);
            }
            self.drag = DragState::Idle;
        }

        self.update_cursor(ui, canvas, pos);
    }

    /// Handles a primary-button press within the canvas.
    fn handle_pointer_press(&mut self, p: egui::Pos2, canvas: egui::Rect) {
        // 0. Check whether a scrollbar thumb was hit.
        if let Some(thumb) = self.scrollbar_x_thumb(canvas) {
            if thumb.contains(p) {
                self.drag = DragState::DraggingScrollbarX {
                    start_x: p.x,
                    start_offset_x: self.scroll_offset.x,
                };
                return;
            }
        }
        if let Some(thumb) = self.scrollbar_y_thumb(canvas) {
            if thumb.contains(p) {
                self.drag = DragState::DraggingScrollbarY {
                    start_y: p.y,
                    start_offset_y: self.scroll_offset.y,
                };
                return;
            }
        }

        // 1. Check whether a widget close button was clicked.
        let mut close_id = None;
        for w in &self.widgets {
            let wr = self.wrect(w, canvas);
            let hdr = w.header_rect(wr);
            let close_btn_rect = w.close_btn_rect(hdr);
            if close_btn_rect.contains(p) {
                close_id = Some(w.id);
                break;
            }
        }

        if let Some(id) = close_id {
            self.widgets.retain(|w| w.id != id);
            return;
        }

        // 2. Check whether a widget resize edge was hit.
        for w in &self.widgets {
            let wr = self.wrect(w, canvas);
            if let Some((rr, rb)) = Self::resize_hit(p, wr) {
                self.drag = DragState::Resizing {
                    id: w.id,
                    resize_right: rr,
                    resize_bottom: rb,
                    current: p,
                };
                return;
            }
        }

        // 3. Start moving a widget if its header was hit.
        for w in &self.widgets {
            let wr = self.wrect(w, canvas);
            let hdr = w.header_rect(wr);
            if hdr.contains(p) {
                self.drag = DragState::Moving {
                    id: w.id,
                    offset: p - wr.min,
                    current: p,
                };
                return;
            }
        }

        // 4. Start a region selection when clicking empty canvas space.
        let in_widget = self.widgets.iter().any(|w| self.wrect(w, canvas).contains(p));
        if !in_widget {
            self.drag = DragState::Selecting { start: p, current: p };
        }
    }

    /// Updates the pointer position for the active drag operation.
    fn handle_pointer_move(&mut self, p: egui::Pos2, canvas: egui::Rect) {
        match self.drag {
            DragState::Selecting { start, .. } => {
                self.drag = DragState::Selecting { start, current: p };
            }
            DragState::Moving { id, offset, .. } => {
                self.drag = DragState::Moving { id, offset, current: p };
            }
            DragState::Resizing { id, resize_right, resize_bottom, .. } => {
                self.drag = DragState::Resizing { id, resize_right, resize_bottom, current: p };
            }
            DragState::Panning { start_pos, start_offset } => {
                let delta = p - start_pos;
                let mut new_offset = start_offset - delta;
                new_offset.x = new_offset.x.max(0.0);
                new_offset.y = new_offset.y.max(0.0);
                self.scroll_offset = new_offset;
            }
            DragState::DraggingScrollbarX { start_x, start_offset_x } => {
                let (content_w, _) = self.total_content_size(canvas);
                let track = self.scrollbar_x_track(canvas);
                let max_scroll = (content_w - canvas.width()).max(1.0);
                let thumb_w = (canvas.width() / content_w * track.width()).clamp(24.0, track.width());
                let track_len = (track.width() - thumb_w).max(1.0);
                let dx = p.x - start_x;
                self.scroll_offset.x = (start_offset_x + (dx / track_len) * max_scroll).max(0.0);
            }
            DragState::DraggingScrollbarY { start_y, start_offset_y } => {
                let (_, content_h) = self.total_content_size(canvas);
                let track = self.scrollbar_y_track(canvas);
                let max_scroll = (content_h - canvas.height()).max(1.0);
                let thumb_h = (canvas.height() / content_h * track.height()).clamp(24.0, track.height());
                let track_len = (track.height() - thumb_h).max(1.0);
                let dy = p.y - start_y;
                self.scroll_offset.y = (start_offset_y + (dy / track_len) * max_scroll).max(0.0);
            }
            _ => {}
        }
    }

    /// Finishes an interaction on release and applies the resulting changes.
    fn handle_pointer_release(&mut self, canvas: egui::Rect) {
        match self.drag {
            DragState::Selecting { start, current } => {
                let tl = start.min(current);
                let br = start.max(current);
                let (c0, r0) = self.snap(tl, canvas);
                let (c1, r1) = self.snap(br, canvas);
                let cs = ((c1 + 1).saturating_sub(c0)).max(1);
                let rs = ((r1 + 1).saturating_sub(r0)).max(1);
                if cs >= 1 && rs >= 1 {
                    self.pending_placement = Some(super::GridRect { col: c0, row: r0, col_span: cs, row_span: rs });
                }
            }
            DragState::Moving { id, offset, current } => {
                let new_tl = current - offset;
                let (nc, nr) = self.snap(new_tl + egui::vec2(CELL_SIZE * 0.5, CELL_SIZE * 0.5), canvas);
                if let Some(idx) = self.widgets.iter().position(|w| w.id == id) {
                    let (cs, rs) = (self.widgets[idx].col_span, self.widgets[idx].row_span);
                    let ok = !self.widgets.iter().any(|w| w.overlaps_except(nc, nr, cs, rs, id));
                    if ok {
                        self.widgets[idx].col = nc;
                        self.widgets[idx].row = nr;
                    }
                }
            }
            DragState::Resizing { id, resize_right, resize_bottom, current } => {
                if let Some(idx) = self.widgets.iter().position(|w| w.id == id) {
                    let (fc, fr) = (self.widgets[idx].col, self.widgets[idx].row);
                    let (sc, sr) = self.snap(current, canvas);
                    let new_cs = if resize_right {
                        ((sc + 1).saturating_sub(fc)).max(1)
                    } else {
                        self.widgets[idx].col_span
                    };
                    let new_rs = if resize_bottom {
                        ((sr + 1).saturating_sub(fr)).max(1)
                    } else {
                        self.widgets[idx].row_span
                    };
                    let ok = !self.widgets.iter().any(|w| w.overlaps_except(fc, fr, new_cs, new_rs, id));
                    if ok {
                        self.widgets[idx].col_span = new_cs;
                        self.widgets[idx].row_span = new_rs;
                    }
                }
            }
            _ => {}
        }
    }

    /// Sets the cursor icon based on the pointer position and interaction state.
    pub(super) fn update_cursor(&self, ui: &egui::Ui, canvas: egui::Rect, pos: Option<egui::Pos2>) {
        let hover_pos = match pos {
            Some(p) => p,
            None => return,
        };

        match self.drag {
            DragState::Panning { .. } => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::AllScroll);
            }
            DragState::DraggingScrollbarX { .. } => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            DragState::DraggingScrollbarY { .. } => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            DragState::Resizing { resize_right, resize_bottom, .. } => {
                ui.ctx().set_cursor_icon(match (resize_right, resize_bottom) {
                    (true, true)  => egui::CursorIcon::ResizeNwSe,
                    (true, false) => egui::CursorIcon::ResizeHorizontal,
                    (false, true) => egui::CursorIcon::ResizeVertical,
                    _             => egui::CursorIcon::Default,
                });
            }
            DragState::Moving { .. } => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            _ => {
                // Check whether a scrollbar thumb is hovered.
                if let Some(thumb) = self.scrollbar_x_thumb(canvas) {
                    if thumb.contains(hover_pos) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        return;
                    }
                }
                if let Some(thumb) = self.scrollbar_y_thumb(canvas) {
                    if thumb.contains(hover_pos) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        return;
                    }
                }

                for w in &self.widgets {
                    let wr = self.wrect(w, canvas);
                    let hdr = w.header_rect(wr);
                    let close_btn_rect = w.close_btn_rect(hdr);
                    if close_btn_rect.contains(hover_pos) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        return;
                    }
                    if let Some((rr, rb)) = Self::resize_hit(hover_pos, wr) {
                        ui.ctx().set_cursor_icon(match (rr, rb) {
                            (true, true)  => egui::CursorIcon::ResizeNwSe,
                            (true, false) => egui::CursorIcon::ResizeHorizontal,
                            (false, true) => egui::CursorIcon::ResizeVertical,
                            _             => egui::CursorIcon::Default,
                        });
                        return;
                    }
                    if hdr.contains(hover_pos) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        return;
                    }
                }
                if canvas.contains(hover_pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                }
            }
        }
    }

    /// Returns the resize edges hit at a position, if any.
    ///
    /// The tuple indicates whether the right and bottom edges were hit.
    pub(super) fn resize_hit(pos: egui::Pos2, wr: egui::Rect) -> Option<(bool, bool)> {
        if !wr.expand(RESIZE_ZONE).contains(pos) { return None; }
        let near_right  = pos.x >= wr.max.x - RESIZE_ZONE
                       && pos.y >= wr.min.y
                       && pos.y <= wr.max.y + RESIZE_ZONE;
        let near_bottom = pos.y >= wr.max.y - RESIZE_ZONE
                       && pos.x >= wr.min.x
                       && pos.x <= wr.max.x + RESIZE_ZONE;
        if near_right || near_bottom { Some((near_right, near_bottom)) } else { None }
    }
}
