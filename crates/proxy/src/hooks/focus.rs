use crate::config::Config;
use crate::logger::log;
use minhook::MinHook;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use windows_sys::Win32::Foundation::{BOOL, HWND, RECT};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, MSG, WM_ACTIVATE, WM_ACTIVATEAPP, WM_KILLFOCUS, WM_NCACTIVATE,
    WM_NULL,
};

static GAME_HWND: AtomicIsize = AtomicIsize::new(0);
static UNLOCK_CURSOR: AtomicBool = AtomicBool::new(false);
static SPOOF_FOCUS: AtomicBool = AtomicBool::new(true);

type FnGetForegroundWindow = unsafe extern "system" fn() -> HWND;
type FnGetActiveWindow = unsafe extern "system" fn() -> HWND;
type FnGetFocus = unsafe extern "system" fn() -> HWND;
type FnPeekMessageW = unsafe extern "system" fn(*mut MSG, HWND, u32, u32, u32) -> BOOL;
type FnPeekMessageA = unsafe extern "system" fn(*mut MSG, HWND, u32, u32, u32) -> BOOL;
type FnGetMessageW = unsafe extern "system" fn(*mut MSG, HWND, u32, u32) -> BOOL;
type FnGetMessageA = unsafe extern "system" fn(*mut MSG, HWND, u32, u32) -> BOOL;
type FnClipCursor = unsafe extern "system" fn(*const RECT) -> BOOL;

static ORIG_GET_FOREGROUND_WINDOW: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_ACTIVE_WINDOW: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_FOCUS: AtomicUsize = AtomicUsize::new(0);
static ORIG_PEEK_MESSAGE_W: AtomicUsize = AtomicUsize::new(0);
static ORIG_PEEK_MESSAGE_A: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_MESSAGE_W: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_MESSAGE_A: AtomicUsize = AtomicUsize::new(0);
static ORIG_CLIP_CURSOR: AtomicUsize = AtomicUsize::new(0);

#[inline]
unsafe fn check_and_update_hwnd(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let current_stored = GAME_HWND.load(Ordering::Relaxed) as HWND;
    if current_stored.is_null() {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId() {
            GAME_HWND.store(hwnd as isize, Ordering::Relaxed);
            log(&format!("[FocusHook] Captured game window HWND: {:p}", hwnd));
        }
    }
}

#[inline]
unsafe fn filter_msg(msg: *mut MSG) {
    if msg.is_null() {
        return;
    }
    let m = &mut *msg;
    check_and_update_hwnd(m.hwnd);

    if !SPOOF_FOCUS.load(Ordering::Relaxed) {
        return;
    }

    const WA_ACTIVE: usize = 1;

    match m.message {
        WM_ACTIVATE => {
            let loword = (m.wParam & 0xFFFF) as usize;
            if loword == 0 {
                // WA_INACTIVE -> WA_ACTIVE
                m.wParam = WA_ACTIVE | (m.wParam & !0xFFFF);
                log("[FocusHook] Intercepted WM_ACTIVATE (WA_INACTIVE -> WA_ACTIVE)");
            }
        }
        WM_ACTIVATEAPP => {
            if m.wParam == 0 {
                // Deactivated -> Active
                m.wParam = 1;
                log("[FocusHook] Intercepted WM_ACTIVATEAPP (Deactivate -> Active)");
            }
        }
        WM_KILLFOCUS => {
            m.message = WM_NULL;
            log("[FocusHook] Intercepted WM_KILLFOCUS (suppressed to WM_NULL)");
        }
        WM_NCACTIVATE => {
            if m.wParam == 0 {
                m.wParam = 1;
                log("[FocusHook] Intercepted WM_NCACTIVATE (forcing title bar active)");
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn hooked_get_foreground_window() -> HWND {
    if SPOOF_FOCUS.load(Ordering::Relaxed) {
        let hwnd = GAME_HWND.load(Ordering::Relaxed) as HWND;
        if !hwnd.is_null() {
            return hwnd;
        }
    }
    let addr = ORIG_GET_FOREGROUND_WINDOW.load(Ordering::Acquire);
    if addr != 0 {
        let orig: FnGetForegroundWindow = std::mem::transmute(addr);
        orig()
    } else {
        std::ptr::null_mut()
    }
}

unsafe extern "system" fn hooked_get_active_window() -> HWND {
    if SPOOF_FOCUS.load(Ordering::Relaxed) {
        let hwnd = GAME_HWND.load(Ordering::Relaxed) as HWND;
        if !hwnd.is_null() {
            return hwnd;
        }
    }
    let addr = ORIG_GET_ACTIVE_WINDOW.load(Ordering::Acquire);
    if addr != 0 {
        let orig: FnGetActiveWindow = std::mem::transmute(addr);
        orig()
    } else {
        std::ptr::null_mut()
    }
}

unsafe extern "system" fn hooked_get_focus() -> HWND {
    if SPOOF_FOCUS.load(Ordering::Relaxed) {
        let hwnd = GAME_HWND.load(Ordering::Relaxed) as HWND;
        if !hwnd.is_null() {
            return hwnd;
        }
    }
    let addr = ORIG_GET_FOCUS.load(Ordering::Acquire);
    if addr != 0 {
        let orig: FnGetFocus = std::mem::transmute(addr);
        orig()
    } else {
        std::ptr::null_mut()
    }
}

unsafe extern "system" fn hooked_peek_message_w(
    lpmsg: *mut MSG,
    hwnd: HWND,
    wmsgfiltermin: u32,
    wmsgfiltermax: u32,
    wremovemsg: u32,
) -> BOOL {
    let addr = ORIG_PEEK_MESSAGE_W.load(Ordering::Acquire);
    let result = if addr != 0 {
        let orig: FnPeekMessageW = std::mem::transmute(addr);
        orig(lpmsg, hwnd, wmsgfiltermin, wmsgfiltermax, wremovemsg)
    } else {
        0
    };

    if result != 0 {
        filter_msg(lpmsg);
    }
    result
}

unsafe extern "system" fn hooked_peek_message_a(
    lpmsg: *mut MSG,
    hwnd: HWND,
    wmsgfiltermin: u32,
    wmsgfiltermax: u32,
    wremovemsg: u32,
) -> BOOL {
    let addr = ORIG_PEEK_MESSAGE_A.load(Ordering::Acquire);
    let result = if addr != 0 {
        let orig: FnPeekMessageA = std::mem::transmute(addr);
        orig(lpmsg, hwnd, wmsgfiltermin, wmsgfiltermax, wremovemsg)
    } else {
        0
    };

    if result != 0 {
        filter_msg(lpmsg);
    }
    result
}

unsafe extern "system" fn hooked_get_message_w(
    lpmsg: *mut MSG,
    hwnd: HWND,
    wmsgfiltermin: u32,
    wmsgfiltermax: u32,
) -> BOOL {
    let addr = ORIG_GET_MESSAGE_W.load(Ordering::Acquire);
    let result = if addr != 0 {
        let orig: FnGetMessageW = std::mem::transmute(addr);
        orig(lpmsg, hwnd, wmsgfiltermin, wmsgfiltermax)
    } else {
        0
    };

    if result > 0 {
        filter_msg(lpmsg);
    }
    result
}

unsafe extern "system" fn hooked_get_message_a(
    lpmsg: *mut MSG,
    hwnd: HWND,
    wmsgfiltermin: u32,
    wmsgfiltermax: u32,
) -> BOOL {
    let addr = ORIG_GET_MESSAGE_A.load(Ordering::Acquire);
    let result = if addr != 0 {
        let orig: FnGetMessageA = std::mem::transmute(addr);
        orig(lpmsg, hwnd, wmsgfiltermin, wmsgfiltermax)
    } else {
        0
    };

    if result > 0 {
        filter_msg(lpmsg);
    }
    result
}

unsafe extern "system" fn hooked_clip_cursor(lp_rect: *const RECT) -> BOOL {
    if UNLOCK_CURSOR.load(Ordering::Relaxed) {
        // Return success without constraining cursor
        log("[FocusHook] ClipCursor bypassed (cursor unlocked)");
        return 1;
    }
    let addr = ORIG_CLIP_CURSOR.load(Ordering::Acquire);
    if addr != 0 {
        let orig: FnClipCursor = std::mem::transmute(addr);
        orig(lp_rect)
    } else {
        1
    }
}

pub fn install_focus_hooks(config: &Config) -> Result<(), minhook::MH_STATUS> {
    SPOOF_FOCUS.store(config.spoof_focus, Ordering::Relaxed);
    UNLOCK_CURSOR.store(config.unlock_cursor, Ordering::Relaxed);

    unsafe {
        // 1. GetForegroundWindow
        let orig = MinHook::create_hook_api(
            "user32.dll",
            "GetForegroundWindow",
            hooked_get_foreground_window as _,
        )?;
        ORIG_GET_FOREGROUND_WINDOW.store(orig as usize, Ordering::Release);

        // 2. GetActiveWindow
        let orig = MinHook::create_hook_api(
            "user32.dll",
            "GetActiveWindow",
            hooked_get_active_window as _,
        )?;
        ORIG_GET_ACTIVE_WINDOW.store(orig as usize, Ordering::Release);

        // 3. GetFocus
        let orig = MinHook::create_hook_api("user32.dll", "GetFocus", hooked_get_focus as _)?;
        ORIG_GET_FOCUS.store(orig as usize, Ordering::Release);

        // 4. PeekMessageW & PeekMessageA
        let orig = MinHook::create_hook_api("user32.dll", "PeekMessageW", hooked_peek_message_w as _)?;
        ORIG_PEEK_MESSAGE_W.store(orig as usize, Ordering::Release);

        let orig = MinHook::create_hook_api("user32.dll", "PeekMessageA", hooked_peek_message_a as _)?;
        ORIG_PEEK_MESSAGE_A.store(orig as usize, Ordering::Release);

        // 5. GetMessageW & GetMessageA
        let orig = MinHook::create_hook_api("user32.dll", "GetMessageW", hooked_get_message_w as _)?;
        ORIG_GET_MESSAGE_W.store(orig as usize, Ordering::Release);

        let orig = MinHook::create_hook_api("user32.dll", "GetMessageA", hooked_get_message_a as _)?;
        ORIG_GET_MESSAGE_A.store(orig as usize, Ordering::Release);

        // 6. ClipCursor
        let orig = MinHook::create_hook_api("user32.dll", "ClipCursor", hooked_clip_cursor as _)?;
        ORIG_CLIP_CURSOR.store(orig as usize, Ordering::Release);

        log("[FocusHook] Focus & message loop hooks successfully installed");
    }

    Ok(())
}
