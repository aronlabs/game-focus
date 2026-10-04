use crate::game::GameProfile;
use std::fs;
use std::io;

pub fn check_hook_status(profile: &mut GameProfile) {
    let dll_path = profile.exe_dir.join("version.dll");
    profile.enabled = dll_path.is_file();

    let ini_path = profile.exe_dir.join("focus_hook.ini");
    if ini_path.is_file() {
        if let Ok(content) = fs::read_to_string(&ini_path) {
            parse_and_apply_ini(&content, profile);
        }
    }
}

fn parse_and_apply_ini(content: &str, profile: &mut GameProfile) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim().to_lowercase();
            let val = val.trim().to_lowercase();
            let bool_val = val == "true" || val == "1" || val == "yes";

            match key.as_str() {
                "spoof_focus" => profile.spoof_focus = bool_val,
                "keep_audio" => profile.keep_audio = bool_val,
                "background_controller" => profile.background_controller = bool_val,
                "unlock_cursor" => profile.unlock_cursor = bool_val,
                _ => {}
            }
        }
    }
}

pub fn generate_ini(profile: &GameProfile) -> String {
    format!(
        r#"; Game Focus & Background Controller Configuration
[Settings]
enabled = {}
spoof_focus = {}
keep_audio = {}
background_controller = {}
unlock_cursor = {}
log_debug = false
"#,
        profile.enabled,
        profile.spoof_focus,
        profile.keep_audio,
        profile.background_controller,
        profile.unlock_cursor
    )
}

pub fn deploy_hook(profile: &GameProfile, dll_bytes: &[u8]) -> io::Result<()> {
    if !profile.exe_dir.exists() {
        fs::create_dir_all(&profile.exe_dir)?;
    }

    let dll_path = profile.exe_dir.join("version.dll");
    fs::write(&dll_path, dll_bytes)?;

    let ini_path = profile.exe_dir.join("focus_hook.ini");
    let ini_content = generate_ini(profile);
    fs::write(&ini_path, &ini_content)?;

    // If executable is nested (e.g. Unreal Engine Binaries/Win64), also copy to install_dir
    if profile.install_dir.is_dir() && profile.install_dir != profile.exe_dir {
        let _ = fs::write(profile.install_dir.join("version.dll"), dll_bytes);
        let _ = fs::write(profile.install_dir.join("focus_hook.ini"), &ini_content);
    }

    Ok(())
}

pub fn remove_hook(profile: &GameProfile) -> io::Result<()> {
    let dll_path = profile.exe_dir.join("version.dll");
    if dll_path.exists() {
        fs::remove_file(&dll_path)?;
    }
    if profile.install_dir.is_dir() && profile.install_dir != profile.exe_dir {
        let root_dll = profile.install_dir.join("version.dll");
        if root_dll.exists() {
            let _ = fs::remove_file(root_dll);
        }
    }
    Ok(())
}

pub fn update_config(profile: &GameProfile) -> io::Result<()> {
    let ini_content = generate_ini(profile);
    let ini_path = profile.exe_dir.join("focus_hook.ini");
    fs::write(&ini_path, &ini_content)?;

    if profile.install_dir.is_dir() && profile.install_dir != profile.exe_dir {
        let _ = fs::write(profile.install_dir.join("focus_hook.ini"), &ini_content);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Platform;
    use std::path::PathBuf;

    #[test]
    fn test_deploy_and_remove() {
        let temp_dir = std::env::temp_dir().join("game_focus_test_deploy");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut profile = GameProfile::new(
            "test_1".to_string(),
            "Test Game".to_string(),
            Platform::Custom,
            temp_dir.clone(),
            temp_dir.join("game.exe"),
        );
        profile.enabled = true;
        profile.unlock_cursor = true;

        let fake_dll = b"MZ\x90\x00test";
        deploy_hook(&profile, fake_dll).unwrap();

        assert!(temp_dir.join("version.dll").is_file());
        assert!(temp_dir.join("focus_hook.ini").is_file());

        let mut check_profile = profile.clone();
        check_hook_status(&mut check_profile);
        assert!(check_profile.enabled);
        assert!(check_profile.unlock_cursor);

        remove_hook(&profile).unwrap();
        assert!(!temp_dir.join("version.dll").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
