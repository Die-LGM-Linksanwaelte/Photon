//! # Configuration Module
//!
//! Defines persistent application configurations and window layout state structures,
//! including default layout templates and serde serialization implementations.

use crate::panels::TabSaveData;
use common::networking::messages::UserRole;
use eframe::Theme;
use egui_dock::DockState;
use serde::{Deserialize, Serialize};
use common::networking::messages::UserRole::Programmer;

/// Represents persistent application-wide preferences and session settings.
#[derive(Serialize, Deserialize)]
pub struct GuiConfig {
    /// Active UI theme (Light or Dark).
    pub theme: Theme,
    /// Last used kernel server IP address.
    pub last_ip_address: String,
    /// Last authenticated username.
    pub last_username: String,
    /// Last selected user role.
    pub last_role: UserRole,
    /// Path or identifier of the last active window layout.
    pub last_window_layout: String,
}

/// Represents a serializable window layout preset, capturing docking surfaces and tab hierarchies.
#[derive(Serialize, Deserialize)]
pub struct GuiLayout {
    /// The target user role this layout is associated with.
    pub role: UserRole,
    /// Human-readable preset name.
    pub name: String,
    /// Detailed description of the layout preset.
    pub description: String,
    /// Serialized docking tree state containing all open panels and tabs.
    pub window_layout: DockState<TabSaveData>,
}

impl Default for GuiLayout {
    /// Returns the default window layout containing an initial terminal tab.
    fn default() -> Self {
        Self {
            role: Programmer,
            name: "default user name".to_string(),
            description: "".to_string(),
            window_layout: DockState::new(vec![TabSaveData::Terminal]),
        }
    }
}