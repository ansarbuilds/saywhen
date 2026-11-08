use crate::inject::FrontmostApp;
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSPasteboard, NSWorkspace};
use std::path::Path;
use std::time::Instant;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> u8;
}

pub fn check_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn is_secure_event_input_enabled() -> bool {
    unsafe { IsSecureEventInputEnabled() != 0 }
}

pub fn prompt_accessibility_permission() -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFMutableDictionary;
    use core_foundation::string::CFString;

    unsafe {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let mut options = CFMutableDictionary::new();
        options.set(key, CFBoolean::true_value());
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const std::ffi::c_void)
    }
}

pub fn accessibility_denied_message() -> String {
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "saywhen".to_string());
    format!(
        "macOS Accessibility permission is required to type into terminals. \
         If saywhen is already enabled in System Settings but this still appears, a recent \
         dev rebuild changed the binary signature: turn saywhen off, remove it from the list, \
         quit the app, run again (cargo signs with Apple Development when available), then add \
         and enable saywhen once more. Dev binary: {exe}. \
         Transcript was copied to your clipboard."
    )
}

pub fn open_accessibility_settings() -> Result<(), String> {
    let _trusted = prompt_accessibility_permission();
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map_err(|e| format!("Failed to open macOS Accessibility settings: {}", e))?;
    Ok(())
}

pub fn detect_frontmost_app() -> FrontmostApp {
    let started = Instant::now();
    autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        if let Some(app) = workspace.frontmostApplication() {
            let name = app
                .localizedName()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "macOS App".to_string());

            let bundle_id = app
                .bundleIdentifier()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty());

            let exe = app
                .executableURL()
                .and_then(|url| url.path())
                .and_then(|path_ns| {
                    let full = path_ns.to_string();
                    Path::new(&full)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                })
                .filter(|s| !s.trim().is_empty());

            let raw_pid = app.processIdentifier();
            let pid = if raw_pid > 0 {
                Some(raw_pid as u32)
            } else {
                None
            };

            let detection_us = started.elapsed().as_micros() as u64;
            return FrontmostApp {
                name,
                bundle_id,
                exe,
                pid,
                detection_us,
                window_center_x: None,
                primary_screen_width: None,
            };
        }

        FrontmostApp {
            name: "macOS App".to_string(),
            bundle_id: None,
            exe: None,
            pid: None,
            detection_us: started.elapsed().as_micros() as u64,
            window_center_x: None,
            primary_screen_width: None,
        }
    })
}

pub fn check_pre_injection_guard(_target: &FrontmostApp) -> Result<(), String> {
    if !check_accessibility_permission() {
        return Err(accessibility_denied_message());
    }
    if is_secure_event_input_enabled() {
        return Err(
            "macOS Secure Keyboard Entry is active (for example a password prompt or Terminal Secure Keyboard Entry), which blocks synthetic keystrokes. Transcript was copied to your clipboard.".to_string(),
        );
    }
    Ok(())
}

pub fn clipboard_has_any_content() -> bool {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        pb.types().map(|arr| !arr.is_empty()).unwrap_or(false)
    })
}
