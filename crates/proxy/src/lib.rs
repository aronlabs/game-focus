pub mod config;
pub mod logger;

#[cfg(windows)]
pub mod hooks;
#[cfg(windows)]
pub mod proxy;

#[cfg(windows)]
pub use proxy::*;

#[cfg(windows)]
use config::Config;
#[cfg(windows)]
use hooks::initialize_hooks;
#[cfg(windows)]
use hooks::focus::probe_and_subclass_existing_window;
#[cfg(windows)]
use hooks::input::try_hook_dinput8;
#[cfg(windows)]
use logger::{init_logger, log};
#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{BOOL, HINSTANCE};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
#[cfg(windows)]
use windows_sys::Win32::System::SystemServices::{
    DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH,
};

#[cfg(windows)]
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lp_reserved: *mut c_void,
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            DisableThreadLibraryCalls(hinst_dll);

            // Spawn initialization thread to prevent DllMain loader-lock deadlocks
            thread::spawn(|| {
                // Brief yield so host process finishes early loader tasks
                thread::sleep(Duration::from_millis(50));

                let config = Config::load_or_create();
                init_logger(config.log_debug);
                log("[FocusHook] Starting game background focus hook...");

                initialize_hooks(&config);

                // For the first 10 seconds of process life, periodically probe for window and dinput8
                if config.enabled {
                    for _ in 0..5 {
                        thread::sleep(Duration::from_secs(2));
                        probe_and_subclass_existing_window();
                        if config.background_controller {
                            try_hook_dinput8();
                        }
                    }
                }
            });
        }
        DLL_PROCESS_DETACH => {
            log("[FocusHook] Detaching DLL");
            minhook::MinHook::uninitialize();
        }
        _ => {}
    }

    1 // TRUE
}
