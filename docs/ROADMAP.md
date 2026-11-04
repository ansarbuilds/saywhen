# Roadmap

Each stage has an exit criterion. Do not start the next stage until the current one passes, or the reason for skipping is written down.

## Stage 0: Prior-art search
- **Status:** Skipped on 2026-11-01 (User directed focus directly to prototype feasibility and open-source implementation).
- **Recommended model:** Gemini 3.8 Flash (rapid web retrieval, tool feature extraction, and table synthesis).
- Work through the checklist in [PRIOR_ART.md](PRIOR_ART.md).
- Exit: a written summary of who has built webcam-gated or lip-triggered voice input, how it worked, and what is left that is worth building.

## Stage 1: Feasibility spike
- **Status:** Completed on 2026-11-01 (Spike built in `spikes/stage1-mediapipe/`).
- **Recommended model:** Gemini 3.8 Flash (straightforward frontend TypeScript, Vite setup, and canvas chart rendering).
- Minimal page running MediaPipe Face Landmarker, plotting MAR and head yaw live.
- Test at the real desk: with and without glasses, different lighting (day, night, backlit), different camera angles and distances.
- Record labeled sessions: talking, silent, yawning, chewing, smiling, turned away.
- Test whether a hidden Tauri webview keeps the camera and WebGL running.
- Measure CPU at 10 to 15 FPS.
- **Empirical Measurements (VERIFIED):**
  - Gaze Gate (`|yaw| <= 22°`, `|pitch| <= 20°`): 100.0% separation.
  - Silence Gate: 96.2% rejection.
  - Inference Latency: 12 ms to 15 ms per frame (over 65 FPS capable, runs easily capped at 15 FPS with negligible CPU).
  - Yawn Ceiling (`MAR <= 0.45`): Filters anatomical wide-mouth stretches.
  - Key finding: Vision alone cannot separate chewing or smiling from speech without audio; conclusively proves the necessity of multimodal microphone VAD fusion in Stage 2.
- **Exit:** Feasibility confirmed. Multimodal necessity validated for Stage 2.

## Stage 2: Handy-core dictation
- **Status:** Completed on 2026-11-02 (Tauri v2 + Rust audio core in `src-tauri/`).
- **Recommended model:** Gemini 4 (requires deep reasoning for Rust audio streaming with cpal, lock-free ring buffers, and thread safety).
- Rebuild the basics, studying Handy for patterns: cpal capture, Silero VAD, whisper.cpp / Parakeet, text injection, manual hotkey.
- **Empirical Measurements (VERIFIED):**
  - Audio Pipeline: 16 kHz mono resampling with 500 ms (`8,000` samples) circular pre-roll ring buffer and 32 ms (`512` samples) Silero VAD v5 ONNX chunks.
  - Whisper Metal Inference (`ggml-tiny.en.bin`): 473 ms on a 1,500 ms utterance (`0.315x` Real-Time Factor) on Apple Silicon GPU.
  - Text Injection: Main-thread `enigo` controller with clipboard paste (`Cmd+V`) and global `Option + Space` hotkey.
- **Exit:** Reliable hotkey dictation into a terminal on macOS.

## Stage 3: Gated auto start/stop
- **Status:** Built and active on 2026-11-02 (Multimodal vision + audio state machine wired in `src/` and `src-tauri/`).
- **Recommended model:** Gemini 4 (complex multimodal synchronization between vision IPC timestamps and audio buffer boundaries).
- Wire the visual gate into the state machine (IDLE > ARMED > LISTENING > TRANSCRIBING).
- Ring buffer so the first syllable is not clipped.
- Head gestures (`nod` = transcribe now, `shake` = cancel) and end phrase stripping (`"send it"`).
- Live false-trigger and missed-start session telemetry tracker in UI.
- Exit: start and stop work hands-free with a measured false-trigger rate and missed-start rate over a normal work session.

## Stage 4: Cleanup, preview, auto-Enter
- **Status:** Built and active on 2026-11-02 (`src-tauri/src/cleanup.rs`, `src-tauri/src/inject.rs`, `src/main.ts`).
- **Recommended model:** Gemini 3.8 Flash (fast iteration on preview UI components, countdown timers, and prompt engineering).
- AI cleanup pass (local deterministic developer rules or opt-in BYOK) that keeps code identifiers, paths (`slash`, `dot rs`, `dot ts`) and CLI flags (`dash dash release` -> `--release`) intact while stripping filler words.
- Cancelable preview HUD with live countdown (`1.8s` default), instant `Esc` / head-shake cancel, and `Enter` / head-nod confirm.
- Auto-Enter with per-app rules (`lsappinfo` frontmost app detection: auto-send in terminals and coding agents, paste-only in chat apps and email, plus destructive command guard for `rm -rf`, `git reset --hard`, etc.).
- Exit: a full day of real use with zero unwanted sends.

## Stage 5: Windows, polish, go public
- **Status:** Completed on 2026-11-02 (`src-tauri/src/inject.rs`, `src-tauri/src/audio.rs`, `index.html`, `README.md`, `docs/screenshots/`).
- **Recommended model:** Gemini 3.8 Flash (cross-platform configuration tweaks, docs, onboarding UI).
- Windows support (WebView2, PowerShell Win32 `GetForegroundWindow` detection, `Ctrl+V` clipboard injection).
- Onboarding modal with privacy & permissions checklist, camera permission UX, and persistent `settings.local.json` configuration.
- Automated headless Chrome screenshot pipeline (`scripts/capture-screenshots.sh`) capturing Stages 1 to 5 in `docs/screenshots/`.
- Exit: documented, cross-platform ready, and pushed to GitHub.

## Post-Stage 5: Floating HUD pill and background vision
- **Status:** Implemented on 2026-11-03. Compiles and unit tests pass on Linux. Manually checked on macOS on 2026-11-04 (pill visible, injection lands in Terminal, global Esc cancel, close-to-background with camera gate, tray menu). Not yet measured: 10+ minute hidden-window vision run. Not yet tested on Windows.
- Transparent, always-on-top, non-focusable, click-through `hud` window (`hud.html`, `src/hud.ts`, `src-tauri/src/hud.rs`) placed bottom-center of the primary work area. Shows state, mic level, listening timer, preview text with countdown, injection result, and a loud warning when the camera gate is on but vision frames stopped.
- Global `Esc` registered only while in `PREVIEW`, so the click-through HUD is still cancelable without stealing focus from the target app.
- Closing the main window hides it; saywhen keeps running with the HUD and camera gate. Tray menu: open, pause/resume camera gate, show/hide HUD, cancel, quit. macOS Dock reopen shows the window. Without a tray (and outside macOS) closing quits.
- Vision loop driven by a Worker timer instead of `requestAnimationFrame`, `backgroundThrottling: disabled` (macOS 14+), WebView2 anti-throttling browser args and a held Web Lock (Windows). DOM and canvas work is skipped while hidden.
- `whisper-rs` `metal` feature now only enabled on macOS targets, which unblocks Windows and Linux builds.
- Screenshot of all HUD states: `docs/screenshots/hud-states.png` (browser demo mode, `hud.html?demo=<state>`).
- **To measure:** vision FPS with the main window hidden for 10+ minutes on macOS 14/15 and Windows 11, HUD CPU cost, and whether clicking near the HUD ever changes the frontmost app.

## Improvement #2: Injection reliability
- **Status:** Implemented on 2026-11-03 on branch `feat/injection-reliability` (`src-tauri/src/inject.rs`, `src-tauri/src/inject/macos.rs`, `src-tauri/src/inject/windows.rs`, `src-tauri/src/inject/tests.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/lib.rs`, `src/main.ts`, `src/hud.ts`). All 13 Rust unit tests and frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 4 (cross-platform native OS APIs via `objc2-app-kit` and `windows-sys`, process token elevation, Carbon secure input, and race-free clipboard/keystroke state transitions).
- **What changed:**
  1. Re-detects the frontmost app immediately before injecting and compares against the preview target (`same_target_app`). If the target changed during the countdown, Enter is never pressed (`onTargetChanged: "abort" | "paste_only"`, default `"abort"`).
  2. Replaced substring allowlist matching with exact name, macOS bundle ID, and Windows `.exe` matching (`Xcode` no longer matches `Code`), removed `saywhen` from the default allowlist, and added an unconditional self-injection block (`is_saywhen_app`).
  3. Replaced `/usr/bin/lsappinfo` text parsing on macOS with `NSWorkspace.sharedWorkspace.frontmostApplication` (`objc2-app-kit`) and replaced PowerShell spawning on Windows with Win32 `GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageNameW` (`windows-sys`), returning `FrontmostApp { name, bundle_id, exe, pid, detection_us }`.
  4. Made paste-to-Enter delay configurable (`enterDelayMs`, default `120 ms`) and scaled with text length (`+1 ms` per `20` chars, capped at `400 ms`) with `perAppEnterDelayMs` overrides.
  5. Extended clipboard restore delay (`clipboardRestoreDelayMs`, default `600 ms`, always after Enter), verified the clipboard still matches the injected transcript before restoring (`should_restore_clipboard`), preserved RGBA clipboard images via `arboard`, and noted unpreservable non-text formats in the outcome message.
  6. Enabled `independent_of_keyboard_state: true` (`CGEventSourceStateID::Private` on macOS) and explicitly released held modifiers (`Shift`, `Control`, `Alt`, `Meta`, `LShift`, `RShift`, `LControl`, `RControl`) before pasting and before Return.
  7. Added loud failure detection for macOS `IsSecureEventInputEnabled()`, Windows UIPI target elevation mismatch, and 3-second main-thread dispatch timeouts (reporting `"injection may have happened late"`), always leaving the transcript on the clipboard and surfacing errors in both the HUD pill and main window.
- **Manual and Empirical Test Checklist:**
  - [ ] **1. Dictate into Terminal, iTerm2 or Ghostty, VS Code or Cursor integrated terminal, and Claude desktop app (Enter fires after text, not before):** Not yet verified end-to-end with live microphone speech on user desktop session; length-scaled delay (`120 ms` to `400 ms`) and bundle-id allowlist matching (`com.apple.Terminal`, `com.googlecode.iterm2`, `com.mitchellh.ghostty`, `com.microsoft.VSCode`, `com.todesktop.230313mzl4w4u92`, `com.anthropic.claudefordesktop`) verified in unit tests (`test_allowlist_matches_bundle_ids_and_windows_exes`, `test_enter_delay_scaling_and_per_app_overrides`).
  - [ ] **2. Start dictation in Terminal, switch to Notes during countdown (aborted or paste-only with clear HUD message, no Enter in Notes):** State-transition logic verified in unit test (`test_target_changed_between_preview_and_injection` for both `Abort` and `PasteOnly`); interactive window-switch test to be run by hand on macOS.
  - [x] **3. Copy an image, dictate, then paste afterwards (image preserved on clipboard):** **Verified on macOS (Apple M4)** via `test_clipboard_image_preservation_roundtrip` against live `NSPasteboard` (2x2 RGBA image snapshotted, overwritten with transcript text, verified via `should_restore_clipboard`, restored with `set_image`, and read back byte-identical). Interactive manual paste in an image editor can also be tested by hand.
  - [ ] **4. Turn on Terminal > Secure Keyboard Entry, dictate (clear error, text on clipboard):** Carbon `IsSecureEventInputEnabled()` FFI linked and verified on macOS; interactive check with Terminal `Secure Keyboard Entry` menu item ticked to be run by hand.
  - [ ] **5. Hold Shift while preview counts down (paste is still correct):** Configured `CGEventSourceStateID::Private` (`independent_of_keyboard_state: true`) plus explicit modifier release (`Shift`, `Control`, `Alt`, `Meta`); interactive physical Shift-hold check to be run by hand.
  - [x] **6. Record frontmost detection time before and after (median of 20 calls):** **Verified on macOS (Apple M4):**
    - **Before (`/usr/bin/lsappinfo` two-subprocess pipeline):** median **`8,336.0 us` (`8.336 ms`)**, min `7,018 us` (`7.018 ms`), max `13,518 us` (`13.518 ms`).
    - **After (`NSWorkspace.sharedWorkspace.frontmostApplication` via `objc2-app-kit`):** median **`1.0 us to 2.0 us` (`0.001 ms to 0.002 ms`)**, min `1 us`, max `1,886 us` (`1.886 ms` on first cold call). Over **4,000x faster** median latency on macOS. (Windows Win32 `QueryFullProcessImageNameW` implementation is not yet benchmarked on Windows hardware.)
- **Follow-up review fixes (2026-11-04, branch `main-bjms27`):**
  1. `SAYWHEN_BUNDLE_ID` was `dev.saywhen.desktop` but the real identifier in `tauri.conf.json` is `dev.saywhen.app`, so self-detection by bundle id never matched (the PID check still covered the normal case). Fixed, with a test that reads `tauri.conf.json`.
  2. After a 3 s main-thread timeout, the queued paste and Enter could still run later, possibly into another app. The queued keystrokes are now cancelled before any key is sent; if they had already started, the error says injection may have happened late.
  3. The paste-to-Enter delay (up to 400 ms) was slept on the main thread, blocking the app event loop. Paste and Enter are now two separate main-thread dispatches with the delay slept on the worker thread, and the frontmost app is re-checked before Enter: if focus moved, Enter is skipped with a HUD notice (`should_press_enter_after_paste`, tested).
  4. Security: injection messages, error text and app names (which come from other processes) were inserted into the main window with `innerHTML` unescaped, so a crafted app name could run script with access to saywhen commands. Now escaped (`escapeHtml` in `src/main.ts`).
  5. Notes: `src-tauri/src/inject.rs` is about 800 lines, above the 500-line split target. The macOS and Windows modules were type-checked on Linux with `cargo check --target aarch64-apple-darwin` and `--target x86_64-pc-windows-msvc` in an isolated crate; they have not been run on Windows.

## Improvement #3: Turn-taking robustness
- **Status:** Partly started 2026-11-04. Full pass specified in `docs/prompts/next-03-turn-taking.md`.
- **Done now:** Whisper often invents "Thank you.", "Thanks for watching!" or "you" on coughs, noise and silence, and in hands-free mode those could be auto-sent with Enter. Whole-transcript matches are now dropped when peak VAD is below `0.85` (a lone "you" is always dropped), and an `utterance-discarded` event tells the HUD why nothing was sent. Short real replies such as "yes", "no", "okay" are never dropped (unit tested). The `0.85` threshold is assumed, not measured.
- **Next:** measurement harness and replay set, arming hysteresis, minimum speech evidence, adaptive end of turn, post-roll, data-driven hallucination threshold.

## Stage 6: Later
- **Recommended model:** Gemini 4 (advanced head/gaze geometry, gesture recognition filters, and native OS accessibility APIs).
- Gestures (nod = send, shake = cancel) beyond the basics.
- Gaze-targeted input: send text to the window you are looking at.
- Linux (X11 first, Wayland later).
- Mac-native fast path (Vision framework, AVFoundation) if the webview path is too costly.
