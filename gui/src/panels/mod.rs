//! # UI Panels Module
//!
//! This module defines the core panel types and docking tab structures used by the GUI interface.
//! It exposes the [`Tab`] enum encapsulating various panel components like [`UniversePanel`] and [`TerminalPanel`].

use std::fmt;
use std::fmt::{Display, Formatter};
use eframe::egui;
use serde::{Deserialize, Serialize};
use common::logging::LogLevel::Error;
use common::networking::subscription_objects::SubscribeTopic::DMXConfiguration;
use common::r_log;

/// Module implementing the interactive terminal panel tab.
pub mod terminal;
/// Module implementing the visual DMX universe panel tab.
pub mod universe;
/// Module implementing the patch panel tab.
pub mod patch;
/// Module implementing the canvas page switcher widget (ViewBar).
pub mod view_bar;

use terminal::TerminalPanel;
use universe::UniversePanel;
use patch::PatchPanel;
use view_bar::ViewBarPanel;
use crate::controller::{send_ui_event, UiEvent};
use crate::UI_EVENT_SENDER;

/// Struct to describe in wich modi the widget is allowed to be used
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCompatibility {
    /// Allowed in both Classic and Canvas modes.
    Both,
    /// Allowed only in Classic mode.
    ClassicOnly,
    /// Allowed only in Canvas mode.
    CanvasOnly,
}

#[allow(dead_code)]
impl TabCompatibility {
    pub fn allows_classic(&self) -> bool {
        matches!(self, TabCompatibility::Both | TabCompatibility::ClassicOnly)
    }

    pub fn allows_canvas(&self) -> bool {
        matches!(self, TabCompatibility::Both | TabCompatibility::CanvasOnly)
    }
}

/// Enum
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TabSaveData {
    Terminal,
    Patch,
    Universe { selected_universe: u8 },
    ViewBar { slot_count: usize },
}

/// Enum representing all dockable tab types supported in the user interface.
#[derive(Clone)]
pub enum Tab {
    /// Tab displaying a visual DMX universe channel grid.
    Universe(UniversePanel),
    /// Tab displaying an interactive terminal command log.
    Terminal(TerminalPanel),
    /// Tab
    Patch(PatchPanel),
    /// Widget displaying the canvas page-switching bar (ViewBar).
    ViewBar(ViewBarPanel),
}

impl Display for Tab {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Tab::Universe{ .. } => {
                write!(f, "{}", serde_json::to_string("UniverseTab").unwrap())},
            Tab::Terminal{ .. } => {
                write!(f, "{}", serde_json::to_string("TerminalTab").unwrap())},
            Tab::Patch{ .. } => {
                write!(f, "{}", serde_json::to_string("PatchTab").unwrap())},
            Tab::ViewBar{ .. } => {
                write!(f, "{}", serde_json::to_string("ViewBarTab").unwrap())},
        }
    }
}

impl Tab {
    /// Returns the compatibility of this tab with the different layout modes (Classic vs. Canvas).
    #[allow(dead_code)]
    pub fn compatibility(&self) -> TabCompatibility {
        match self {
            Tab::Universe(_) => TabCompatibility::Both,
            Tab::Terminal(_) => TabCompatibility::Both,
            Tab::Patch(_) => TabCompatibility::Both,
            Tab::ViewBar(_) => TabCompatibility::CanvasOnly,
        }
    }

    /// Returns the human-readable display title for this tab header.
    pub fn title(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("Universe {}", panel.selected_universe),
            Tab::Terminal(_) => "Terminal".to_string(),
            Tab::Patch(_) => "Patch".to_string(),
            Tab::ViewBar(_) => "Views".to_string(),
        }
    }
    /// Returns true if this tab is of the same variant type as the other tab.
    pub fn is_same_type(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Returns a unique string identifier for this tab instance used by the dock state.
    pub fn unique_id(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("universe_tab_{}", panel.tab_id),
            Tab::Terminal(panel) => format!("terminal_tab_{}", panel.tab_id),
            Tab::Patch(panel) => format!("patch_tab_{}", panel.tab_id),
            Tab::ViewBar(panel) => format!("view_bar_tab_{}", panel.tab_id),
        }
    }

    /// Renders the content UI for the active tab variant.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Tab::Universe(panel) => panel.ui(ui),
            Tab::Terminal(panel) => panel.ui(ui),
            Tab::Patch(panel) => panel.ui(ui),
            Tab::ViewBar(panel) => panel.ui(ui),
        }
    }

    /// Callback triggered when the application successfully connects and authenticates.
    ///
    /// Initiates necessary server subscriptions (such as requesting DMX configuration updates).
    pub fn on_connect(&mut self) {
        match self {
            Tab::Universe(_) => {
                if let Some(sender) = UI_EVENT_SENDER.read().unwrap().as_ref() {
                    if let Err(e) = sender.send(UiEvent::SubscribeRequest {topic: DMXConfiguration}) {
                        r_log!(Error, "Failed to send UiEvent: {}", e);
                    }
                }
            }
            Tab::Patch(_) => {}
            _ => {}
        }
    }

    pub fn on_close(&mut self) {
        send_ui_event(UiEvent::CloseTab {
            tab: self.clone(),
        });
    }
}
