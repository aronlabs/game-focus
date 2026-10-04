use crate::embedded::VERSION_DLL_BYTES;
use game_focus_scanner::{
    check_hook_status, deploy_hook, remove_hook, scan_all_installed_games, update_config,
    GameProfile, Platform,
};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FilterCategory {
    All,
    EnabledOnly,
    Steam,
    Epic,
    Custom,
}

pub struct AppState {
    pub games: Vec<GameProfile>,
    pub search_query: String,
    pub filter: FilterCategory,
    pub status_message: Option<(String, Instant)>,
    pub minimize_requested: bool,
    pub quit_requested: bool,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = Self {
            games: Vec::new(),
            search_query: String::new(),
            filter: FilterCategory::All,
            status_message: None,
            minimize_requested: false,
            quit_requested: false,
        };
        state.rescan();
        state
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn status(&self) -> Option<&str> {
        if let Some((msg, time)) = &self.status_message {
            if time.elapsed() < Duration::from_secs(4) {
                return Some(msg);
            }
        }
        None
    }

    fn custom_games_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let dir = PathBuf::from(appdata).join("GameFocusManager");
            let _ = fs::create_dir_all(&dir);
            dir.join("custom_games.json")
        } else {
            PathBuf::from("custom_games.json")
        }
    }

    fn load_custom_games(&self) -> Vec<GameProfile> {
        let path = Self::custom_games_path();
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut list) = serde_json::from_str::<Vec<GameProfile>>(&content) {
                    for g in &mut list {
                        check_hook_status(g);
                    }
                    return list;
                }
            }
        }
        Vec::new()
    }

    fn save_custom_games(&self) {
        let custom_list: Vec<&GameProfile> = self
            .games
            .iter()
            .filter(|g| g.platform == Platform::Custom)
            .collect();

        let path = Self::custom_games_path();
        if let Ok(json) = serde_json::to_string_pretty(&custom_list) {
            let _ = fs::write(path, json);
        }
    }

    pub fn rescan(&mut self) {
        let mut scanned = scan_all_installed_games();
        let custom = self.load_custom_games();

        for c in custom {
            if !scanned.iter().any(|g| g.exe_path == c.exe_path) {
                scanned.push(c);
            }
        }

        scanned.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.games = scanned;
        self.set_status(format!("Scanned {} games across all drives", self.games.len()));
    }

    pub fn add_custom_game(&mut self, exe_path: PathBuf) {
        if !exe_path.is_file() {
            return;
        }

        let name = exe_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Custom Game")
            .to_string();

        let install_dir = exe_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let mut profile = GameProfile::new(
            format!("custom_{}", name.to_lowercase()),
            name.clone(),
            Platform::Custom,
            install_dir,
            exe_path,
        );
        check_hook_status(&mut profile);

        if !self.games.iter().any(|g| g.exe_path == profile.exe_path) {
            self.games.push(profile);
            self.save_custom_games();
            self.set_status(format!("Added custom game: {}", name));
        }
    }

    pub fn enable_game(&mut self, id: &str) {
        let status_msg = if let Some(game) = self.games.iter_mut().find(|g| g.id == id) {
            game.enabled = true;
            match deploy_hook(game, VERSION_DLL_BYTES) {
                Ok(_) => format!("Enabled background focus for {}", game.name),
                Err(e) => {
                    game.enabled = false;
                    format!("Failed to deploy hook: {}", e)
                }
            }
        } else {
            return;
        };
        self.set_status(status_msg);
    }

    pub fn disable_game(&mut self, id: &str) {
        let status_msg = if let Some(game) = self.games.iter_mut().find(|g| g.id == id) {
            game.enabled = false;
            match remove_hook(game) {
                Ok(_) => format!("Disabled background focus for {}", game.name),
                Err(e) => format!("Failed to remove hook: {}", e),
            }
        } else {
            return;
        };
        self.set_status(status_msg);
    }

    pub fn update_game_settings(&mut self, id: &str) {
        if let Some(game) = self.games.iter().find(|g| g.id == id) {
            if game.enabled {
                let _ = update_config(game);
                self.set_status(format!("Updated settings for {}", game.name));
            }
        }
    }

    pub fn filtered_games(&self) -> Vec<&GameProfile> {
        let query = self.search_query.trim().to_lowercase();
        self.games
            .iter()
            .filter(|g| {
                if !query.is_empty() && !g.name.to_lowercase().contains(&query) {
                    return false;
                }
                match self.filter {
                    FilterCategory::All => true,
                    FilterCategory::EnabledOnly => g.enabled,
                    FilterCategory::Steam => g.platform == Platform::Steam,
                    FilterCategory::Epic => g.platform == Platform::Epic,
                    FilterCategory::Custom => g.platform == Platform::Custom,
                }
            })
            .collect()
    }
}
