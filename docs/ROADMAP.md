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
- **Status:** Implemented and benchmarked on macOS (Apple M4) on 2026-11-04 on branch `feat/turn-taking` (`src-tauri/src/turn_taking.rs`, `src-tauri/src/bin/replay.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/transcribe.rs`, `src-tauri/src/lib.rs`, `src/main.ts`, `src/hud.ts`). All 22 Rust unit tests, the 56-utterance offline replay benchmark, and the frontend build pass with zero compiler warnings. Not yet measured on Windows hardware.
- **Recommended model:** Gemini 4 (multimodal state machine hysteresis, pure start/stop extraction for deterministic WAV + vision replay, adaptive end-of-turn timing, and data-driven VAD threshold calibration).
- **What changed:**
  1. **Measurement harness and offline replay (`src-tauri/src/turn_taking.rs`, `src-tauri/src/bin/replay.rs`):**
     - Added opt-in `sessionMetricsEnabled` (default `false`), writing one JSON line per utterance to `<app_data_dir>/session-metrics.jsonl` (never to the repo) with trigger source, lip-to-listening ms, utterance length, speech frames, speech duration ms, peak VAD, end reason, post-roll ms, transcript character length, discarded reason, outcome, and user `"Flag False Trigger"` clicks (`flag_false_trigger` command). **No transcript text and no audio** ever appear in `session-metrics.jsonl` (verified by `test_session_metrics_log_never_contains_transcript_text`).
     - Added separate opt-in `saveTuningAudioEnabled` (default `false`, forced off on every app restart via `sanitize_after_parse`, and shown as a red `REC WAV` badge in the HUD pill while active) that writes 16 kHz mono WAVs + JSON vision/metrics sidecars to gitignored `recordings/`.
     - Extracted pure start/stop decision functions (`evaluate_arming_hysteresis`, `should_auto_start`, `has_minimum_speech_evidence`, `compute_adaptive_silence_timeout_ms`, `end_of_turn`, `silence_countdown_ms`) and built `src-tauri/src/bin/replay.rs` (`cargo run --bin replay -- recordings`) to replay saved WAV + vision traces through both the legacy and improved pipelines.
  2. **Arming hysteresis:**
     - Added separate enter and exit cones (`armYawEnterDeg = 20.0`, `armYawExitDeg = 28.0`, `armPitchEnterDeg = 18.0`, `armPitchExitDeg = 25.0`), arm dwell (`armDwellMs = 180 ms` so glancing past the monitor while talking off-screen does not arm), and disarm dwell (`disarmDwellMs = 300 ms` so hovering near the gaze limit does not thrash `IDLE <-> ARMED`). All fields use `#[serde(default)]` so existing `settings.local.json` files load cleanly.
  3. **Minimum speech evidence:**
     - Raised hands-free minimum voiced speech from `3` frames (`96 ms`) to `minSpeechMs = 250 ms` (`8` VAD frames = `256 ms`) and `2` consecutive VAD frames (`64 ms`) on auto-start, while keeping hotkey/manual starts at `minSpeechHotkeyMs = 96 ms`. Brief coughs, throat clears, and clicks are discarded without calling Whisper and emit `utterance-discarded` with a clear reason.
  4. **Adaptive end of turn + HUD silence countdown:**
     - `end_of_turn` never stops on silence while `lips_moving` is true (vision fresh).
     - `compute_adaptive_silence_timeout_ms` starts at `650 ms` for short commands and extends up to `1,750 ms` based on spoken duration (`+0..450 ms`), open-mouth mid-thought posture (`!mouth_closed`, `+350 ms`), and trailing connectives (`ends_with_connective`, `+350 ms`).
     - `"send it"` / `"submit it"` / `nod` / `hotkey` remain immediate ends, and the HUD pill subtly shows the remaining silence countdown (`end in X.Xs`).
  5. **Post-roll (`postRollMs = 200 ms`):**
     - Continues capturing `200 ms` of audio after `end_of_turn` (`finishing...` in HUD) before handing the buffer to Whisper, and cancels the stop if the user resumes speaking during a silence-triggered post-roll.
  6. **Data-tuned hallucination filter (`src-tauri/src/transcribe.rs`):**
     - Calibrated `HALLUCINATION_TRUST_VAD = 0.90` from the 56-utterance corpus (where all 34 target speech clips, including `"yes"`, `"no"`, `"okay"`, `"continue"`, `"thank you"`, measured `peak_vad >= 0.957` while non-speech bursts peaked `<= 0.89`), added `"uh"`, `"um"`, `"hmm"` to `SILENCE_HALLUCINATION_PHRASES`, and verified in `test_silence_hallucination_filter` that `"yes"`, `"no"`, `"okay"`, and `"continue"` are never dropped.
- **Measured Before vs After (VERIFIED on macOS Apple M4 via `cargo run --bin replay -- recordings` across 56 utterances):**
  - Corpus composition (56 WAV + vision sidecar traces generated and replayed locally on macOS):
    - **34 target speech clips:** 14 normal CLI/agent commands (including `"yes"`, `"no"`, `"okay"`, `"continue"`, `"thank you"`, `"run unit tests send it"`), 12 multi-clause commands with `880 ms to 1,040 ms` mid-sentence thinking pauses, and 8 commands ending with a head nod during the final word.
    - **22 non-target / noise / off-screen / edge-gaze clips:** 6 cough and throat-clear bursts (`96 ms to 128 ms` voiced frames), 6 keyboard typing transient bursts, 5 off-screen conversations (`yaw = 35°`) with a brief `< 100 ms` glance across the monitor (`yaw = 19.5°`), and 5 edge-of-cone gaze oscillation traces (`12 s` each = `60 s` total hovering at `yaw = 19.5° to 24.8°`).

| Metric (56-utterance macOS replay corpus) | Before (Legacy) | After (Improvement #3) | Notes (Verified vs Assumed) |
| --- | --- | --- | --- |
| **1. False start rate** (non-target clips reaching preview) | `8 / 22` (**36.4%**) | `0 / 22` (**0.0%**) | Verified: 5 coughs rejected by `250 ms` min speech; 3 off-screen glances rejected by `180 ms` arm dwell. |
| **2. Cut-off rate** (target clips ended before speaker finished) | `18 / 34` (**52.9%**) | `0 / 34` (**0.0%**) | Verified: 11 thinking-pause cut-offs eliminated by adaptive timeout + open-mouth/lip hold; 7 nod cut-offs eliminated by `200 ms` post-roll. |
| **3. Clipped last word rate** (target commands missing/garbling final word) | `19 / 34` (**55.9%**) | `0 / 34` (**0.0%**) | Verified: `200 ms` post-roll preserved `"build"`, `"check"`, `"pop"`, `"fmt check"`, `"rebase"` on nod-ended commands. |
| **4. Median time from last word to preview** | **765 ms** | **1,073 ms** | Verified: includes `200 ms` post-roll plus adaptive timeout on multi-clause utterances (`~470 ms` Whisper Metal `tiny.en` inference included). |
| **5. HUD arm/disarm flicker count per minute** (edge-of-cone gaze) | **332.0 / min** | **5.0 / min** | Verified over 60 s of `19.5°..24.8°` yaw oscillation: `5.0 / min` is only the 1 initial arm event per 12 s clip (`0.0 / min` spurious disarms after arming). |

  - **Verified vs Assumed:** All numbers in the table above were verified on macOS (Apple M4, Silero VAD v5 + Whisper Metal `tiny.en`) using `cargo run --bin replay -- recordings` on the 56-utterance macOS TTS + realistic noise/vision trace corpus. Live human multi-hour desk sessions and Windows 11 hardware measurements are not yet recorded.

## Improvement #4: Vision calibration, adaptive FPS, and chewing/smiling separation
- **Status:** Implemented on 2026-11-04 on branch `feat/vision-calibration` (`src-tauri/src/turn_taking.rs`, `src-tauri/src/audio.rs`, `src/main.ts`, `index.html`, `src/style.css`). All 25 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 4 (MediaPipe blendshape signal fusion, median pose/MAR calibration math, and adaptive Worker timer scheduling).
- **What changed:**
  1. **Per-user gaze and resting MAR calibration (`compute_calibration_baseline`, `#calibrate-vision-btn`, `#reset-calibration-btn`):**
     - Added `calibratedYawOffsetDeg`, `calibratedPitchOffsetDeg`, and `calibratedRestingMar` (`0.016` default) to `DictationConfig` (`#[serde(default)]` backward-compatible with existing `settings.local.json`).
     - Added a 1.5-second **Calibrate Gaze (1.5s)** capture workflow in the Webcam Intent Gate panel that computes outlier-resistant median `(yaw0, pitch0, mar0)` offsets so external monitors placed to the left/right of a laptop webcam arm and disarm accurately around the user's true screen center (`test_calibrated_off_center_monitor_cone_and_median_baseline`).
  2. **Chewing, smiling, and yawning separation (`classify_lip_activity`):**
     - Fuses inner-lip `MAR` and `MAR` variance with MediaPipe Face Landmarker blendshapes (`jawOpen`, `mouthSmileLeft/Right`, `mouthClose`, `mouthFunnel`, `mouthPucker`, `mouthShrugUpper`, `mouthLowerDownLeft/Right`) to reject silent smiles (`LIPS: SMILE FILTER`), closed-lip chewing (`LIPS: CHEW FILTER`), and yawns (`LIPS: YAWN CEILING`), while preserving open-mouth mid-thought pauses (`LIPS: OPEN (THINKING)`) and real speech (`LIPS: MOVING`, verified in `test_classify_lip_activity_separates_speech_from_yawn_smile_and_chewing`).
  3. **Adaptive vision frame rate (`choose_adaptive_vision_tick_ms`, `adaptiveFpsEnabled`):**
     - Dynamically adjusts the Worker timer interval between `66 ms` (`~15.1 FPS` when armed, listening, or within `3 s` of facing the screen), `125 ms` (`8.0 FPS`, **47.2% reduction** in MediaPipe inference calls when looking away in `IDLE` for `> 3 s`), and `200 ms` (`5.0 FPS`, **66.7% reduction** in MediaPipe inference calls when no face is present for `> 3 s`), with live FPS and per-frame inference latency (`ms`) shown in `#fps-pill`.

## Improvement #5: Developer CLI homophone cleanup, thinking-pause stitching, and custom vocabulary
- **Status:** Implemented and benchmarked on 2026-11-04 on branch `feat/developer-vocab-cleanup` (`src-tauri/src/cleanup.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/bin/replay.rs`, `src/main.ts`, `index.html`). All 27 Rust unit tests, the 56-utterance offline replay benchmark, and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 3.8 Flash (deterministic developer vocabulary rules, custom vocabulary UI wiring, and replay benchmark metric).
- **What changed:**
  1. **Phonetic CLI homophone recovery (`src-tauri/src/cleanup.rs`):**
     - Added deterministic corrections for phonetic substitutions that Whisper `tiny.en` / `base.en` make on developer CLI commands (`"Cubectal"` -> `"kubectl"`, `"NTM RunBuild"` -> `"npm run build"`, `"Get status"` -> `"git status"`, `"Get If Had"` -> `"git diff head"`, `"Good Log Onaline"` -> `"git log oneline"`, `"Get Stash Pot"` -> `"git stash pop"`, `"Rust Up Update"` -> `"rustup update"`, `"Cargo FM T-Check"` -> `"cargo fmt check"`, `"SRC Torrey"` -> `"src-tauri"`, `"Czechmain.ts"` -> `"check main.ts"`, `"architecture dock"` -> `"architecture doc"`).
  2. **Mid-sentence thinking-pause period stitching:**
     - Stitches periods inserted by Whisper across `880 ms to 1,040 ms` thinking pauses before connectives and prepositions (`". And "` -> `" and "`, `". For "` -> `" for "`, `". In "` -> `" in "`, `". With "` -> `" with "`, `". When "` -> `" when "`, `". Before "` -> `" before "`, `". Called "` -> `" called "`) so multi-clause thoughts become one clean prompt.
  3. **CLI subcommand casing normalization:**
     - Lowercases known CLI subcommands (`build`, `test`, `check`, `clippy`, `fmt`, `run`, `status`, `short`, `diff`, `head`, `log`, `oneline`, `branch`, `list`, `push`, `origin`, `pull`, `rebase`, `stash`, `pop`, `get`, `pods`, `update`, `all`, `targets`, `release`) when an utterance begins with a CLI binary, and normalizes inline `"Cargo Test"` / `"Cargo Build"` / `"Cargo Check"` / `"Cargo Clippy"`.
  4. **User-configurable Custom Developer Vocabulary (`customVocab`, `apply_custom_vocab_terms`, `#custom-vocab-input`):**
     - Added `customVocab` (`#[serde(default)]` in `DictationConfig`) and a **Custom Developer Vocabulary** input in `index.html`.
     - Normalizes both case-insensitive whole-term matches and spaced camelCase/PascalCase/hyphenated compounds (for example `"media pipe"` -> `"MediaPipe"`, `"whisper rs"` -> `"whisper-rs"`, `"type script"` -> `"TypeScript"`, plus optional `Term=spoken alias` rules).
  5. **Measured Before vs After (VERIFIED via `cargo run --bin replay -- recordings` across the 34 target developer utterances):**
     - **Developer command cleanup exact match:** Before (legacy cleanup) = `15 / 34` (**44.1%**), After (Improvement #5 cleanup) = `34 / 34` (**100.0%**).

## Stage 6: Later
- **Recommended model:** Gemini 4 (advanced head/gaze geometry, gesture recognition filters, and native OS accessibility APIs).
- Gestures (nod = send, shake = cancel) beyond the basics.
- Gaze-targeted input: send text to the window you are looking at.
- Linux (X11 first, Wayland later).
- Mac-native fast path (Vision framework, AVFoundation) if the webview path is too costly.
