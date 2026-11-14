//! Data structures, geometry types, and theme colors used by the canvas system.

use eframe::egui;
use crate::panels::{Tab, TabCompatibility};
use crate::panels::view_bar::CELL_SIZE;

/// Height of a conventional widget header, in points.
pub const HEADER_H:            f32 = 26.0;
/// Width of the interactive resize area along widget edges, in points.
pub const RESIZE_ZONE:         f32 = 10.0;
/// Thickness of the canvas scrollbars, in points.
pub const SCROLLBAR_THICKNESS: f32 = 8.0;

/// A rectangular region in the canvas grid.
pub struct GridRect {
    /// Leftmost grid column.
    pub col:      usize,
    /// Topmost grid row.
    pub row:      usize,
    /// Width of the region in grid cells.
    pub col_span: usize,
    /// Height of the region in grid cells.
    pub row_span: usize,
}

/// A panel widget placed on the canvas with a grid position and size.
pub struct PlacedWidget {
    /// Unique identifier within its canvas.
    pub id:       u32,
    /// Widget content and behavior.
    pub content:  Tab,
    /// Leftmost grid column.
    pub col:      usize,
    /// Topmost grid row.
    pub row:      usize,
    /// Width of the widget in grid cells.
    pub col_span: usize,
    /// Height of the widget in grid cells.
    pub row_span: usize,
}

impl PlacedWidget {
    /// Returns whether this widget overlaps a region, excluding `except_id`.
    pub fn overlaps_except(&self, col: usize, row: usize, col_span: usize, row_span: usize, except_id: u32) -> bool {
        if self.id == except_id { return false; }
        !(self.col + self.col_span <= col
            || col + col_span <= self.col
            || self.row + self.row_span <= row
            || row + row_span <= self.row)
    }

    /// Returns whether this widget overlaps the specified region.
    pub fn overlaps(&self, col: usize, row: usize, col_span: usize, row_span: usize) -> bool {
        self.overlaps_except(col, row, col_span, row_span, u32::MAX)
    }

    /// Returns whether this is a canvas-only widget with a top-left corner header.
    pub fn is_corner_header(&self) -> bool {
        self.content.compatibility() == TabCompatibility::CanvasOnly
    }

    /// Calculates the header rectangle: either the top-left cell or a full-width bar.
    pub fn header_rect(&self, wr: egui::Rect) -> egui::Rect {
        if self.is_corner_header() {
            egui::Rect::from_min_size(wr.min, egui::vec2(CELL_SIZE, CELL_SIZE))
        } else {
            egui::Rect::from_min_size(wr.min, egui::vec2(wr.width(), HEADER_H))
        }
    }

    /// Calculates the close button rectangle within the header.
    pub fn close_btn_rect(&self, hdr: egui::Rect) -> egui::Rect {
        if self.is_corner_header() {
            egui::Rect::from_min_size(hdr.right_top() + egui::vec2(-16.0, 2.0), egui::vec2(14.0, 14.0))
        } else {
            egui::Rect::from_min_size(hdr.right_top() + egui::vec2(-22.0, 4.0), egui::vec2(18.0, 18.0))
        }
    }

    /// Calculates the content rectangle used by the widget's child UI.
    pub fn content_rect(&self, wr: egui::Rect) -> egui::Rect {
        if self.is_corner_header() {
            wr // Use the full widget rectangle without a header offset.
        } else {
            egui::Rect::from_min_max(
                wr.min + egui::vec2(1.0, HEADER_H + 1.0),
                wr.max - egui::vec2(1.0, 1.0),
            )
        }
    }
}

/// Current pointer interaction state, including selection, movement, resizing,
/// panning, and scrollbar dragging.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragState {
    Idle,
    Selecting  { start: egui::Pos2, current: egui::Pos2 },
    Moving     { id: u32, offset: egui::Vec2, current: egui::Pos2 },
    Resizing   { id: u32, resize_right: bool, resize_bottom: bool, current: egui::Pos2 },
    Panning    { start_pos: egui::Pos2, start_offset: egui::Vec2 },
    DraggingScrollbarX { start_x: f32, start_offset_x: f32 },
    DraggingScrollbarY { start_y: f32, start_offset_y: f32 },
}

/// Canvas colors derived from the active egui theme.
#[derive(Clone, Copy, Debug)]
pub struct Colors {
    /// Canvas background color.
    pub bg:           egui::Color32,
    /// Grid line color.
    pub grid:         egui::Color32,
    /// Widget background color.
    pub widget_bg:    egui::Color32,
    /// Default widget header background color.
    pub hdr_bg:       egui::Color32,
    /// Active or hovered widget header background color.
    pub hdr_active:   egui::Color32,
    /// Default border color.
    pub border:       egui::Color32,
    /// Accent border color.
    pub border_act:   egui::Color32,
    /// Primary text color.
    pub text:         egui::Color32,
    /// Selection fill color.
    pub sel_fill:     egui::Color32,
    /// Selection outline color.
    pub sel_stroke:   egui::Color32,
    /// Translucent interaction preview fill color.
    pub ghost_fill:   egui::Color32,
    /// Interaction preview outline color.
    pub ghost_stroke: egui::Color32,
}

impl Colors {
    /// Builds the canvas palette from the active egui visuals.
    pub fn from_ui(ui: &egui::Ui) -> Self {
        let v      = ui.visuals();
        let accent = v.hyperlink_color;

        let ws = v.window_stroke();
        let grid = egui::Color32::from_rgba_unmultiplied(
            ws.color.r(), ws.color.g(), ws.color.b(), 70,
        );

        let sbf = v.selection.bg_fill;
        let sel_fill = egui::Color32::from_rgba_unmultiplied(
            sbf.r(), sbf.g(), sbf.b(), 55,
        );
        let ghost_fill = egui::Color32::from_rgba_unmultiplied(
            accent.r(), accent.g(), accent.b(), 35,
        );

        Self {
            bg:           v.extreme_bg_color,
            grid,
            widget_bg:    v.window_fill(),
            hdr_bg:       v.widgets.noninteractive.bg_fill,
            hdr_active:   v.widgets.hovered.bg_fill,
            border:       ws.color,
            border_act:   accent,
            text:         v.text_color(),
            sel_fill,
            sel_stroke:   v.selection.stroke.color,
            ghost_fill,
            ghost_stroke: accent,
        }
    }
}
