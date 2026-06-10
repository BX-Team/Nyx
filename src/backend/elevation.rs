#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use std::mem;
    use std::ptr;
    unsafe {
        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        #[link(name = "advapi32")]
        extern "system" {
            fn OpenProcessToken(
                process: *mut std::ffi::c_void,
                desired_access: u32,
                token_handle: *mut *mut std::ffi::c_void,
            ) -> i32;
            fn GetTokenInformation(
                token_handle: *mut std::ffi::c_void,
                token_information_class: u32,
                token_information: *mut std::ffi::c_void,
                token_information_length: u32,
                return_length: *mut u32,
            ) -> i32;
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }

        const TOKEN_QUERY: u32 = 0x0008;
        const TOKEN_ELEVATION: u32 = 20;

        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        #[repr(C)]
        struct TokenElevation {
            token_is_elevated: u32,
        }
        let mut elevation: TokenElevation = mem::zeroed();
        let mut size: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            mem::size_of::<TokenElevation>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != 0 && elevation.token_is_elevated != 0
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_elevated() -> bool {
    unsafe { libc::geteuid() == 0 }
}
