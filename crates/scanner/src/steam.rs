use crate::game::{GameProfile, Platform};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Parse Steam's libraryfolders.vdf to find all Steam library paths across all drives
pub fn parse_library_folders(vdf_content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut in_library_folders = false;

    for line in vdf_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("\"libraryfolders\"") {
            in_library_folders = true;
            continue;
        }

        if in_library_folders {
            let tokens: Vec<&str> = trimmed
                .split('"')
                .filter(|s| !s.trim().is_empty())
                .collect();

            if tokens.len() >= 2 && tokens[0].eq_ignore_ascii_case("path") {
                let raw_path = tokens[1].replace("\\\\", "\\");
                paths.push(PathBuf::from(raw_path));
            }
        }
    }

    paths
}

/// Parse appmanifest_<appid>.acf to extract appid, name, and installdir
pub fn parse_app_manifest(acf_content: &str) -> Option<(String, String, String)> {
    let mut appid = None;
    let mut name = None;
    let mut installdir = None;

    for line in acf_content.lines() {
        let trimmed = line.trim();
        let tokens: Vec<&str> = trimmed
            .split('"')
            .filter(|s| !s.trim().is_empty())
            .collect();

        if tokens.len() >= 2 {
            let key = tokens[0].to_lowercase();
            let val = tokens[1].to_string();

            match key.as_str() {
                "appid" => appid = Some(val),
                "name" => name = Some(val),
                "installdir" => installdir = Some(val),
                _ => {}
            }
        }
    }

    match (appid, name, installdir) {
        (Some(id), Some(n), Some(dir)) => Some((id, n, dir)),
        _ => None,
    }
}

/// Heuristic to locate the main game executable within an installation folder
pub fn find_main_executable(game_dir: &Path) -> Option<PathBuf> {
    if !game_dir.exists() {
        return None;
    }

    let excluded = [
        "unins",
        "uninstall",
        "unitycrashhandler",
        "crashreport",
        "dxsetup",
        "vcredist",
        "easyanticheat",
        "beservice",
        "prereq",
        "setup",
    ];

    let mut candidates: Vec<PathBuf> = Vec::new();

    // Check Binaries/Win64 first (Unreal Engine convention)
    let ue_dir = game_dir.join("Binaries").join("Win64");
    if ue_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&ue_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("exe") {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if !excluded.iter().any(|ex| name.contains(ex)) {
                        return Some(path);
                    }
                }
            }
        }
    }

    // Walk up to 2 directories deep from game_dir
    for entry in WalkDir::new(game_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("exe") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !excluded.iter().any(|ex| name.contains(ex)) {
                candidates.push(path.to_path_buf());
            }
        }
    }

    // Sort candidates: prefer those matching folder name or largest file size
    if candidates.is_empty() {
        return None;
    }

    let folder_name = game_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if let Some(matching) = candidates.iter().find(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase().contains(&folder_name) || folder_name.contains(&s.to_lowercase()))
            .unwrap_or(false)
    }) {
        return Some(matching.clone());
    }

    // Fall back to first candidate
    candidates.into_iter().next()
}

/// Scan a specific Steam library folder for installed games
pub fn scan_steam_library(library_dir: &Path) -> Vec<GameProfile> {
    let mut games = Vec::new();
    let steamapps = library_dir.join("steamapps");
    if !steamapps.is_dir() {
        return games;
    }

    let common = steamapps.join("common");

    let entries = match fs::read_dir(&steamapps) {
        Ok(e) => e,
        Err(_) => return games,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some((appid, name, installdir)) = parse_app_manifest(&content) {
                    // Skip Steamworks common redistributables
                    if appid == "228980" {
                        continue;
                    }

                    let game_dir = common.join(&installdir);
                    if let Some(exe_path) = find_main_executable(&game_dir) {
                        let profile = GameProfile::new(
                            format!("steam_{}", appid),
                            name,
                            Platform::Steam,
                            game_dir,
                            exe_path,
                        );
                        games.push(profile);
                    }
                }
            }
        }
    }

    games
}

/// Scan all Steam libraries given a root Steam installation path
pub fn scan_steam_root(steam_root: &Path) -> Vec<GameProfile> {
    let mut libraries = Vec::new();
    libraries.push(steam_root.to_path_buf());

    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    if vdf_path.exists() {
        if let Ok(content) = fs::read_to_string(&vdf_path) {
            let parsed_libs = parse_library_folders(&content);
            for lib in parsed_libs {
                if !libraries.contains(&lib) {
                    libraries.push(lib);
                }
            }
        }
    }

    let mut all_games = Vec::new();
    for lib in libraries {
        all_games.extend(scan_steam_library(&lib));
    }

    all_games
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_library_folders() {
        let vdf = r#"
        "libraryfolders"
        {
            "0"
            {
                "path" "C:\\Program Files (x86)\\Steam"
                "label" ""
                "contentid" "1234"
                "apps"
                {
                    "228980" "100"
                }
            }
            "1"
            {
                "path" "D:\\SteamLibrary"
                "label" "Fast NVMe"
                "contentid" "5678"
                "apps"
                {
                    "1245620" "60000000000"
                }
            }
        }
        "#;

        let paths = parse_library_folders(vdf);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from(r"C:\Program Files (x86)\Steam"));
        assert_eq!(paths[1], PathBuf::from(r"D:\SteamLibrary"));
    }

    #[test]
    fn test_parse_app_manifest() {
        let acf = r#"
        "AppState"
        {
            "appid" "1245620"
            "Universe" "1"
            "name" "ELDEN RING"
            "StateFlags" "4"
            "installdir" "ELDEN RING"
            "LastUpdated" "1645745678"
        }
        "#;

        let res = parse_app_manifest(acf);
        assert!(res.is_some());
        let (appid, name, installdir) = res.unwrap();
        assert_eq!(appid, "1245620");
        assert_eq!(name, "ELDEN RING");
        assert_eq!(installdir, "ELDEN RING");
    }
}
