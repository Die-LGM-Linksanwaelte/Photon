use crate::panels::TabSaveData;
use common::networking::messages::UserRole;
use eframe::Theme;
use egui_dock::DockState;
use serde::{Deserialize, Serialize};
use common::networking::messages::UserRole::Programmer;

#[derive(Serialize, Deserialize)]
pub struct GuiConfig {
    pub theme: Theme,
    pub last_ip_address: String,
    pub last_username: String,
    pub last_role: UserRole,
    pub last_window_layout: String,
}

#[derive(Serialize, Deserialize)]
pub struct GuiLayout {
    pub role: UserRole,
    pub name: String,
    pub description: String,
    pub window_layout: DockState<TabSaveData>,
}

impl Default for GuiLayout {
    fn default() -> Self {
        Self {
            role: Programmer,
            name: "default user name".to_string(),
            description: "".to_string(),
            window_layout: DockState::new(vec![TabSaveData::Terminal]),
        }
    }
}