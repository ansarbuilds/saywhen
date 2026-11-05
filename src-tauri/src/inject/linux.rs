#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
use super::FrontmostApp;
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
use std::process::Command;
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
use std::time::Instant;

/// Parses the output of `xprop -root _NET_ACTIVE_WINDOW` into a hex window ID such as `"0x3a00007"`.
pub fn parse_xprop_active_window_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if !line.contains("_NET_ACTIVE_WINDOW") {
            continue;
        }
        if let Some(pos) = line.rfind("0x") {
            let token: String = line[pos..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit() || *c == 'x' || *c == 'X')
                .collect();
            if token.len() > 2 && token != "0x0" && token != "0x00000000" {
                return Some(token);
            }
        }
    }
    None
}

/// Parses the output of `xprop -id <wid> _NET_WM_PID WM_CLASS _NET_WM_NAME` into
/// `(display_name, wm_class, pid)`.
pub fn parse_xprop_window_props(output: &str) -> (Option<String>, Option<String>, Option<u32>) {
    let mut pid: Option<u32> = None;
    let mut wm_class: Option<String> = None;
    let mut wm_name: Option<String> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("_NET_WM_PID(") {
            if let Some((_, rhs)) = trimmed.split_once('=') {
                pid = rhs.trim().parse::<u32>().ok();
            }
        } else if trimmed.starts_with("WM_CLASS(") {
            if let Some((_, rhs)) = trimmed.split_once('=') {
                // Format is typically: `"instance", "ClassName"`
                let quoted: Vec<&str> = rhs
                    .split('"')
                    .enumerate()
                    .filter_map(|(i, seg)| if i % 2 == 1 { Some(seg.trim()) } else { None })
                    .filter(|s| !s.is_empty())
                    .collect();
                if let Some(&class_str) = quoted.last() {
                    wm_class = Some(class_str.to_string());
                }
            }
        } else if trimmed.starts_with("_NET_WM_NAME(") {
            if let Some((_, rhs)) = trimmed.split_once('=') {
                let unquoted = rhs.trim().trim_matches('"').trim();
                if !unquoted.is_empty() {
                    wm_name = Some(unquoted.to_string());
                }
            }
        }
    }

    let display_name = wm_class.clone().or(wm_name);
    (display_name, wm_class, pid)
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub fn detect_frontmost_app() -> FrontmostApp {
    let t0 = Instant::now();
    let root_out = Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output();

    if let Ok(out) = root_out {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(wid) = parse_xprop_active_window_id(&stdout) {
                if let Ok(win_out) = Command::new("xprop")
                    .args(["-id", &wid, "_NET_WM_PID", "WM_CLASS", "_NET_WM_NAME"])
                    .output()
                {
                    if win_out.status.success() {
                        let props_str = String::from_utf8_lossy(&win_out.stdout);
                        let (display_name, wm_class, pid) = parse_xprop_window_props(&props_str);
                        let exe = pid.and_then(|p| {
                            std::fs::read_link(format!("/proc/{}/exe", p))
                                .ok()
                                .and_then(|path| {
                                    path.file_name()
                                        .map(|f| f.to_string_lossy().to_string())
                                })
                        });
                        let name = display_name
                            .or_else(|| exe.clone())
                            .unwrap_or_else(|| "Focused Window".to_string());
                        return FrontmostApp {
                            name,
                            bundle_id: wm_class,
                            exe,
                            pid,
                            detection_us: t0.elapsed().as_micros() as u64,
                        };
                    }
                }
            }
        }
    }

    FrontmostApp {
        name: "Focused Window".to_string(),
        bundle_id: None,
        exe: None,
        pid: None,
        detection_us: t0.elapsed().as_micros() as u64,
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub fn check_pre_injection_guard(_target: &FrontmostApp) -> Result<(), String> {
    let is_wayland = std::env::var("XDG_SESSION_TYPE")
        .map(|v| v.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
    let has_x11_display = std::env::var("DISPLAY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    if is_wayland && !has_x11_display {
        return Err(
            "Pure Wayland session without XWayland DISPLAY detected: synthetic keystrokes may be blocked by the compositor. Transcript was copied to your clipboard.".to_string(),
        );
    }
    Ok(())
}
