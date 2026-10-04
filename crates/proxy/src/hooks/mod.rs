pub mod focus;
pub mod input;

use crate::config::Config;
use crate::logger::log;
use minhook::MinHook;

pub fn initialize_hooks(config: &Config) {
    if !config.enabled {
        log("[HookManager] Disabled via focus_hook.ini; skipping hooks.");
        return;
    }

    if let Err(e) = focus::install_focus_hooks(config) {
        log(&format!("[HookManager] Failed to install focus hooks: {:?}", e));
    }

    input::install_input_hooks(config);

    if let Err(e) = unsafe { MinHook::enable_all_hooks() } {
        log(&format!("[HookManager] MinHook::enable_all_hooks error: {:?}", e));
    } else {
        log("[HookManager] All hooks enabled successfully.");
    }
}
