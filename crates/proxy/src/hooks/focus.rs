use crate::config::Config;
use crate::logger::log;
use minhook::MinHook;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use windows_sys::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, RECT};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CreateWindowExA, CreateWindowExW, EnumWindows,
    GetWindowThreadProcessId, SetWindowLongPtrW, GWLP_WNDPROC, HMENU, MSG, WM_ACTIVATE,
    WM_ACTIVATEAPP, WM_KILLFOCUS, WM_NCACTIVATE, WM_NULL,
};

static GAME_HWND: AtomicIsize = AtomicIsize::new(0);
static ORIG_WND_PROC: AtomicUsize = AtomicUsize::new(0);
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

type FnCreateWindowExW = unsafe extern "system" fn(
    u32,
    *const u16,
    *const u16,
    u32,
    i32,
    i32,
    i32,
    i32,
    HWND,
    HMENU,
    HINSTANCE,
    *mut c_void,
) -> HWND;

type FnCreateWindowExA = unsafe extern "system" fn(
    u32,
    *const u8,
    *const u8,
    u32,
    i32,
    i32,
    i32,
    i32,
    HWND,
    HMENU,
    HINSTANCE,
    *mut c_void,
) -> HWND;

static ORIG_GET_FOREGROUND_WINDOW: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_ACTIVE_WINDOW: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_FOCUS: AtomicUsize = AtomicUsize::new(0);
static ORIG_PEEK_MESSAGE_W: AtomicUsize = AtomicUsize::new(0);
static ORIG_PEEK_MESSAGE_A: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_MESSAGE_W: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_MESSAGE_A: AtomicUsize = AtomicUsize::new(0);
static ORIG_CLIP_CURSOR: AtomicUsize = AtomicUsize::new(0);
static ORIG_CREATE_WINDOW_EX_W: AtomicUsize = AtomicUsize::new(0);
static ORIG_CREATE_WINDOW_EX_A: AtomicUsize = AtomicUsize::new(0);

pub unsafe extern "system" fn hooked_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    const WA_ACTIVE: usize = 1;

    let orig_proc_addr = ORIG_WND_PROC.load(Ordering::Acquire);
    if orig_proc_addr == 0 {
        return 0;
    }

    if SPOOF_FOCUS.load(Ordering::Relaxed) {
        match msg {
            WM_ACTIVATE => {
                let loword = (wparam & 0xFFFF) as usize;
                if loword == 0 {
                    // WA_INACTIVE -> WA_ACTIVE
                    let spoofed = WA_ACTIVE | (wparam & !0xFFFF);
                    log("[FocusHook] WndProc: intercepted WM_ACTIVATE (WA_INACTIVE -> WA_ACTIVE)");
                    return CallWindowProcW(
                        std::mem::transmute(orig_proc_addr),
                        hwnd,
                        msg,
                        spoofed,
                        lparam,
                    );
                }
            }
            WM_ACTIVATEAPP => {
                if wparam == 0 {
                    log("[FocusHook] WndProc: intercepted WM_ACTIVATEAPP (0 -> 1)");
                    return CallWindowProcW(
                        std::mem::transmute(orig_proc_addr),
                        hwnd,
                        msg,
                        1,
                        lparam,
                    );
                }
            }
            WM_KILLFOCUS => {
                log("[FocusHook] WndProc: intercepted WM_KILLFOCUS (suppressed)");
                return 0;
            }
            WM_NCACTIVATE => {
                if wparam == 0 {
                    log("[FocusHook] WndProc: intercepted WM_NCACTIVATE (forcing title active)");
                    return CallWindowProcW(
                        std::mem::transmute(orig_proc_addr),
                        hwnd,
                        msg,
                        1,
                        lparam,
                    );
                }
            }
            _ => {}
        }
    }

    CallWindowProcW(
        std::mem::transmute(orig_proc_addr),
        hwnd,
        msg,
        wparam,
        lparam,
    )
}

pub unsafe fn subclass_window(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let current = GAME_HWND.load(Ordering::Relaxed) as HWND;
    if !current.is_null() && current == hwnd {
        return;
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid != GetCurrentProcessId() {
        return;
    }

    GAME_HWND.store(hwnd as isize, Ordering::Release);

    let old_proc = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, hooked_wnd_proc as *const () as isize);
    if old_proc != 0 && old_proc != (hooked_wnd_proc as *const () as isize) {
        ORIG_WND_PROC.store(old_proc as usize, Ordering::Release);
        log(&format!(
            "[FocusHook] Subclassed window HWND: {:p} (Old WndProc: {:#X})",
            hwnd, old_proc
        ));
    }
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        subclass_window(hwnd);
        return 0; // stop enumerating
    }
    1 // continue
}

unsafe extern "system" fn hooked_create_window_ex_w(
    dwexstyle: u32,
    lpclassname: *const u16,
    lpwindowname: *const u16,
    dwstyle: u32,
    x: i32,
    y: i32,
    nwidth: i32,
    nheight: i32,
    hwndparent: HWND,
    hmenu: HMENU,
    hinstance: HINSTANCE,
    lpparam: *mut c_void,
) -> HWND {
    let addr = ORIG_CREATE_WINDOW_EX_W.load(Ordering::Acquire);
    let hwnd = if addr != 0 {
        let orig: FnCreateWindowExW = std::mem::transmute(addr);
        orig(
            dwexstyle,
            lpclassname,
            lpwindowname,
            dwstyle,
            x,
            y,
            nwidth,
            nheight,
            hwndparent,
            hmenu,
            hinstance,
            lpparam,
        )
    } else {
        CreateWindowExW(
            dwexstyle,
            lpclassname,
            lpwindowname,
            dwstyle,
            x,
            y,
            nwidth,
            nheight,
            hwndparent,
            hmenu,
            hinstance,
            lpparam,
        )
    };

    if !hwnd.is_null() && hwndparent.is_null() {
        subclass_window(hwnd);
    }

    hwnd
}

unsafe extern "system" fn hooked_create_window_ex_a(
    dwexstyle: u32,
    lpclassname: *const u8,
    lpwindowname: *const u8,
    dwstyle: u32,
    x: i32,
    y: i32,
    nwidth: i32,
    nheight: i32,
    hwndparent: HWND,
    hmenu: HMENU,
    hinstance: HINSTANCE,
    lpparam: *mut c_void,
) -> HWND {
    let addr = ORIG_CREATE_WINDOW_EX_A.load(Ordering::Acquire);
    let hwnd = if addr != 0 {
        let orig: FnCreateWindowExA = std::mem::transmute(addr);
        orig(
            dwexstyle,
            lpclassname,
            lpwindowname,
            dwstyle,
            x,
            y,
            nwidth,
            nheight,
            hwndparent,
            hmenu,
            hinstance,
            lpparam,
        )
    } else {
        CreateWindowExA(
            dwexstyle,
            lpclassname,
            lpwindowname,
            dwstyle,
            x,
            y,
            nwidth,
            nheight,
            hwndparent,
            hmenu,
            hinstance,
            lpparam,
        )
    };

    if !hwnd.is_null() && hwndparent.is_null() {
        subclass_window(hwnd);
    }

    hwnd
}

#[inline]
unsafe fn filter_msg(msg: *mut MSG) {
    if msg.is_null() {
        return;
    }
    let m = &mut *msg;
    if !m.hwnd.is_null() {
        subclass_window(m.hwnd);
    }

    if !SPOOF_FOCUS.load(Ordering::Relaxed) {
        return;
    }

    const WA_ACTIVE: usize = 1;

    match m.message {
        WM_ACTIVATE => {
            let loword = (m.wParam & 0xFFFF) as usize;
            if loword == 0 {
                m.wParam = WA_ACTIVE | (m.wParam & !0xFFFF);
            }
        }
        WM_ACTIVATEAPP => {
            if m.wParam == 0 {
                m.wParam = 1;
            }
        }
        WM_KILLFOCUS => {
            m.message = WM_NULL;
        }
        WM_NCACTIVATE => {
            if m.wParam == 0 {
                m.wParam = 1;
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

pub fn probe_and_subclass_existing_window() {
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), 0);
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

        // 4. CreateWindowExW & CreateWindowExA (catch window creation immediately)
        if let Ok(orig) = MinHook::create_hook_api(
            "user32.dll",
            "CreateWindowExW",
            hooked_create_window_ex_w as _,
        ) {
            ORIG_CREATE_WINDOW_EX_W.store(orig as usize, Ordering::Release);
        }

        if let Ok(orig) = MinHook::create_hook_api(
            "user32.dll",
            "CreateWindowExA",
            hooked_create_window_ex_a as _,
        ) {
            ORIG_CREATE_WINDOW_EX_A.store(orig as usize, Ordering::Release);
        }

        // 5. PeekMessageW & PeekMessageA
        let orig = MinHook::create_hook_api("user32.dll", "PeekMessageW", hooked_peek_message_w as _)?;
        ORIG_PEEK_MESSAGE_W.store(orig as usize, Ordering::Release);

        let orig = MinHook::create_hook_api("user32.dll", "PeekMessageA", hooked_peek_message_a as _)?;
        ORIG_PEEK_MESSAGE_A.store(orig as usize, Ordering::Release);

        // 6. GetMessageW & GetMessageA
        let orig = MinHook::create_hook_api("user32.dll", "GetMessageW", hooked_get_message_w as _)?;
        ORIG_GET_MESSAGE_W.store(orig as usize, Ordering::Release);

        let orig = MinHook::create_hook_api("user32.dll", "GetMessageA", hooked_get_message_a as _)?;
        ORIG_GET_MESSAGE_A.store(orig as usize, Ordering::Release);

        // 7. ClipCursor
        let orig = MinHook::create_hook_api("user32.dll", "ClipCursor", hooked_clip_cursor as _)?;
        ORIG_CLIP_CURSOR.store(orig as usize, Ordering::Release);

        // Check for any window created before hook attached
        probe_and_subclass_existing_window();

        log("[FocusHook] Focus, CreateWindow, and WndProc hooks installed");
    }

    Ok(())
}
