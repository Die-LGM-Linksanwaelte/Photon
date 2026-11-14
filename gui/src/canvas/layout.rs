//! Management of page slots for canvas layouts.
//!
//! Synchronizes slot names and the active index with any placed ViewBar widgets,
//! and handles page switching and clearing.

use eframe::egui;
use crate::panels::Tab;
use crate::panels::view_bar::{SlotEntry, ViewBarPanel};
use super::{Canvas, GridRect};

/// A populated canvas page stored in a slot.
pub struct PageSlot {
    /// Display name of the page.
    pub name: String,
    /// Canvas contents of the page.
    pub canvas: Canvas,
}

/// Manages page slots. Pages can be switched through a ViewBar widget or the
/// fallback menu in the menu bar.
pub struct CanvasLayout {
    /// Page slots; `None` represents an empty slot.
    pub slots: Vec<Option<PageSlot>>,
    /// Index of the currently selected slot.
    pub current_slot: usize,
}

#[allow(dead_code)]
pub type MaLayout = CanvasLayout;

impl CanvasLayout {
    /// Creates a layout with ten slots and initializes the first page with a ViewBar.
    pub fn new() -> Self {
        let mut slots: Vec<Option<PageSlot>> = Vec::new();
        slots.resize_with(10, || None);

        // Initialize page 1 with a 2-by-6 ViewBar widget on the left.
        let mut initial_canvas = Canvas::new();
        initial_canvas.place_widget(
            Tab::ViewBar(ViewBarPanel::new(0, 10)),
            GridRect { col: 0, row: 0, col_span: 2, row_span: 6 },
        );

        slots[0] = Some(PageSlot {
            name: "Seite 1".into(),
            canvas: initial_canvas,
        });

        Self {
            slots,
            current_slot: 0,
        }
    }

    /// Returns a mutable reference to the current page's canvas, creating the page if needed.
    pub fn current_canvas_mut(&mut self) -> &mut Canvas {
        if self.current_slot >= self.slots.len() {
            self.slots.resize_with(self.current_slot + 1, || None);
        }
        if self.slots[self.current_slot].is_none() {
            self.slots[self.current_slot] = Some(PageSlot {
                name: format!("Seite {}", self.current_slot + 1),
                canvas: Canvas::new(),
            });
        }
        &mut self.slots[self.current_slot].as_mut().unwrap().canvas
    }

    /// Switches to a page slot, creating the slot and page if needed.
    pub fn switch_to(&mut self, slot: usize) {
        if slot >= self.slots.len() {
            self.slots.resize_with(slot + 1, || None);
        }
        if self.slots[slot].is_none() {
            let mut new_canvas = Canvas::new();
            new_canvas.place_widget(
                Tab::ViewBar(ViewBarPanel::new(0, self.slots.len().max(10))),
                GridRect { col: 0, row: 0, col_span: 2, row_span: 6 },
            );
            self.slots[slot] = Some(PageSlot {
                name: format!("Seite {}", slot + 1),
                canvas: new_canvas,
            });
        }
        self.current_slot = slot;
    }

    /// Renames a populated page slot.
    pub fn rename_slot(&mut self, slot: usize, name: String) {
        if slot < self.slots.len() {
            if let Some(ref mut s) = self.slots[slot] {
                s.name = name;
            }
        }
    }

    /// Clears a page slot and selects a remaining page if the current page was cleared.
    pub fn clear_slot(&mut self, slot: usize) {
        if slot < self.slots.len() {
            self.slots[slot] = None;
            if self.current_slot == slot {
                if let Some(first) = self.slots.iter().position(|s| s.is_some()) {
                    self.current_slot = first;
                } else {
                    let mut fallback = Canvas::new();
                    fallback.place_widget(
                        Tab::ViewBar(ViewBarPanel::new(0, self.slots.len().max(10))),
                        GridRect { col: 0, row: 0, col_span: 2, row_span: 6 },
                    );
                    self.slots[0] = Some(PageSlot {
                        name: "Seite 1".into(),
                        canvas: fallback,
                    });
                    self.current_slot = 0;
                }
            }
        }
    }

    /// Synchronizes ViewBar slot state with the layout and renders the current canvas.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let current_slot = self.current_slot;
        let slot_names: Vec<Option<String>> = self.slots.iter()
            .map(|s| s.as_ref().map(|ps| ps.name.clone()))
            .collect();

        for widget in &mut self.current_canvas_mut().widgets {
            if let Tab::ViewBar(ref mut vb) = widget.content {
                let count = vb.slots.len().max(slot_names.len()).max(10);
                let slot_entries: Vec<SlotEntry> = (0..count)
                    .map(|i| {
                        if let Some(name) = slot_names.get(i).and_then(|n| n.as_ref()) {
                            SlotEntry {
                                name: name.clone(),
                                is_empty: false,
                            }
                        } else {
                            SlotEntry {
                                name: format!("Seite {}", i + 1),
                                is_empty: true,
                            }
                        }
                    })
                    .collect();
                vb.slots = slot_entries;
                vb.current_slot = current_slot;
            }
        }

        self.current_canvas_mut().ui(ui);
    }
}
