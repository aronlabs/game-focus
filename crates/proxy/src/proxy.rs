use std::ffi::c_void;
use std::sync::Once;
use windows_sys::Win32::Foundation::{BOOL, HANDLE, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

pub type FnGetFileVersionInfoA = unsafe extern "system" fn(*const u8, u32, u32, *mut c_void) -> BOOL;
pub type FnGetFileVersionInfoByHandle = unsafe extern "system" fn(u32, HANDLE, *mut c_void, *mut u32) -> BOOL;
pub type FnGetFileVersionInfoExA = unsafe extern "system" fn(u32, *const u8, u32, u32, *mut c_void) -> BOOL;
pub type FnGetFileVersionInfoExW = unsafe extern "system" fn(u32, *const u16, u32, u32, *mut c_void) -> BOOL;
pub type FnGetFileVersionInfoSizeA = unsafe extern "system" fn(*const u8, *mut u32) -> u32;
pub type FnGetFileVersionInfoSizeExA = unsafe extern "system" fn(u32, *const u8, *mut u32) -> u32;
pub type FnGetFileVersionInfoSizeExW = unsafe extern "system" fn(u32, *const u16, *mut u32) -> u32;
pub type FnGetFileVersionInfoSizeW = unsafe extern "system" fn(*const u16, *mut u32) -> u32;
pub type FnGetFileVersionInfoW = unsafe extern "system" fn(*const u16, u32, u32, *mut c_void) -> BOOL;
pub type FnVerFindFileA = unsafe extern "system" fn(u32, *const u8, *const u8, *const u8, *mut u8, *mut u32, *mut u8, *mut u32) -> u32;
pub type FnVerFindFileW = unsafe extern "system" fn(u32, *const u16, *const u16, *const u16, *mut u16, *mut u32, *mut u16, *mut u32) -> u32;
pub type FnVerInstallFileA = unsafe extern "system" fn(u32, *const u8, *const u8, *const u8, *const u8, *const u8, *mut u8, *mut u32) -> u32;
pub type FnVerInstallFileW = unsafe extern "system" fn(u32, *const u16, *const u16, *const u16, *const u16, *const u16, *mut u16, *mut u32) -> u32;
pub type FnVerLanguageNameA = unsafe extern "system" fn(u32, *mut u8, u32) -> u32;
pub type FnVerLanguageNameW = unsafe extern "system" fn(u32, *mut u16, u32) -> u32;
pub type FnVerQueryValueA = unsafe extern "system" fn(*const c_void, *const u8, *mut *mut c_void, *mut u32) -> BOOL;
pub type FnVerQueryValueW = unsafe extern "system" fn(*const c_void, *const u16, *mut *mut c_void, *mut u32) -> BOOL;

#[derive(Default)]
pub struct VersionProcs {
    pub get_file_version_info_a: Option<FnGetFileVersionInfoA>,
    pub get_file_version_info_by_handle: Option<FnGetFileVersionInfoByHandle>,
    pub get_file_version_info_ex_a: Option<FnGetFileVersionInfoExA>,
    pub get_file_version_info_ex_w: Option<FnGetFileVersionInfoExW>,
    pub get_file_version_info_size_a: Option<FnGetFileVersionInfoSizeA>,
    pub get_file_version_info_size_ex_a: Option<FnGetFileVersionInfoSizeExA>,
    pub get_file_version_info_size_ex_w: Option<FnGetFileVersionInfoSizeExW>,
    pub get_file_version_info_size_w: Option<FnGetFileVersionInfoSizeW>,
    pub get_file_version_info_w: Option<FnGetFileVersionInfoW>,
    pub ver_find_file_a: Option<FnVerFindFileA>,
    pub ver_find_file_w: Option<FnVerFindFileW>,
    pub ver_install_file_a: Option<FnVerInstallFileA>,
    pub ver_install_file_w: Option<FnVerInstallFileW>,
    pub ver_language_name_a: Option<FnVerLanguageNameA>,
    pub ver_language_name_w: Option<FnVerLanguageNameW>,
    pub ver_query_value_a: Option<FnVerQueryValueA>,
    pub ver_query_value_w: Option<FnVerQueryValueW>,
}

static mut PROCS: VersionProcs = VersionProcs {
    get_file_version_info_a: None,
    get_file_version_info_by_handle: None,
    get_file_version_info_ex_a: None,
    get_file_version_info_ex_w: None,
    get_file_version_info_size_a: None,
    get_file_version_info_size_ex_a: None,
    get_file_version_info_size_ex_w: None,
    get_file_version_info_size_w: None,
    get_file_version_info_w: None,
    ver_find_file_a: None,
    ver_find_file_w: None,
    ver_install_file_a: None,
    ver_install_file_w: None,
    ver_language_name_a: None,
    ver_language_name_w: None,
    ver_query_value_a: None,
    ver_query_value_w: None,
};

static INIT_ONCE: Once = Once::new();

pub fn ensure_initialized() {
    INIT_ONCE.call_once(|| unsafe {
        load_real_version_dll();
    });
}

unsafe fn load_real_version_dll() {
    let mut sys_dir: [u16; 300] = [0; 300];
    let len = GetSystemDirectoryW(sys_dir.as_mut_ptr(), 260);
    if len == 0 {
        return;
    }

    let dll_name: [u16; 13] = [
        '\\' as u16, 'v' as u16, 'e' as u16, 'r' as u16, 's' as u16, 'i' as u16,
        'o' as u16, 'n' as u16, '.' as u16, 'd' as u16, 'l' as u16, 'l' as u16, 0,
    ];

    let mut full_path: Vec<u16> = sys_dir[..len as usize].to_vec();
    full_path.extend_from_slice(&dll_name);

    let module: HMODULE = LoadLibraryW(full_path.as_ptr());
    if module.is_null() {
        return;
    }

    macro_rules! resolve {
        ($field:ident, $name:literal, $t:ty) => {
            if let Some(proc) = GetProcAddress(module, concat!($name, "\0").as_ptr() as *const u8) {
                PROCS.$field = Some(std::mem::transmute::<_, $t>(proc));
            }
        };
    }

    resolve!(get_file_version_info_a, "GetFileVersionInfoA", FnGetFileVersionInfoA);
    resolve!(get_file_version_info_by_handle, "GetFileVersionInfoByHandle", FnGetFileVersionInfoByHandle);
    resolve!(get_file_version_info_ex_a, "GetFileVersionInfoExA", FnGetFileVersionInfoExA);
    resolve!(get_file_version_info_ex_w, "GetFileVersionInfoExW", FnGetFileVersionInfoExW);
    resolve!(get_file_version_info_size_a, "GetFileVersionInfoSizeA", FnGetFileVersionInfoSizeA);
    resolve!(get_file_version_info_size_ex_a, "GetFileVersionInfoSizeExA", FnGetFileVersionInfoSizeExA);
    resolve!(get_file_version_info_size_ex_w, "GetFileVersionInfoSizeExW", FnGetFileVersionInfoSizeExW);
    resolve!(get_file_version_info_size_w, "GetFileVersionInfoSizeW", FnGetFileVersionInfoSizeW);
    resolve!(get_file_version_info_w, "GetFileVersionInfoW", FnGetFileVersionInfoW);
    resolve!(ver_find_file_a, "VerFindFileA", FnVerFindFileA);
    resolve!(ver_find_file_w, "VerFindFileW", FnVerFindFileW);
    resolve!(ver_install_file_a, "VerInstallFileA", FnVerInstallFileA);
    resolve!(ver_install_file_w, "VerInstallFileW", FnVerInstallFileW);
    resolve!(ver_language_name_a, "VerLanguageNameA", FnVerLanguageNameA);
    resolve!(ver_language_name_w, "VerLanguageNameW", FnVerLanguageNameW);
    resolve!(ver_query_value_a, "VerQueryValueA", FnVerQueryValueA);
    resolve!(ver_query_value_w, "VerQueryValueW", FnVerQueryValueW);
}

// Exported forwarders
#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoA(
    p1: *const u8,
    p2: u32,
    p3: u32,
    p4: *mut c_void,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_a {
        f(p1, p2, p3, p4)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoByHandle(
    p1: u32,
    p2: HANDLE,
    p3: *mut c_void,
    p4: *mut u32,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_by_handle {
        f(p1, p2, p3, p4)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoExA(
    p1: u32,
    p2: *const u8,
    p3: u32,
    p4: u32,
    p5: *mut c_void,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_ex_a {
        f(p1, p2, p3, p4, p5)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoExW(
    p1: u32,
    p2: *const u16,
    p3: u32,
    p4: u32,
    p5: *mut c_void,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_ex_w {
        f(p1, p2, p3, p4, p5)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoSizeA(p1: *const u8, p2: *mut u32) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_size_a {
        f(p1, p2)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoSizeExA(p1: u32, p2: *const u8, p3: *mut u32) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_size_ex_a {
        f(p1, p2, p3)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoSizeExW(p1: u32, p2: *const u16, p3: *mut u32) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_size_ex_w {
        f(p1, p2, p3)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoSizeW(p1: *const u16, p2: *mut u32) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_size_w {
        f(p1, p2)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn GetFileVersionInfoW(
    p1: *const u16,
    p2: u32,
    p3: u32,
    p4: *mut c_void,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.get_file_version_info_w {
        f(p1, p2, p3, p4)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerFindFileA(
    p1: u32,
    p2: *const u8,
    p3: *const u8,
    p4: *const u8,
    p5: *mut u8,
    p6: *mut u32,
    p7: *mut u8,
    p8: *mut u32,
) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.ver_find_file_a {
        f(p1, p2, p3, p4, p5, p6, p7, p8)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerFindFileW(
    p1: u32,
    p2: *const u16,
    p3: *const u16,
    p4: *const u16,
    p5: *mut u16,
    p6: *mut u32,
    p7: *mut u16,
    p8: *mut u32,
) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.ver_find_file_w {
        f(p1, p2, p3, p4, p5, p6, p7, p8)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerInstallFileA(
    p1: u32,
    p2: *const u8,
    p3: *const u8,
    p4: *const u8,
    p5: *const u8,
    p6: *const u8,
    p7: *mut u8,
    p8: *mut u32,
) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.ver_install_file_a {
        f(p1, p2, p3, p4, p5, p6, p7, p8)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerInstallFileW(
    p1: u32,
    p2: *const u16,
    p3: *const u16,
    p4: *const u16,
    p5: *const u16,
    p6: *const u16,
    p7: *mut u16,
    p8: *mut u32,
) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.ver_install_file_w {
        f(p1, p2, p3, p4, p5, p6, p7, p8)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerLanguageNameA(p1: u32, p2: *mut u8, p3: u32) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.ver_language_name_a {
        f(p1, p2, p3)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerLanguageNameW(p1: u32, p2: *mut u16, p3: u32) -> u32 {
    ensure_initialized();
    if let Some(f) = PROCS.ver_language_name_w {
        f(p1, p2, p3)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerQueryValueA(
    p1: *const c_void,
    p2: *const u8,
    p3: *mut *mut c_void,
    p4: *mut u32,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.ver_query_value_a {
        f(p1, p2, p3, p4)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "system" fn VerQueryValueW(
    p1: *const c_void,
    p2: *const u16,
    p3: *mut *mut c_void,
    p4: *mut u32,
) -> BOOL {
    ensure_initialized();
    if let Some(f) = PROCS.ver_query_value_w {
        f(p1, p2, p3, p4)
    } else {
        0
    }
}
