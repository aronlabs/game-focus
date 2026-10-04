pub mod deployer;
pub mod epic;
pub mod game;
pub mod steam;

pub use deployer::{check_hook_status, deploy_hook, remove_hook, update_config};
pub use game::{GameProfile, Platform};

use std::path::PathBuf;

/// Get the Epic Games manifests directory
pub fn find_epic_manifests_path() -> PathBuf {
    if let Ok(progdata) = std::env::var("ProgramData") {
        PathBuf::from(progdata)
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests")
    } else {
        PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests")
    }
}

/// Find Steam root installation path
pub fn find_steam_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExA, RegQueryValueExA, HKEY, HKEY_CURRENT_USER, KEY_READ,
        };

        unsafe {
            let mut hkey: HKEY = std::ptr::null_mut();
            let subkey = b"Software\\Valve\\Steam\0";
            if RegOpenKeyExA(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) == 0 {
                let val_name = b"SteamPath\0";
                let mut buf = [0u8; 512];
                let mut buf_len = buf.len() as u32;

                let res = RegQueryValueExA(
                    hkey,
                    val_name.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    buf.as_mut_ptr(),
                    &mut buf_len,
                );
                RegCloseKey(hkey);

                if res == 0 && buf_len > 1 {
                    let path_str = String::from_utf8_lossy(&buf[..(buf_len - 1) as usize]);
                    let path = PathBuf::from(path_str.trim().replace('/', "\\"));
                    if path.is_dir() {
                        return Some(path);
                    }
                }
            }
        }
    }

    // Standard fallback locations across drives
    let common_paths = [
        r"C:\Program Files (x86)\Steam",
        r"C:\Steam",
        r"D:\Steam",
        r"E:\Steam",
    ];

    for p in &common_paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return Some(path);
        }
    }

    None
}

/// Scan all installed games across Steam (all secondary/primary drives) and Epic Games
pub fn scan_all_installed_games() -> Vec<GameProfile> {
    let mut games = Vec::new();

    // 1. Steam
    if let Some(steam_root) = find_steam_path() {
        games.extend(steam::scan_steam_root(&steam_root));
    }

    // 2. Epic Games
    let epic_dir = find_epic_manifests_path();
    if epic_dir.is_dir() {
        games.extend(epic::scan_epic_dir(&epic_dir));
    }

    // Check hook status for each detected game
    for game in &mut games {
        check_hook_status(game);
    }

    // Sort alphabetically by name
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    games
}
