# Architecture

Status: draft, pre-prototype. Everything here is a plan, not a measured result.

## Stack

- **Shell:** Tauri v2 (Rust backend, TypeScript frontend in the system webview).
- **Vision:** MediaPipe Tasks Vision (Face Landmarker + Hand Landmarker) running in the webview via WASM/WebGL. Chosen because it works the same on macOS, Windows and Linux without native per-platform vision code.
- **Audio capture:** `cpal` in Rust.
- **Voice activity detection:** Silero VAD (ONNX).
- **Transcription:** whisper.cpp (via Rust bindings) and NVIDIA Parakeet, user-selectable. Models downloaded on first run, never committed.
- **AI cleanup:** local LLM or bring-your-own-key provider. Optional, off by default until configured.
- **Text injection:** `enigo` for keystrokes (`src-tauri/src/inject.rs`, `inject/macos.rs`, `inject/windows.rs`), clipboard paste via `arboard` as the default path, then a synthetic Enter after a length-scaled delay.
- **Storage:** SQLite for transcript history, auto-purged after 7 days. Settings in a local config file.

## Pipeline

```
webcam -> webview (MediaPipe) -> face/gaze/MAR features --+
                                                         |
mic -> cpal -> ring buffer -> Silero VAD ----------------+--> intent gate (Rust)
                                                         |
                                        LISTENING: audio segment
                                                         |
                                       whisper.cpp / Parakeet
                                                         |
                                            AI cleanup pass
                                                         |
                                       preview + cancel window
                                                         |
                                    inject text + Enter (enigo)
```

The webview sends only derived numbers (landmark-based features such as MAR, yaw, pitch, face present, gesture events) to Rust over Tauri IPC. Raw frames never leave the webview process memory.

### Windows and background run

- `main` window: settings, webcam preview and the vision loop. Closing it hides it; the app keeps running (tray menu or macOS Dock reopen brings it back). The vision loop is ticked by a dedicated Worker timer so it keeps running when the window is hidden, and skips DOM work while hidden.
- `hud` window: transparent, always-on-top, non-focusable and click-through (`set_ignore_cursor_events(true)`). It only listens to backend events (`audio-telemetry`, `preview-started`, `utterance-complete`, ...) and never takes focus, so the frontmost app stays the injection target.
- Because the HUD cannot be clicked, `src-tauri/src/hud.rs` registers a global `Esc` shortcut only while the pipeline is in `PREVIEW` and unregisters it right after, so saywhen never swallows `Esc` outside the countdown.
- The backend tracks whether the camera gate is switched on (`set_camera_gate_enabled`). Telemetry carries `cameraEnabled` and `visionActive`, and the HUD shows a stall warning when the gate is on but frames stopped arriving.

### Reliable text injection and safety guards

- **Native frontmost app detection (`FrontmostApp { name, bundle_id, exe, pid, detection_us }`):**
  - macOS (`src-tauri/src/inject/macos.rs`): queries `NSWorkspace.sharedWorkspace.frontmostApplication` via `objc2-app-kit` inside an autorelease pool (`1 us to 2 us` warm median latency over 20 calls, down from `8,336 us` with two `/usr/bin/lsappinfo` subprocesses).
  - Windows (`src-tauri/src/inject/windows.rs`): queries `GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageNameW` via `windows-sys` without spawning PowerShell.
- **Exact allowlist and self-injection guard:**
  - Matches exact app names (case-insensitive), macOS bundle identifiers (such as `com.apple.Terminal`, `com.googlecode.iterm2`, `dev.warp.Warp-Stable`, `com.mitchellh.ghostty`, `com.microsoft.VSCode`, `com.todesktop.230313mzl4w4u92`, `dev.zed.Zed`, `com.anthropic.claudefordesktop`), and Windows executable names (`WindowsTerminal.exe`, `Code.exe`, `Cursor.exe`, etc.). Substring matches (such as `"Xcode"` matching `"Code"`) are rejected.
  - `saywhen` itself (`dev.saywhen.desktop`, `saywhen.exe`, or matching PID) is removed from the default allowlist and unconditionally refused so `saywhen` never types into or presses Enter on its own windows.
- **Pre-injection target verification (`onTargetChanged: "abort" | "paste_only"`, default `"abort"`):**
  - Right before injecting at the end of the preview countdown, `saywhen` re-detects the frontmost app and compares PID, bundle ID, executable, and name against the preview target. If the user switched apps during the countdown, `saywhen` never presses Enter: it either aborts (leaving the transcript on the clipboard and surfacing an error) or pastes without Enter (`"paste_only"`).
- **Length-scaled Enter delay (`enterDelayMs` + `perAppEnterDelayMs`):**
  - Defaults to `120 ms` base delay plus `1 ms` per `20` characters, capped at `400 ms` (with optional per-app overrides), giving Electron apps and terminals time to process the clipboard paste before `Return` arrives.
- **Held-modifier protection and private event state:**
  - Uses `independent_of_keyboard_state: true` (`CGEventSourceStateID::Private` on macOS) and explicitly releases `Shift`, `Control`, `Alt`, and `Meta` (`LShift`/`RShift`/`LControl`/`RControl`) before pasting and before pressing `Return`.
- **Safe clipboard restoration (`clipboardRestoreDelayMs`, default `600 ms`):**
  - Snapshots plain text or RGBA image data (`arboard`) before writing the transcript. Waits `600 ms` (after Enter) and only restores the previous clipboard if the clipboard still equals the injected transcript, preventing race conditions and preserving user copies made during the window.
- **Loud failure guards:**
  - Detects macOS `IsSecureEventInputEnabled()` (Carbon) and Windows UIPI privilege mismatch (target process elevated while `saywhen` is not), plus 3-second main-thread dispatch timeouts (reporting honestly that injection may have happened late). Every failure path copies the transcript to the clipboard and emits both `pipeline-error` and `utterance-complete.injectionError`.

## State machine and gate

```
IDLE > ARMED > LISTENING > TRANSCRIBING > PREVIEW > SENT
                                                  > CANCELED
```

| State | Entered when | Leaves when |
| --- | --- | --- |
| IDLE | App running, no face or outside arming cone | Face present inside enter cone (`\|yaw\| <= 20°`, `\|pitch\| <= 18°`) for `armDwellMs` (`180 ms`) -> ARMED |
| ARMED | Face present and armed with hysteresis | Lips moving + sustained speech (`>= 2` VAD frames or recent voice within `320 ms`) inside exit cone -> LISTENING; outside exit cone (`\|yaw\| > 28°` or `\|pitch\| > 25°`) for `disarmDwellMs` (`300 ms`) -> IDLE |
| LISTENING | Trigger satisfied; `500 ms` pre-roll prepended | Adaptive silence (`650 ms` to `1,750 ms` once lips stop moving), end phrase (`"send it"` / `"submit it"`), or nod -> `200 ms` post-roll -> TRANSCRIBING (or discarded if `< 250 ms` speech); shake -> CANCELED |
| TRANSCRIBING | Post-roll completed and minimum speech evidence met | Text ready (after cleanup and hallucination filter) -> PREVIEW; empty/hallucinated -> `utterance-discarded` back to ARMED; error -> fail loudly |
| PREVIEW | Cleaned text shown with countdown | Timeout or nod -> SENT; shake, Esc, `"cancel"`, look away -> CANCELED |
| SENT | Text injected and Enter pressed | Immediately -> ARMED |
| CANCELED | User canceled | Immediately -> ARMED |

A manual hotkey (`Option + Space`) can force LISTENING at any time with a lower minimum-speech threshold (`96 ms` vs `250 ms`), as an accessibility and fallback path.

### Turn-taking rules (`src-tauri/src/turn_taking.rs`)

- **Arming hysteresis and per-user calibration (`evaluate_arming_hysteresis`, `compute_calibration_baseline`):**
  - Separate enter and exit cones prevent `IDLE <-> ARMED` thrashing near the gaze boundary: enters `ARMED` only after the face stays within `|yaw - calibratedYawOffsetDeg| <= 20.0°` and `|pitch - calibratedPitchOffsetDeg| <= 18.0°` (`armYawEnterDeg`, `armPitchEnterDeg`) for `armDwellMs = 180 ms` (so glancing across the monitor while talking to someone else does not arm).
  - Once armed, stays `ARMED` inside `|yaw - calibratedYawOffsetDeg| <= 28.0°` and `|pitch - calibratedPitchOffsetDeg| <= 25.0°` (`armYawExitDeg`, `armPitchExitDeg`) and disarms only after leaving that cone continuously for `disarmDwellMs = 300 ms`.
  - A 1.5-second **Calibrate Gaze (1.5s)** workflow in the main window records median `(calibratedYawOffsetDeg, calibratedPitchOffsetDeg, calibratedRestingMar)` for off-center external monitors and individual resting lip geometry.
- **Blendshape-assisted lip activity classification (`classify_lip_activity`):**
  - Combines inner-lip `MAR` and `MAR` variance with MediaPipe Face Landmarker blendshapes (`jawOpen`, `mouthSmileLeft`, `mouthSmileRight`, `mouthClose`, `mouthFunnel`, `mouthPucker`, `mouthShrugUpper`, `mouthLowerDownLeft`, `mouthLowerDownRight`) to separate active speech (`LIPS: MOVING`) from wide yawns (`LIPS: YAWN CEILING`), silent smiles (`LIPS: SMILE FILTER`), closed-lip chewing (`LIPS: CHEW FILTER`), and open-mouth thinking pauses (`LIPS: OPEN (THINKING)`).
- **Adaptive vision frame rate (`choose_adaptive_vision_tick_ms`, `adaptiveFpsEnabled`):**
  - Runs the Worker-driven MediaPipe loop at `15 FPS` (`66 ms`) whenever the pipeline is active, the user is facing the screen, or within `3 seconds` of looking away.
  - Scales down in `IDLE` to `8 FPS` (`125 ms`, `47.2%` fewer inferences) when a face is present but turned away for `> 3 s`, and `5 FPS` (`200 ms`, `66.7%` fewer inferences) when no face is in frame for `> 3 s`.
- **Multimodal auto-start (`should_auto_start`):**
  - Requires `ARMED`, fresh vision within the exit cone, `lips_moving == true`, and either `>= 2` consecutive 32 ms VAD frames (`64 ms`) above `vadThreshold` (`0.35`) or a VAD frame above threshold with another voiced frame within the last `320 ms`. Because the `500 ms` pre-roll ring buffer is prepended on transition, requiring `64 ms` of voiced evidence never clips the first syllable while rejecting single-frame (`32 ms`) keyboard clicks.
- **Minimum speech evidence (`has_minimum_speech_evidence`):**
  - Hands-free utterances require at least `minSpeechMs = 250 ms` (`8` VAD frames of `32 ms` = `256 ms`) of voiced speech before Whisper is called. Shorter sounds (coughs, throat clears, brief clicks) are discarded immediately and emit `utterance-discarded` with a reason (`Ignored brief sound (96 ms speech < 250 ms minimum)`). Hotkey-started utterances use `minSpeechHotkeyMs = 96 ms` (`3` frames).
- **Adaptive end of turn (`compute_adaptive_silence_timeout_ms`, `end_of_turn`, `silence_countdown_ms`):**
  - Never ends on silence while `lips_moving` is true and vision is fresh.
  - Starts from `silenceTimeoutMs = 650 ms` for short commands (`<= 900 ms` of speech) with a closed mouth, and extends up to `maxSilenceTimeoutMs = 1,750 ms` by adding:
    - up to `+450 ms` scaled with spoken duration (`(speech_ms - 900) / 3`) for multi-clause utterances,
    - `+350 ms` when the speaker's mouth stays open mid-thought (`!mouth_closed`), and
    - `+350 ms` when a partial transcript ends in a connective (`ends_with_connective`: `"and"`, `"the"`, `"to"`, `,`, etc.).
  - The HUD pill subtly displays the remaining silence countdown (`end in X.Xs`) once at least `250 ms` of silence has elapsed.
- **Post-roll (`postRollMs = 200 ms`):**
  - After `end_of_turn` triggers (`Silence` or `Nod`), the audio worker keeps capturing `200 ms` of trailing audio (`finishing...` in the HUD pill) before handing the buffer to Whisper so quiet final consonants and words spoken while nodding are not clipped. If the speaker resumes talking during a silence-triggered post-roll, the stop is canceled and `LISTENING` continues seamlessly.
- **Data-tuned hallucination filter (`src-tauri/src/transcribe.rs`):**
  - Whole-transcript stock hallucinations (`"thank you"`, `"thanks for watching"`, `"you"`, `"uh"`, `"um"`, `"hmm"`, etc.) are dropped when `peak_vad < 0.90` (`HALLUCINATION_TRUST_VAD`, tuned from the 56-utterance corpus where real speech measured `peak_vad >= 0.957` and non-speech bursts peaked `<= 0.89`). Short real replies (`"yes"`, `"no"`, `"okay"`, `"continue"`) are never dropped.
- **Opt-in measurement harness and offline replay (`src-tauri/src/bin/replay.rs`):**
  - `sessionMetricsEnabled` (default `false`): appends one JSON line per utterance to `<app_data_dir>/session-metrics.jsonl` containing timing, VAD, end reason, transcript character length, outcome, and false-trigger flags. **Never logs transcript text or audio.**
  - `saveTuningAudioEnabled` (default `false`, forced off on every app startup, shows a bright `REC WAV` badge in the HUD pill while active): saves 16 kHz mono WAV files and JSON vision/metrics sidecars to local `recordings/` (gitignored) for deterministic evaluation with `cargo run --bin replay`.
- **Developer cleanup pass and custom vocabulary (`src-tauri/src/cleanup.rs`):**
  - `run_local_developer_cleanup_with_vocab` runs in `< 50 us` locally with zero network latency:
    1. Strips leading and inline spoken fillers (`"uh"`, `"um"`, `"er"`, `"you know"`, `"basically"`).
    2. Corrects Whisper `tiny.en` / `base.en` phonetic CLI homophones (`"Cubectal"` / `"kube control"` -> `"kubectl"`, `"NTM RunBuild"` -> `"npm run build"`, `"Get status"` -> `"git status"`, `"Get If Had"` -> `"git diff head"`, `"Good Log Onaline"` -> `"git log oneline"`, `"Get Stash Pot"` -> `"git stash pop"`, `"SRC Torrey"` -> `"src-tauri"`, `"Czechmain.ts"` -> `"check main.ts"`, `"architecture dock"` -> `"architecture doc"`, `"say when"` -> `"saywhen"`).
    3. Converts spoken CLI flags and file extensions (`"dash dash"` -> `"--"`, `"dot slash"` -> `"./"`, `"dot rs"` -> `".rs"`, `"dot ts"` -> `".ts"`, `"dot toml"` -> `".toml"`, `"dot json"` -> `".json"`, etc.).
    4. Stitches mid-sentence thinking-pause periods before lowercase connectives and prepositions (`". And "` -> `" and "`, `". For "` -> `" for "`, `". In "` -> `" in "`, `". With "` -> `" with "`, `". When "` -> `" when "`, `". Before "` -> `" before "`, `". Called "` -> `" called "`).
    5. Normalizes capitalized CLI binaries (`cargo`, `git`, `npm`, `kubectl`, `rustup`, ...) and subcommands (`build`, `test`, `check`, `clippy`, `fmt`, `run`, `status`, `diff`, `log`, `oneline`, `stash`, `pop`, ...), and strips trailing periods from pure CLI commands.
    6. Applies user-configurable `customVocab` (`apply_custom_vocab_terms`, supporting both plain terms like `saywhen, MediaPipe, Silero, whisper-rs, TypeScript` and `Preferred=spoken alias` pairs) with whole-word and camelCase/hyphenated compound matching.

## Privacy design

- Camera frames are processed in memory and dropped. No frame is written to disk, logged or sent over the network.
- Only derived features cross the IPC boundary.
- Audio segments are discarded after transcription unless the user opts in to keeping them.
- Transcript history is stored locally in SQLite and purged after 7 days.
- No telemetry by default. Any network call (model download, BYOK cleanup) is explicit and visible in settings.
- A clear on-screen indicator whenever the camera is active, plus a one-click pause.

## Open questions

1. **Hidden webview:** does a hidden or minimized Tauri webview keep camera access and WebGL running at a steady frame rate on macOS (WKWebView) and Windows (WebView2), or does it get throttled?
2. **CPU and battery:** what does MediaPipe Face Landmarker cost at 10 to 15 FPS on a typical laptop? Target is under about 10% CPU.
3. **Best lips-moving feature:** MAR variance, landmark velocity, blendshape scores (jawOpen, mouthClose), or a small learned classifier? Which separates talking from yawning, chewing and smiling best?
4. **Camera permission UX:** how to ask for always-on camera access in a way that feels trustworthy, and how each OS surfaces the camera-in-use indicator.
5. **Auto-Enter reliability:** does clipboard paste plus synthetic Enter work reliably in terminals, IDEs and Electron apps? Where does it break (IME, secure input, remote desktops)?
