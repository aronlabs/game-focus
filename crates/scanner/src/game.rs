use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Steam,
    Epic,
    Custom,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Steam => write!(f, "Steam"),
            Platform::Epic => write!(f, "Epic Games"),
            Platform::Custom => write!(f, "Custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub install_dir: PathBuf,
    pub exe_dir: PathBuf,
    pub exe_path: PathBuf,
    pub enabled: bool,
    pub spoof_focus: bool,
    pub keep_audio: bool,
    pub background_controller: bool,
    pub unlock_cursor: bool,
}

impl GameProfile {
    pub fn new(
        id: String,
        name: String,
        platform: Platform,
        install_dir: PathBuf,
        exe_path: PathBuf,
    ) -> Self {
        let exe_dir = exe_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| install_dir.clone());

        Self {
            id,
            name,
            platform,
            install_dir,
            exe_dir,
            exe_path,
            enabled: false,
            spoof_focus: true,
            keep_audio: true,
            background_controller: true,
            unlock_cursor: false,
        }
    }
}
