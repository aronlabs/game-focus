use crate::game::{GameProfile, Platform};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EpicItem {
    display_name: Option<String>,
    install_location: Option<String>,
    launch_executable: Option<String>,
    app_name: Option<String>,
}

/// Parse an individual Epic Games .item manifest file
pub fn parse_epic_manifest(content: &str) -> Option<GameProfile> {
    let item: EpicItem = serde_json::from_str(content).ok()?;

    let name = item.display_name?;
    let install_dir_str = item.install_location?;
    let launch_exe = item.launch_executable?;
    let app_name = item.app_name.unwrap_or_else(|| name.clone());

    let install_dir = PathBuf::from(install_dir_str);
    let exe_path = install_dir.join(&launch_exe);

    Some(GameProfile::new(
        format!("epic_{}", app_name),
        name,
        Platform::Epic,
        install_dir,
        exe_path,
    ))
}

/// Scan an Epic Games manifests directory for installed games
pub fn scan_epic_dir(manifests_dir: &Path) -> Vec<GameProfile> {
    let mut games = Vec::new();

    let entries = match fs::read_dir(manifests_dir) {
        Ok(e) => e,
        Err(_) => return games,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("item") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(profile) = parse_epic_manifest(&content) {
                    if profile.exe_path.exists() {
                        games.push(profile);
                    }
                }
            }
        }
    }

    games
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_epic_manifest() {
        let json = r#"{
            "FormatVersion": 0,
            "AppVersionString": "1.0.0",
            "AppName": "GrandTheftAutoV",
            "DisplayName": "Grand Theft Auto V",
            "InstallLocation": "D:\\EpicGames\\GTAV",
            "LaunchExecutable": "PlayGTAV.exe"
        }"#;

        let profile = parse_epic_manifest(json);
        assert!(profile.is_some());
        let p = profile.unwrap();
        assert_eq!(p.name, "Grand Theft Auto V");
        assert_eq!(p.platform, Platform::Epic);
        assert_eq!(p.install_dir, PathBuf::from(r"D:\EpicGames\GTAV"));
        assert!(p.exe_path.ends_with("PlayGTAV.exe"));
    }
}
