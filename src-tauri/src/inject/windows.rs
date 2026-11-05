use crate::inject::FrontmostApp;
use std::path::Path;
use std::time::Instant;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, HANDLE, RECT};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId, SM_CXSCREEN,
};

pub fn check_accessibility_permission() -> bool {
    true
}

pub fn is_secure_event_input_enabled() -> bool {
    false
}

pub fn open_accessibility_settings() -> Result<(), String> {
    Ok(())
}

pub fn detect_frontmost_app() -> FrontmostApp {
    let started = Instant::now();
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return FrontmostApp {
                name: "Windows App".to_string(),
                bundle_id: None,
                exe: None,
                pid: None,
                detection_us: started.elapsed().as_micros() as u64,
                window_center_x: None,
                primary_screen_width: None,
            };
        }

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let window_center_x = if GetWindowRect(hwnd, &mut rect) != 0 {
            Some(rect.left + (rect.right - rect.left) / 2)
        } else {
            None
        };
        let cx = GetSystemMetrics(SM_CXSCREEN);
        let primary_screen_width = if cx > 0 { Some(cx as u32) } else { None };

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return FrontmostApp {
                name: "Windows App".to_string(),
                bundle_id: None,
                exe: None,
                pid: None,
                detection_us: started.elapsed().as_micros() as u64,
                window_center_x,
                primary_screen_width,
            };
        }

        let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if proc_handle.is_null() {
            return FrontmostApp {
                name: "Windows App".to_string(),
                bundle_id: None,
                exe: None,
                pid: Some(pid),
                detection_us: started.elapsed().as_micros() as u64,
                window_center_x,
                primary_screen_width,
            };
        }

        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(proc_handle, 0, buf.as_mut_ptr(), &mut size);
        let _ = CloseHandle(proc_handle);

        if ok != 0 && size > 0 {
            let full_path = String::from_utf16_lossy(&buf[..size as usize]);
            let path = Path::new(&full_path);
            let exe = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .filter(|s| !s.is_empty());
            let name = path
                .file_stem()
                .map(|f| f.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| exe.clone())
                .unwrap_or_else(|| "Windows App".to_string());

            return FrontmostApp {
                name,
                bundle_id: None,
                exe,
                pid: Some(pid),
                detection_us: started.elapsed().as_micros() as u64,
                window_center_x,
                primary_screen_width,
            };
        }

        FrontmostApp {
            name: "Windows App".to_string(),
            bundle_id: None,
            exe: None,
            pid: Some(pid),
            detection_us: started.elapsed().as_micros() as u64,
            window_center_x,
            primary_screen_width,
        }
    }
}

fn is_handle_elevated(proc_handle: HANDLE) -> Option<bool> {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(proc_handle, TOKEN_QUERY, &mut token) == 0 {
            if GetLastError() == ERROR_ACCESS_DENIED {
                return Some(true);
            }
            return None;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        if ok != 0 {
            Some(elevation.TokenIsElevated != 0)
        } else {
            None
        }
    }
}

pub fn is_current_process_elevated() -> bool {
    unsafe { is_handle_elevated(GetCurrentProcess()).unwrap_or(false) }
}

pub fn is_process_elevated(pid: u32) -> Option<bool> {
    unsafe {
        let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if proc_handle.is_null() {
            if GetLastError() == ERROR_ACCESS_DENIED {
                return Some(true);
            }
            return None;
        }
        let res = is_handle_elevated(proc_handle);
        let _ = CloseHandle(proc_handle);
        res
    }
}

pub fn check_pre_injection_guard(target: &FrontmostApp) -> Result<(), String> {
    if let Some(pid) = target.pid {
        if !is_current_process_elevated() && is_process_elevated(pid) == Some(true) {
            return Err(format!(
                "{} is running as Administrator (elevated), so Windows UIPI blocks synthetic input from a non-elevated saywhen. Transcript was copied to your clipboard; run the target app normally or launch saywhen as Administrator.",
                target.name
            ));
        }
    }
    Ok(())
}

pub fn clipboard_has_any_content() -> bool {
    false
}
