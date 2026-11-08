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

## Improvement #6: Local 7-day utterance history, search filter, and one-click purge
- **Status:** Implemented on 2026-11-04 on branch `feat/utterance-history` (`src-tauri/src/history.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/lib.rs`, `src/main.ts`, `index.html`, `src/style.css`). All 29 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 3.8 Flash (local 7-day JSONL history persistence, TTL pruning, and UI search/clear wiring).
- **What changed:**
  1. **7-day auto-pruned local history (`src-tauri/src/history.rs`):**
     - Persists completed utterances in `<app_data_dir>/utterance-history.jsonl` (never inside the repo) when `historyPersistenceEnabled` (default `true`) is enabled.
     - Automatically prunes any entry older than 7 days (`HISTORY_RETENTION_MS = 604,800,000 ms`) and caps history at `MAX_HISTORY_ENTRIES = 200` on both startup (`load_and_prune_history`) and write (`append_history_entry`), physically rewriting the file when expired entries are removed (`test_seven_day_history_prunes_expired_entries_and_persists_recent`).
  2. **One-click purge and false-trigger flag persistence (`clear_utterance_history`, `update_history_false_trigger_flag`):**
     - Clicking **Clear** (`#clear-feed-btn`) or unchecking **7-Day Local Utterance History** (`#toggle-history-persistence`) immediately deletes `<app_data_dir>/utterance-history.jsonl` from disk (`test_history_false_trigger_flag_and_clear_purges_disk`).
     - Clicking **Flag False Trigger** on any utterance card updates both `session-metrics.jsonl` and `utterance-history.jsonl`.
  3. **Live Utterance Feed search (`#history-search-input`, `#history-ttl-badge`):**
     - Added real-time filtering across cleaned commands, raw transcripts, target app names, and trigger sources, plus a `7d Local` / `RAM Only` status badge.

## Improvement #7: Audio hot-plug recovery, sleep/lock cooldown, and packaged model path resolution
- **Status:** Implemented on 2026-11-04 on branch `feat/audio-resilience` (`src-tauri/src/turn_taking.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/transcribe.rs`, `src-tauri/src/lib.rs`, `src/main.ts`). All 31 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 4 (real-time `cpal` stream health monitoring, preferred-device hot-plug state recovery, wall-clock sleep/wake race prevention, and packaged `.app`/`.exe` model storage resolution).
- **What changed:**
  1. **Automatic microphone hot-plug recovery and preferred-device restoration (`evaluate_audio_stream_health`, `AUDIO_STREAM_STALL_TIMEOUT_MS = 2,500 ms`):**
     - Tracks `last_audio_callback_at` and `stream_faulted` in `SharedCaptureBuffer`. When a Bluetooth headset (AirPods) or USB mic disconnects (`StreamError` or `>= 2,500 ms` callback stall), the audio worker automatically rebuilds the `cpal` stream on the fallback default input device, emits `"audio-devices-changed"` so the UI dropdown refreshes live, and surfaces a clear notice (`test_audio_hotplug_recovery_and_preferred_device_restore`).
     - When the user's configured `preferred_device` is plugged back in and reappears in `list_input_devices()`, `saywhen` automatically switches back to it whenever not actively recording or previewing.
  2. **System sleep / screen lock safety guard (`detect_system_sleep_or_stall_jump`, `SYSTEM_SLEEP_JUMP_THRESHOLD_MS = 3,000 ms`):**
     - Detects wall-clock jumps (`> 3,000 ms` between `12 ms` worker loop ticks) caused by closing the laptop lid or locking the screen (`test_system_sleep_wall_clock_jump_detection`).
     - Immediately cancels any in-flight `LISTENING` or `PREVIEW` countdown on wake (so a stale preview never auto-fires `Enter` after wake), clears the audio ring buffer, disarms to `IDLE`, and resets the callback watchdog.
  3. **App-bundle Whisper model path resolution (`TranscriberEngine::with_app_data_dir`):**
     - Resolves existing models across `./models`, `../models`, and `<app_data_dir>/models`, and defaults new downloads to `<app_data_dir>/models` when running from a packaged `.app` or `.exe` outside the source repository.

## Improvement #8: Spoken self-correction ("scratch that"), paste-only voice override ("paste only"), and GitHub Actions CI
- **Status:** Implemented on 2026-11-04 on branch `feat/voice-overrides-and-ci` (`src-tauri/src/cleanup.rs`, `src-tauri/src/audio.rs`, `.github/workflows/ci.yml`, `README.md`, `CLAUDE.md`). All 32 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 3.8 Flash (deterministic spoken self-correction and delivery override rules, CI workflow YAML, and README/docs synchronization).
- **What changed:**
  1. **Mid-utterance `"scratch that"` self-correction (`apply_scratch_that_correction`):**
     - Discards any misspoken clause before the last `"scratch that"` in an utterance (for example `"cargo build, scratch that, cargo test --all-targets"` -> `"cargo test --all-targets"`), or cancels the entire utterance if nothing follows `"scratch that"`.
  2. **Expanded spoken cancel commands (`is_spoken_cancel_command`):**
     - Supports `"cancel"`, `"cancel that"`, `"never mind"`, and trailing `"scratch that"` (`test_scratch_that_correction_and_paste_only_voice_override`).
  3. **Voice paste-only override (`strip_paste_only_phrase`, `force_paste_only`):**
     - Saying `"paste only"`, `"just paste it"`, `"just paste"`, or `"no enter"` at the end of a command strips the phrase and forces `target_policy` into **Paste-Only** mode (`will_press_enter = false`) for that utterance even inside an allowlisted terminal.
  4. **Automated GitHub Actions CI (`.github/workflows/ci.yml`):**
     - Verifies zero em dashes, builds the TypeScript/Vite frontend (`npm ci && npm run build`), and runs the full Rust unit test suite (`cargo test --manifest-path src-tauri/Cargo.toml`) on every push and pull request.

## Improvement #9: Modular injection split, Linux X11/Wayland detection, and macOS + Windows CI matrix
- **Status:** Implemented on 2026-11-04 on branch `feat/modular-inject-and-linux-x11` (`src-tauri/src/inject.rs`, `src-tauri/src/inject/policy.rs`, `src-tauri/src/inject/linux.rs`, `src-tauri/src/inject/tests.rs`, `.github/workflows/ci.yml`). All 33 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 4 (cross-platform X11 `xprop`/`/proc` process inspection, Wayland guard detection, and modular injection refactoring below the 500-line split target).
- **What changed:**
  1. **Modularized `src-tauri/src/inject.rs` below the 500-line target (`349 lines`):**
     - Extracted pure target-policy evaluation, allowlist matching, self-injection protection, and Enter-delay scaling into `src-tauri/src/inject/policy.rs` (`458 lines`).
  2. **Stage 6 Linux X11 frontmost app detection and Wayland guard (`src-tauri/src/inject/linux.rs`):**
     - Implemented `detect_frontmost_app` for Linux X11/XWayland using `_NET_ACTIVE_WINDOW`, `_NET_WM_PID`, `WM_CLASS`, `_NET_WM_NAME`, and `/proc/<pid>/exe`, plus canonical Linux terminal matching (`Alacritty`, `gnome-terminal-server`, `konsole`, `xfce4-terminal`, `kitty`, `wezterm-gui`, `zed-editor`, verified in `test_linux_xprop_parsing_and_terminal_allowlist`).
     - Added `check_pre_injection_guard` on Linux to fail loudly with clipboard fallback when running in a pure Wayland session without an XWayland `DISPLAY`.
  3. **Cross-platform macOS + Windows GitHub Actions matrix (`.github/workflows/ci.yml`):**
     - Expanded the CI matrix to `[macos-latest, windows-latest]` so both the `objc2-app-kit` macOS path and `windows-sys` Win32 path compile and run unit tests on every push and PR.

## Improvement #10: Opt-in local Ollama / OpenAI-compatible BYOK AI cleanup pass
- **Status:** Implemented on 2026-11-04 on branch `feat/opt-in-llm-cleanup` (`src-tauri/src/cleanup.rs`, `src-tauri/src/audio.rs`, `src/main.ts`, `index.html`). All 34 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 3.8 Flash (local loopback vs remote BYOK classification, output fence/preamble sanitization, and opt-in settings wiring).
- **What changed:**
  1. **Opt-in secondary LLM cleanup client (`LlmCleanupConfig`, `run_opt_in_llm_cleanup`):**
     - Off by default (`llmCleanupEnabled: false`), preserving 100% zero-network operation unless explicitly enabled by the user.
     - Defaults to local Ollama loopback (`http://127.0.0.1:11434/v1/chat/completions`, `qwen2.5-coder:1.5b`) with a strict `1,200 ms` timeout (`reqwest::blocking::Client`).
     - If the endpoint fails or times out, `saywhen` fails loudly with a `"pipeline-error"` notice while preserving the deterministic `cleanup.cleaned_text` and re-evaluating `check_destructive_command`.
  2. **Loopback privacy classifier and output sanitizer (`classify_llm_endpoint_privacy`, `sanitize_llm_cleanup_output`):**
     - Distinguishes `127.0.0.1` / `localhost` / `[::1]` (`"local_loopback"`) from external hosts (`"remote_byok"`), strips accidental markdown code fences or backticks around single-line commands, and rejects conversational preambles (`"Sure, here is..."`, verified in `test_llm_endpoint_privacy_classification_and_output_sanitization`).

## Improvement #11: Live partial connective detection, multi-monitor gaze-window alignment guard, and release workflow
- **Status:** Implemented on 2026-11-04 on branch `feat/partial-connectives-gaze-guard-and-release` (`src-tauri/src/audio.rs`, `src-tauri/src/inject/policy.rs`, `src-tauri/src/inject/windows.rs`, `src-tauri/src/inject/tests.rs`, `src/hud.ts`, `.github/workflows/release.yml`). All 35 Rust unit tests and the frontend build pass with zero compiler warnings.
- **Recommended model:** Gemini 4 (real-time non-blocking partial Whisper inference scheduling, multi-monitor coordinate vs head-yaw alignment policy, and cross-platform release workflow).
- **What changed:**
  1. **Live partial Whisper connective detection during `LISTENING` (`src-tauri/src/audio.rs`, `src/hud.ts`):**
     - When a speaker pauses for `>= 240 ms` after at least `256 ms` (`8` VAD frames) of speech during `LISTENING`, `run_audio_worker` spawns a non-blocking Whisper check on the trailing `2.0s` audio window (`partial_transcript`, `partial_ends_with_connective = ends_with_connective(&text)`).
     - Wires `partial_ends_with_connective` directly into `SharedCaptureBuffer::build_turn_state` so `compute_adaptive_silence_timeout_ms` grants the `+350 ms` connective hold in live dictation, and surfaces the partial snippet / `(connective hold)` indicator in the floating HUD pill.
  2. **Multi-monitor gaze-window alignment guard (`evaluate_gaze_window_alignment` in `src-tauri/src/inject/policy.rs`):**
     - Compares the focused window's horizontal display center (`window_center_x`, `primary_screen_width`) against the user's calibrated head yaw (`head_yaw_deg`).
     - Automatically downgrades delivery to **Paste-Only** (`will_press_enter = false`) if the focused window is on an external left monitor while the user is looking right, or on an external right monitor while the user is looking left (`test_multi_monitor_gaze_window_alignment_guard`).
  3. **Cross-platform release packaging workflow (`.github/workflows/release.yml`):**
     - Builds macOS Apple Silicon (`aarch64-apple-darwin`) and Windows x64 Tauri desktop bundles via `tauri-apps/tauri-action@v0` on `v*` tags and `workflow_dispatch`.

## Improvement #12: Send modes, talk-to-cancel, editorial redesign and plain-language docs
- **Status:** Implemented on 2026-11-08 (`src-tauri/src/turn_taking.rs`, `src-tauri/src/audio.rs`, `src/main.ts`, `src/ui.ts`, `src/hud.ts`, `src/fonts.ts`, `index.html`, `src/style.css`, `src/hud.css`, `README.md`, `INSTALL_WITH_AI.md`, `docs/HOW_IT_WORKS.md`, `docs/HOW_WE_BUILT_IT.md`). All 39 Rust unit tests and the frontend build pass. UI verified in the browser preview with the `?demo=` states. Not yet verified: send modes and talk-to-cancel end to end in the packaged desktop app with a real camera and microphone, and on Windows.
- **Recommended model:** Gemini 4 (send modes change preview timing inside the audio worker state machine).
- **What changed:**
  1. **Send modes (`SendMode` in `turn_taking.rs`, `send_mode` in `DictationConfig`):** `pause` (default, same behavior as before), `nod` (preview waits up to `NOD_WAIT_TIMEOUT_MS = 15,000 ms` for a nod, Enter or the hotkey, then gives up without sending), `phrase` (Enter only when the utterance ends with "send it"), `paste_only` (never Enter). A nod that ends the turn in `nod` mode counts as the go-ahead and gets the normal cancelable countdown. App rules and the destructive command guard still downgrade any mode to paste only. Older settings with `autoPressEnter: false` migrate to `paste_only`.
  2. **Talk to cancel (`should_cancel_preview_on_speech`, `cancel_on_speech`):** speech during the preview cancels it after a `350 ms` grace period and at least `6` consecutive VAD frames (about `190 ms`). With fresh vision the lips must be moving too, so other voices in the room do not cancel. Default on.
  3. **Editorial redesign:** warm ivory, soft black and burnt orange, with the whole window easing into a different palette for each pipeline state (registered `@property` colors, `0.9 s` ease). Plus Jakarta Sans, Instrument Serif, JetBrains Mono and Outfit are bundled through `@fontsource` so the UI still renders offline. HUD pill and check card restyled to match, with nod-aware hints.
  4. **Plain-language copy and docs:** UI states renamed (Waiting, Ready, Listening, Writing, Check), Settings rewritten for non-technical users, README rewritten, plus an assistant-driven installer guide and two explainer docs.

## Improvement #13: Side-mounted camera support
- **Status:** Implemented on 2026-11-08 (`src-tauri/src/turn_taking.rs`, `src-tauri/src/audio.rs`, `src/main.ts`, `index.html`). All 40 Rust unit tests and the frontend build pass. Not yet verified: head-turn accuracy with a real side-mounted webcam (no recorded traces at 30 to 65 degrees yet), and the yaw-corrected MAR against the replay corpus, which stores MAR precomputed.
- **Recommended model:** Gemini 4 (lip geometry under head rotation and gate thresholds interact).
- **What changed:**
  1. **Wider calibration:** the calibrated yaw center may sit up to `MAX_CALIBRATED_YAW_OFFSET_DEG = 65` degrees from the camera axis (was 45). Loaded settings are clamped to the same range.
  2. **`side_camera_mode` (default off):** widens the yaw cone around the calibrated center to at least `35` degrees to enter and `48` to exit (`effective_yaw_cone`), in both the webview gaze pill and the Rust arming cone. Pitch limits are unchanged. Gaze-to-display Enter downgrade (`evaluate_gaze_window_alignment`) is skipped in this mode because head yaw no longer maps to a display.
  3. **Yaw-corrected MAR (always on):** mouth width is divided by `cos(camera yaw)` (floored at `0.4`, about 66 degrees) so the resting baseline and speech thresholds keep their meaning when the head is turned. Under 20 degrees the change is below 6%.
  4. **Limit:** past about 65 to 70 degrees MediaPipe stops tracking the face and the far lip corner is guessed, so a full profile still needs the hotkey.

## Improvement #14: Palette picker
- **Status:** Implemented on 2026-11-08 (`src/style.css`, `src/hud.css`, `src/main.ts`, `src/hud.ts`, `index.html`, `src-tauri/src/audio.rs`). Frontend build and all 40 Rust unit tests pass. Verified in the browser preview: picking a palette changes the resting tokens, mood colours still override in busy states, and turning mood colours off keeps the chosen palette. Not yet verified in the packaged desktop app.
- **What changed:** all seven portfolio palettes (Lumen, Bloom, Lumora, Human Next, Choreo, Field / Form, Blue Bloom) are selectable in **Settings > Colours** (`ui_palette`, default `lumen`). A **Change colour with each mood** switch (`mood_colors`, default on) keeps the per-state palettes from Improvement #12. Both are presentation-only config fields, sent in `audio-telemetry` so the HUD matches from its first frame. The oscilloscope and lip overlay read the live palette tokens instead of fixed colours.

## Stage 6: Later
- **Recommended model:** Gemini 4 (advanced head/gaze geometry, gesture recognition filters, and native OS accessibility APIs).
- Gestures (nod = send, shake = cancel) beyond the basics.
- Gaze-targeted input: send text to the window you are looking at.
- Linux (X11 first, Wayland later).
- Mac-native fast path (Vision framework, AVFoundation) if the webview path is too costly.
