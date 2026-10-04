use crate::config::Config;
use crate::logger::log;
use minhook::MinHook;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Once;
use windows_sys::Win32::Foundation::{HMODULE, HWND};
use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

static DINPUT_HOOKED: AtomicBool = AtomicBool::new(false);
static DINPUT_HOOK_ONCE: Once = Once::new();

const DISCL_EXCLUSIVE: u32 = 0x00000001;
const DISCL_NONEXCLUSIVE: u32 = 0x00000002;
const DISCL_FOREGROUND: u32 = 0x00000004;
const DISCL_BACKGROUND: u32 = 0x00000008;

type FnDirectInput8Create = unsafe extern "system" fn(
    *mut c_void,
    u32,
    *const c_void,
    *mut *mut c_void,
    *mut c_void,
) -> i32;

type FnCreateDevice = unsafe extern "system" fn(
    *mut c_void,
    *const c_void,
    *mut *mut c_void,
    *mut c_void,
) -> i32;

type FnSetCooperativeLevel = unsafe extern "system" fn(*mut c_void, HWND, u32) -> i32;

static ORIG_DINPUT8_CREATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_CREATE_DEVICE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_COOPERATIVE_LEVEL: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn hooked_set_cooperative_level(
    this: *mut c_void,
    hwnd: HWND,
    mut flags: u32,
) -> i32 {
    let old_flags = flags;
    if (flags & DISCL_FOREGROUND) != 0 {
        flags = (flags & !DISCL_FOREGROUND) | DISCL_BACKGROUND;
    }
    if (flags & DISCL_EXCLUSIVE) != 0 {
        flags = (flags & !DISCL_EXCLUSIVE) | DISCL_NONEXCLUSIVE;
    }

    log(&format!(
        "[InputHook] SetCooperativeLevel modified flags: {:#010X} -> {:#010X}",
        old_flags, flags
    ));

    let orig_addr = ORIG_SET_COOPERATIVE_LEVEL.load(Ordering::Acquire);
    if orig_addr != 0 {
        let orig: FnSetCooperativeLevel = std::mem::transmute(orig_addr);
        orig(this, hwnd, flags)
    } else {
        0
    }
}

unsafe extern "system" fn hooked_create_device(
    this: *mut c_void,
    rguid: *const c_void,
    lplp_device: *mut *mut c_void,
    punk_outer: *mut c_void,
) -> i32 {
    let orig_addr = ORIG_CREATE_DEVICE.load(Ordering::Acquire);
    let res = if orig_addr != 0 {
        let orig: FnCreateDevice = std::mem::transmute(orig_addr);
        orig(this, rguid, lplp_device, punk_outer)
    } else {
        -1
    };

    if res >= 0 && !lplp_device.is_null() && !(*lplp_device).is_null() {
        let device = *lplp_device;
        let vtable = *(device as *mut *mut *mut c_void);
        let set_coop_level_ptr = *vtable.add(13);

        if ORIG_SET_COOPERATIVE_LEVEL.load(Ordering::Acquire) == 0 {
            if let Ok(orig) =
                MinHook::create_hook(set_coop_level_ptr, hooked_set_cooperative_level as _)
            {
                let _ = MinHook::enable_hook(set_coop_level_ptr);
                ORIG_SET_COOPERATIVE_LEVEL.store(orig as usize, Ordering::Release);
                log("[InputHook] IDirectInputDevice8::SetCooperativeLevel hooked");
            }
        }
    }

    res
}

unsafe extern "system" fn hooked_direct_input8_create(
    hinst: *mut c_void,
    dw_version: u32,
    riidltf: *const c_void,
    ppv_out: *mut *mut c_void,
    punk_outer: *mut c_void,
) -> i32 {
    let orig_addr = ORIG_DINPUT8_CREATE.load(Ordering::Acquire);
    let res = if orig_addr != 0 {
        let orig: FnDirectInput8Create = std::mem::transmute(orig_addr);
        orig(hinst, dw_version, riidltf, ppv_out, punk_outer)
    } else {
        -1
    };

    if res >= 0 && !ppv_out.is_null() && !(*ppv_out).is_null() {
        let dinput = *ppv_out;
        let vtable = *(dinput as *mut *mut *mut c_void);
        let create_device_ptr = *vtable.add(3);

        if ORIG_CREATE_DEVICE.load(Ordering::Acquire) == 0 {
            if let Ok(orig) = MinHook::create_hook(create_device_ptr, hooked_create_device as _) {
                let _ = MinHook::enable_hook(create_device_ptr);
                ORIG_CREATE_DEVICE.store(orig as usize, Ordering::Release);
                log("[InputHook] IDirectInput8::CreateDevice hooked");
            }
        }
    }

    res
}

pub fn try_hook_dinput8() {
    if DINPUT_HOOKED.load(Ordering::Relaxed) {
        return;
    }

    unsafe {
        let hmodule: HMODULE = GetModuleHandleA(b"dinput8.dll\0".as_ptr());
        if hmodule.is_null() {
            return;
        }

        let proc = GetProcAddress(hmodule, b"DirectInput8Create\0".as_ptr());
        if let Some(target) = proc {
            DINPUT_HOOK_ONCE.call_once(|| {
                if let Ok(orig) =
                    MinHook::create_hook(target as _, hooked_direct_input8_create as _)
                {
                    let _ = MinHook::enable_hook(target as _);
                    ORIG_DINPUT8_CREATE.store(orig as usize, Ordering::Release);
                    DINPUT_HOOKED.store(true, Ordering::Relaxed);
                    log("[InputHook] DirectInput8Create successfully hooked");
                }
            });
        }
    }
}

pub fn install_input_hooks(config: &Config) {
    if !config.background_controller {
        return;
    }

    // 1. Set SDL2 / SDL3 environment hints
    unsafe {
        let joystick_hint: Vec<u16> = "SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS\0"
            .encode_utf16()
            .collect();
        let val: [u16; 2] = ['1' as u16, 0];
        let _ = SetEnvironmentVariableW(joystick_hint.as_ptr(), val.as_ptr());

        let audio_hint: Vec<u16> = "SDL_HINT_AUDIO_RESUME_ON_FOCUS_GAIN\0"
            .encode_utf16()
            .collect();
        let audio_val: [u16; 2] = ['0' as u16, 0];
        let _ = SetEnvironmentVariableW(audio_hint.as_ptr(), audio_val.as_ptr());
    }
    log("[InputHook] Set SDL background joystick and audio environment hints");

    // 2. DirectInput8 check
    try_hook_dinput8();
}
