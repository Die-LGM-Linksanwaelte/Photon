//! # ViewBarPanel
//!
//! A canvas widget for switching between pages. It renders one button per grid cell,
//! reserving the top-left cell for the header drawn by the canvas. The number of slot
//! entries grows to fill the available grid cells.

use eframe::egui;
use crate::controller::{send_ui_event, UiEvent};

/// Width and height, in points, of one ViewBar grid cell.
pub const CELL_SIZE: f32 = 48.0;

/// The display state for one page slot in the ViewBar.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotEntry {
    /// The page name shown in the slot tooltip and label.
    pub name: String,
    /// Whether this slot has no page assigned to it.
    pub is_empty: bool,
}

/// A canvas widget that displays page slots as grid-aligned buttons.
#[derive(Clone)]
pub struct ViewBarPanel {
    /// The identifier of the tab containing this panel.
    pub tab_id: u32,
    /// Local copy of the page slot states, synchronized by the canvas layout.
    pub slots: Vec<SlotEntry>,
    /// Index of the currently selected page slot.
    pub current_slot: usize,
    /// Index and draft name of the slot currently being renamed.
    pub editing_slot: Option<(usize, String)>,
}

/// An interaction requested by a ViewBar slot button.
enum SlotAction {
    /// Switch to the specified slot.
    Switch(usize),
    /// Clear the specified slot.
    Clear(usize),
    /// Begin renaming the specified slot with the given draft name.
    StartRename(usize, String),
    /// Finish renaming the specified slot with the given name.
    ConfirmRename(usize, String),
}

impl ViewBarPanel {
    /// Creates a ViewBar with the requested number of slots.
    ///
    /// The first slot starts occupied; all subsequent slots start empty.
    pub fn new(tab_id: u32, initial_slot_count: usize) -> Self {
        let slots = (0..initial_slot_count)
            .map(|i| SlotEntry {
                name: if i == 0 { "Page 1".to_string() } else { format!("Page {}", i + 1) },
                is_empty: i != 0,
            })
            .collect();

        Self {
            tab_id,
            slots,
            current_slot: 0,
            editing_slot: None,
        }
    }

    /// Adds empty slots until the grid, excluding its header cell, has one entry per cell.
    fn ensure_capacity(&mut self, cols: usize, rows: usize) {
        let capacity = cols.saturating_mul(rows).saturating_sub(1);

        while self.slots.len() < capacity {
            let i = self.slots.len();
            self.slots.push(SlotEntry {
                name: format!("Page {}", i + 1),
                is_empty: true,
            });
        }
    }

    /// Renders the ViewBar and applies slot interactions.
    ///
    /// The slot list is extended to fill the available grid area. Existing entries are
    /// retained if the widget later becomes smaller.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let avail_w = ui.available_width();
        let avail_h = ui.available_height();
        let cols = ((avail_w / CELL_SIZE).round() as usize).max(1);
        let rows = ((avail_h / CELL_SIZE).round() as usize).max(1);


        self.ensure_capacity(cols, rows);

        let base_min = ui.min_rect().min;

        let mut switch_to = None;
        let mut start_rename = None;
        let mut clear_slot = None;
        let mut confirm_rename = None;

        for i in 0..self.slots.len() {
            if let Some(action) = self.render_slot_button(ui, i, cols, rows, base_min) {
                match action {
                    SlotAction::Switch(i) => switch_to = Some(i),
                    SlotAction::Clear(i) => clear_slot = Some(i),
                    SlotAction::StartRename(i, draft) => start_rename = Some((i, draft)),
                    SlotAction::ConfirmRename(i, new_name) => confirm_rename = Some((i, new_name)),
                }
            }
        }

        if let Some(i) = switch_to {
            self.current_slot = i;
            self.editing_slot = None;
            send_ui_event(UiEvent::SwitchCanvasSlot(i));
        }

        if let Some(i) = clear_slot {
            send_ui_event(UiEvent::ClearCanvasSlot(i));
        }

        if let Some((i, draft)) = start_rename {
            self.editing_slot = Some((i, draft));
        }

        if let Some((i, new_name)) = confirm_rename {
            let trimmed = new_name.trim();
            let final_name = if trimmed.is_empty() { format!("Page {}", i + 1) } else { trimmed.to_string() };
            self.slots[i].name = final_name.clone();
            self.editing_slot = None;
            send_ui_event(UiEvent::RenameCanvasSlot { slot: i, name: final_name });
        }
    }

    /// Renders one slot button and returns any interaction requested by the user.
    ///
    /// Slots outside the visible grid rows are skipped.
    fn render_slot_button(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        cols: usize,
        rows: usize,
        base_min: egui::Pos2,
    ) -> Option<SlotAction> {
        let v = ui.visuals().clone();
        let accent = v.hyperlink_color;
        let font_num = egui::FontId::monospace(12.0);
        let font_sub = egui::FontId::proportional(9.0);

        let slot_cell_idx = i + 1;
        let cell_c = slot_cell_idx % cols;
        let cell_r = slot_cell_idx / cols;

        if cell_r >= rows && rows > 0 {
            return None;
        }

        let slot = &self.slots[i];
        let is_active = self.current_slot == i && !slot.is_empty;
        let is_editing = matches!(self.editing_slot, Some((idx, _)) if idx == i);

        let btn_pos = base_min + egui::vec2(
            cell_c as f32 * CELL_SIZE + 1.0,
            cell_r as f32 * CELL_SIZE + 1.0,
        );
        let btn_size = egui::vec2(CELL_SIZE - 2.0, CELL_SIZE - 2.0);
        let rect = egui::Rect::from_min_size(btn_pos, btn_size);

        let sense = if is_editing { egui::Sense::hover() } else { egui::Sense::click() };
        let response = ui.allocate_rect(rect, sense);

        let painter = ui.painter_at(rect);
        let rounding = egui::Rounding::same(3.0);
        let hovered = response.hovered() && !is_editing;
        let mut action = None;

        if !is_editing {
            response.context_menu(|ui| {
                if !slot.is_empty {
                    ui.label(egui::RichText::new(format!("Slot {:02}: {}", i + 1, slot.name)).strong());
                    ui.separator();
                    if ui.button("✏  Rename").clicked() {
                        action = Some(SlotAction::StartRename(i, slot.name.clone()));
                        ui.close_menu();
                    }
                    if ui.button("🗑  Clear").clicked() {
                        action = Some(SlotAction::Clear(i));
                        ui.close_menu();
                    }
                } else {
                    ui.label(egui::RichText::new(format!("Slot {:02} (Leer)", i + 1)).weak());
                    ui.separator();
                    if ui.button("＋ Add new").clicked() {
                        action = Some(SlotAction::Switch(i));
                        ui.close_menu();
                    }
                }
            });

            if response.clicked() {
                action = Some(SlotAction::Switch(i));
            }
            if response.double_clicked() && !slot.is_empty {
                action = Some(SlotAction::StartRename(i, slot.name.clone()));
            }

            // Show a tooltip describing the slot and its available interactions.
            if !slot.is_empty {
                response.on_hover_text(format!(
                    "Slot {:02}: {}\nClick: Select\nDouble Click: Rename\nRight Click: Options",
                    i + 1, slot.name
                ));
            } else {
                response.on_hover_text(format!("Slot {:02} (empty)\nClick: Create Page", i + 1));
            }
        }

        // Draw the button.
        if is_editing {
            painter.rect_filled(rect, rounding, v.extreme_bg_color);
            painter.rect_stroke(rect, rounding, egui::Stroke::new(1.5, accent));

            let edit_rect = rect.shrink(2.0);
            if let Some((_, ref mut draft)) = self.editing_slot {
                let mut child = ui.child_ui(edit_rect, egui::Layout::top_down(egui::Align::Center), None);
                let te = child.add(
                    egui::TextEdit::singleline(draft)
                        .desired_width(edit_rect.width())
                        .hint_text("Name"),
                );
                te.request_focus();
                if te.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                    action = Some(SlotAction::ConfirmRename(i, draft.clone()));                }
                if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                    self.editing_slot = None;
                }
            }
        } else {
            let (bg_color, stroke) = if is_active {
                (
                    egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 50),
                    egui::Stroke::new(2.0, accent),
                )
            } else if !slot.is_empty {
                if hovered {
                    (v.widgets.hovered.bg_fill, egui::Stroke::new(1.0, v.widgets.hovered.bg_stroke.color))
                } else {
                    (v.widgets.inactive.bg_fill, egui::Stroke::new(1.0, v.widgets.inactive.bg_stroke.color))
                }
            } else {
                // Empty slot.
                if hovered {
                    (
                        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 25),
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 120)),
                    )
                } else {
                    (
                        v.extreme_bg_color,
                        egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(
                            v.window_stroke().color.r(),
                            v.window_stroke().color.g(),
                            v.window_stroke().color.b(),
                            60,
                        )),
                    )
                }
            };

            painter.rect_filled(rect, rounding, bg_color);
            painter.rect_stroke(rect, rounding, stroke);

            let num_color = if is_active {
                accent
            } else if !slot.is_empty {
                v.strong_text_color()
            } else {
                v.weak_text_color()
            };

            painter.text(
                rect.center_top() + egui::vec2(0.0, 6.0),
                egui::Align2::CENTER_TOP,
                format!("{:02}", i + 1),
                font_num.clone(),
                num_color,
            );

            // Draw the lower label or active-slot indicator.
            if is_active {
                painter.circle_filled(
                    rect.center_bottom() + egui::vec2(0.0, -8.0),
                    2.5,
                    accent,
                );
            } else if !slot.is_empty {
                let short_name: String = slot.name.chars().take(4).collect();
                painter.text(
                    rect.center_bottom() + egui::vec2(0.0, -5.0),
                    egui::Align2::CENTER_BOTTOM,
                    short_name,
                    font_sub.clone(),
                    v.weak_text_color(),
                );
            } else {
                painter.text(
                    rect.center_bottom() + egui::vec2(0.0, -5.0),
                    egui::Align2::CENTER_BOTTOM,
                    "+",
                    font_sub.clone(),
                    v.weak_text_color().linear_multiply(0.6),
                );
            }
        }
        action
    }
}
