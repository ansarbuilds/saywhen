use super::*;
use std::collections::HashMap;
use std::time::Instant;

fn app(name: &str, bundle_id: Option<&str>, exe: Option<&str>, pid: Option<u32>) -> FrontmostApp {
    FrontmostApp {
        name: name.to_string(),
        bundle_id: bundle_id.map(str::to_string),
        exe: exe.map(str::to_string),
        pid,
        detection_us: 0,
        window_center_x: None,
        primary_screen_width: None,
    }
}

#[test]
fn test_allowlist_xcode_does_not_match_code() {
    let allowlist = default_auto_send_allowlist();
    let xcode = app("Xcode", Some("com.apple.dt.Xcode"), Some("Xcode"), Some(4001));
    assert!(!matches_allowlist(&xcode, &allowlist));

    let policy = evaluate_app_injection_policy(&xcode, true, true, false, &allowlist);
    assert_eq!(policy.policy, "paste_only");
    assert!(!policy.will_press_enter);

    let claude_notes = app("My Claude Notes", Some("com.example.notes"), None, Some(4002));
    assert!(!matches_allowlist(&claude_notes, &allowlist));
}

#[test]
fn test_allowlist_matches_bundle_ids_and_windows_exes() {
    let allowlist = default_auto_send_allowlist();

    let vscode_mac = app(
        "Code",
        Some("com.microsoft.VSCode"),
        Some("Electron"),
        Some(5001),
    );
    assert!(matches_allowlist(&vscode_mac, &allowlist));

    let cursor_mac = app(
        "Cursor",
        Some("com.todesktop.230313mzl4w4u92"),
        Some("Cursor"),
        Some(5002),
    );
    assert!(matches_allowlist(&cursor_mac, &allowlist));

    let iterm_mac = app(
        "iTerm2",
        Some("com.googlecode.iterm2"),
        Some("iTerm2"),
        Some(5003),
    );
    assert!(matches_allowlist(&iterm_mac, &allowlist));

    let warp_mac = app(
        "Warp",
        Some("dev.warp.Warp-Stable"),
        Some("stable"),
        Some(5004),
    );
    assert!(matches_allowlist(&warp_mac, &allowlist));

    let ghostty_mac = app(
        "Ghostty",
        Some("com.mitchellh.ghostty"),
        Some("ghostty"),
        Some(5005),
    );
    assert!(matches_allowlist(&ghostty_mac, &allowlist));

    let zed_mac = app("Zed", Some("dev.zed.Zed"), Some("zed"), Some(5006));
    assert!(matches_allowlist(&zed_mac, &allowlist));

    let win_term = app(
        "WindowsTerminal",
        None,
        Some("WindowsTerminal.exe"),
        Some(6001),
    );
    assert!(matches_allowlist(&win_term, &allowlist));

    let win_code = app("Code", None, Some("Code.exe"), Some(6002));
    assert!(matches_allowlist(&win_code, &allowlist));

    let win_cursor = app("Cursor", None, Some("Cursor.exe"), Some(6003));
    assert!(matches_allowlist(&win_cursor, &allowlist));

    // Direct bundle ID or exe in custom user allowlist
    let custom_allowlist = vec![
        "com.googlecode.iterm2".to_string(),
        "WindowsTerminal.exe".to_string(),
    ];
    let renamed_iterm = app("Custom iTerm", Some("com.googlecode.iterm2"), None, Some(7001));
    assert!(matches_allowlist(&renamed_iterm, &custom_allowlist));
    assert!(matches_allowlist(&win_term, &custom_allowlist));
}

#[test]
fn test_saywhen_self_is_always_refused() {
    let mut allowlist = default_auto_send_allowlist();
    assert!(!allowlist.iter().any(|a| a.eq_ignore_ascii_case("saywhen")));

    // Even if a legacy settings file still lists "saywhen", self-injection is refused.
    allowlist.push("saywhen".to_string());

    let by_name = app("saywhen", None, None, Some(999_991));
    assert!(is_saywhen_app(&by_name));
    assert!(!matches_allowlist(&by_name, &allowlist));

    let by_bundle = app("Custom Title", Some(SAYWHEN_BUNDLE_ID), None, Some(999_992));
    assert!(is_saywhen_app(&by_bundle));

    let by_exe = app("Window", None, Some("saywhen.exe"), Some(999_993));
    assert!(is_saywhen_app(&by_exe));

    let by_pid = app("Terminal", Some("com.apple.Terminal"), None, Some(std::process::id()));
    assert!(is_saywhen_app(&by_pid));

    let policy = evaluate_app_injection_policy(&by_name, true, true, false, &allowlist);
    assert_eq!(policy.policy, "self_guard");
    assert!(!policy.will_press_enter);

    let term = app("Terminal", Some("com.apple.Terminal"), Some("Terminal"), Some(1001));
    let preview_policy = evaluate_app_injection_policy(&term, true, true, false, &allowlist);
    let decision = resolve_target_for_injection(&preview_policy, &by_name, TargetChangePolicy::PasteOnly);
    assert!(matches!(decision, PreInjectionDecision::Abort(_)));
}

#[test]
fn test_target_changed_between_preview_and_injection() {
    let allowlist = default_auto_send_allowlist();
    let terminal = app("Terminal", Some("com.apple.Terminal"), Some("Terminal"), Some(2001));
    let slack = app("Slack", Some("com.tinyspeck.slackmacgap"), Some("Slack"), Some(2002));

    let preview_policy = evaluate_app_injection_policy(&terminal, true, true, false, &allowlist);
    assert!(preview_policy.will_press_enter);

    // 1. Target unchanged -> proceeds with Enter
    let same = resolve_target_for_injection(&preview_policy, &terminal, TargetChangePolicy::Abort);
    assert_eq!(
        same,
        PreInjectionDecision::Proceed {
            target_app: terminal.clone(),
            press_enter: true,
            policy_applied: "auto_send".to_string(),
            target_changed_notice: None,
        }
    );

    // 2. Target changed to Slack with Abort policy -> aborts
    let aborted = resolve_target_for_injection(&preview_policy, &slack, TargetChangePolicy::Abort);
    match aborted {
        PreInjectionDecision::Abort(msg) => {
            assert!(msg.contains("Terminal -> Slack"));
            assert!(msg.contains("aborted"));
        }
        other => panic!("Expected Abort, got {:?}", other),
    }

    // 3. Target changed to Slack with PasteOnly policy -> pastes without Enter
    let paste_only = resolve_target_for_injection(&preview_policy, &slack, TargetChangePolicy::PasteOnly);
    match paste_only {
        PreInjectionDecision::Proceed {
            target_app,
            press_enter,
            policy_applied,
            target_changed_notice,
        } => {
            assert_eq!(target_app.name, "Slack");
            assert!(!press_enter, "Must NEVER press Enter when target app changed");
            assert_eq!(policy_applied, "target_changed_paste_only");
            assert!(target_changed_notice.is_some());
        }
        other => panic!("Expected Proceed with press_enter=false, got {:?}", other),
    }
}

#[test]
fn test_enter_delay_scaling_and_per_app_overrides() {
    let terminal = app("Terminal", Some("com.apple.Terminal"), Some("Terminal"), Some(3001));
    let cursor = app(
        "Cursor",
        Some("com.todesktop.230313mzl4w4u92"),
        Some("Cursor"),
        Some(3002),
    );
    let mut overrides = HashMap::new();
    overrides.insert("Cursor".to_string(), 180);

    // Short text (12 chars < 20) -> base 120 ms
    assert_eq!(compute_enter_delay_ms(120, "cargo check", &terminal, &overrides), 120);

    // 200 chars -> 120 + (200 / 20) = 130 ms
    let medium = "a".repeat(200);
    assert_eq!(compute_enter_delay_ms(120, &medium, &terminal, &overrides), 130);

    // 1,000 chars -> 120 + (1000 / 20) = 170 ms
    let long = "b".repeat(1000);
    assert_eq!(compute_enter_delay_ms(120, &long, &terminal, &overrides), 170);

    // 10,000 chars -> 120 + 500 = 620 -> capped at 400 ms
    let huge = "c".repeat(10_000);
    assert_eq!(compute_enter_delay_ms(120, &huge, &terminal, &overrides), 400);

    // Per-app override for Cursor: 180 + (200 / 20) = 190 ms
    assert_eq!(compute_enter_delay_ms(120, &medium, &cursor, &overrides), 190);
}

#[test]
fn test_clipboard_restore_decision() {
    let injected = "cargo test --manifest-path src-tauri/Cargo.toml";
    assert!(should_restore_clipboard(injected, Some(injected)));
    assert!(!should_restore_clipboard(injected, Some("user copied something new")));
    assert!(!should_restore_clipboard(injected, Some("")));
    assert!(!should_restore_clipboard(injected, None));
}

#[test]
fn test_frontmost_app_detection_latency_20_calls() {
    let mut samples_us = Vec::with_capacity(20);
    let mut last = FrontmostApp::default();
    for _ in 0..20 {
        let t0 = Instant::now();
        last = detect_frontmost_app();
        samples_us.push(t0.elapsed().as_micros() as u64);
    }
    samples_us.sort_unstable();
    let median_us = (samples_us[9] + samples_us[10]) as f64 / 2.0;
    println!(
        "NSWORKSPACE_FRONTMOST_20_CALLS: app={:?} bundle={:?} pid={:?} median={:.1} us ({:.3} ms), min={} us, max={} us",
        last.name,
        last.bundle_id,
        last.pid,
        median_us,
        median_us / 1000.0,
        samples_us[0],
        samples_us[19]
    );
    assert!(!last.name.is_empty());
}

#[test]
fn test_clipboard_image_preservation_roundtrip() {
    let Ok(mut cb) = Clipboard::new() else {
        return;
    };
    let orig_text = cb.get_text().ok();
    let orig_img = cb.get_image().ok().map(|i| (i.width, i.height, i.bytes.into_owned()));

    // Place a 2x2 RGBA test image on the OS clipboard
    let test_pixels: Vec<u8> = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
    ];
    cb.set_image(ImageData {
        width: 2,
        height: 2,
        bytes: Cow::Borrowed(&test_pixels),
    })
    .expect("set_image should succeed");

    let saved_img = cb.get_image().expect("get_image should read back 2x2 image");
    assert_eq!(saved_img.width, 2);
    assert_eq!(saved_img.height, 2);
    let saved_bytes = saved_img.bytes.into_owned();

    // Overwrite with transcript text as injection does
    let transcript = "git diff --stat";
    cb.set_text(transcript.to_string()).expect("set_text should succeed");
    let current_text = cb.get_text().ok();
    assert!(should_restore_clipboard(transcript, current_text.as_deref()));

    // Restore the saved image
    cb.set_image(ImageData {
        width: 2,
        height: 2,
        bytes: Cow::Borrowed(&saved_bytes),
    })
    .expect("restoring image should succeed");

    let after_restore = cb.get_image().expect("image should be back on clipboard");
    assert_eq!(after_restore.width, 2);
    assert_eq!(after_restore.height, 2);
    assert_eq!(after_restore.bytes.as_ref(), test_pixels.as_slice());

    // Restore the user's original clipboard state
    if let Some(t) = orig_text {
        let _ = cb.set_text(t);
    } else if let Some((w, h, bytes)) = orig_img {
        let _ = cb.set_image(ImageData {
            width: w,
            height: h,
            bytes: Cow::Borrowed(&bytes),
        });
    }
}

#[test]
fn test_saywhen_bundle_id_matches_tauri_identifier() {
    let conf: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).expect("tauri.conf.json parses");
    assert_eq!(conf["identifier"].as_str(), Some(SAYWHEN_BUNDLE_ID));
}

#[test]
fn test_enter_skipped_when_focus_moves_after_paste() {
    let terminal = app("Terminal", Some("com.apple.Terminal"), Some("Terminal"), Some(8001));
    let slack = app("Slack", Some("com.tinyspeck.slackmacgap"), Some("Slack"), Some(8002));
    let own = app("saywhen", Some(SAYWHEN_BUNDLE_ID), None, Some(std::process::id()));

    assert!(should_press_enter_after_paste(&terminal, &terminal.clone()));
    assert!(!should_press_enter_after_paste(&terminal, &slack));
    assert!(!should_press_enter_after_paste(&terminal, &own));
}

#[test]
fn test_linux_xprop_parsing_and_terminal_allowlist() {
    use super::linux::{parse_xprop_active_window_id, parse_xprop_window_props};

    let root = "_NET_ACTIVE_WINDOW(WINDOW): window id # 0x3a00007\n";
    assert_eq!(
        parse_xprop_active_window_id(root),
        Some("0x3a00007".to_string())
    );
    assert_eq!(
        parse_xprop_active_window_id("_NET_ACTIVE_WINDOW(WINDOW): window id # 0x0\n"),
        None
    );

    let props = "\
_NET_WM_PID(CARDINAL) = 42190\n\
WM_CLASS(STRING) = \"Alacritty\", \"Alacritty\"\n\
_NET_WM_NAME(UTF8_STRING) = \"zsh - ~/Projects/saywhen\"\n";
    let (display_name, wm_class, pid) = parse_xprop_window_props(props);
    assert_eq!(display_name.as_deref(), Some("Alacritty"));
    assert_eq!(wm_class.as_deref(), Some("Alacritty"));
    assert_eq!(pid, Some(42190));

    let allow = default_auto_send_allowlist();
    let alacritty_linux = app("Alacritty", Some("Alacritty"), Some("alacritty"), Some(42190));
    assert!(matches_allowlist(&alacritty_linux, &allow));

    let gnome_term = app(
        "Gnome-terminal",
        Some("org.gnome.Terminal"),
        Some("gnome-terminal-server"),
        Some(42191),
    );
    assert!(matches_allowlist(&gnome_term, &allow));
}

#[test]
fn test_multi_monitor_gaze_window_alignment_guard() {
    let mut left_display_terminal = app(
        "Terminal",
        Some("com.apple.Terminal"),
        Some("Terminal"),
        Some(9001),
    );
    left_display_terminal.window_center_x = Some(-1200);
    left_display_terminal.primary_screen_width = Some(1920);

    // Looking left or center toward the left display -> aligned (no warning)
    assert!(evaluate_gaze_window_alignment(&left_display_terminal, -18.0).is_none());
    assert!(evaluate_gaze_window_alignment(&left_display_terminal, 0.0).is_none());

    // Looking strongly right (+19 deg) while focused terminal is on the left display -> mismatch!
    let mismatch_left = evaluate_gaze_window_alignment(&left_display_terminal, 19.0)
        .expect("should flag left-display window vs right gaze");
    assert!(mismatch_left.contains("left display"));
    assert!(mismatch_left.contains("Paste-Only"));

    let mut right_display_cursor = app(
        "Cursor",
        Some("com.todesktop.230313mzl4w4u92"),
        Some("Cursor"),
        Some(9002),
    );
    right_display_cursor.window_center_x = Some(2800);
    right_display_cursor.primary_screen_width = Some(1920);

    // Looking right or center -> aligned
    assert!(evaluate_gaze_window_alignment(&right_display_cursor, 16.0).is_none());
    assert!(evaluate_gaze_window_alignment(&right_display_cursor, 0.0).is_none());

    // Looking strongly left (-18 deg) while focused Cursor is on the right display -> mismatch!
    let mismatch_right = evaluate_gaze_window_alignment(&right_display_cursor, -18.0)
        .expect("should flag right-display window vs left gaze");
    assert!(mismatch_right.contains("right display"));
    assert!(mismatch_right.contains("Paste-Only"));
}
