use parking_lot::Mutex;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

static LOG_ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn init_logger(enabled: bool) {
    LOG_ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        let _ = std::fs::remove_file("focus_hook.log");
        log("[FocusHook] Logger initialized");
    }
}

pub fn log(msg: &str) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let _lock = LOG_MUTEX.lock();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("focus_hook.log")
    {
        let _ = writeln!(file, "{}", msg);
    }
}
