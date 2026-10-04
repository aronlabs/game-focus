use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub enabled: bool,
    pub spoof_focus: bool,
    pub keep_audio: bool,
    pub background_controller: bool,
    pub unlock_cursor: bool,
    pub log_debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            spoof_focus: true,
            keep_audio: true,
            background_controller: true,
            unlock_cursor: false,
            log_debug: false,
        }
    }
}

pub const DEFAULT_INI: &str = r#"; Game Focus & Background Controller Configuration
[Settings]
; Set to false to disable all hooks entirely
enabled = true

; Trick game into believing it is always the active foreground window
spoof_focus = true

; Prevent game from muting audio when losing focus
keep_audio = true

; Allow gamepads (XInput/DInput/SDL) to function in background
background_controller = true

; Release mouse cursor bounds so it can leave the window freely (ClipCursor bypass)
unlock_cursor = false

; Write debug logs to focus_hook.log
log_debug = false
"#;

impl Config {
    pub fn parse_str(content: &str) -> Self {
        let mut config = Self::default();
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
                    "enabled" => config.enabled = bool_val,
                    "spoof_focus" => config.spoof_focus = bool_val,
                    "keep_audio" => config.keep_audio = bool_val,
                    "background_controller" => config.background_controller = bool_val,
                    "unlock_cursor" => config.unlock_cursor = bool_val,
                    "log_debug" => config.log_debug = bool_val,
                    _ => {}
                }
            }
        }
        config
    }

    pub fn load_or_create() -> Self {
        let path = Path::new("focus_hook.ini");
        if !path.exists() {
            let _ = fs::write(path, DEFAULT_INI);
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(c) => Self::parse_str(&c),
            Err(_) => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert!(cfg.enabled);
        assert!(cfg.spoof_focus);
        assert!(cfg.keep_audio);
        assert!(cfg.background_controller);
        assert!(!cfg.unlock_cursor);
        assert!(!cfg.log_debug);
    }

    #[test]
    fn test_parse_custom_ini() {
        let ini = r#"
        [Settings]
        enabled = false
        spoof_focus = true
        keep_audio = 1
        background_controller = yes
        unlock_cursor = true
        log_debug = true
        "#;
        let cfg = Config::parse_str(ini);
        assert!(!cfg.enabled);
        assert!(cfg.spoof_focus);
        assert!(cfg.keep_audio);
        assert!(cfg.background_controller);
        assert!(cfg.unlock_cursor);
        assert!(cfg.log_debug);
    }

    #[test]
    fn test_parse_default_ini_matches_default_struct() {
        let parsed = Config::parse_str(DEFAULT_INI);
        assert_eq!(parsed, Config::default());
    }
}
