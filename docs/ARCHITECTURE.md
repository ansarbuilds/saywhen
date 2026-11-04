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

## State machine

```
IDLE > ARMED > LISTENING > TRANSCRIBING > PREVIEW > SENT
                                                  > CANCELED
```

| State | Entered when | Leaves when |
| --- | --- | --- |
| IDLE | App running, no face or not facing screen | Face present and facing screen -> ARMED |
| ARMED | Face present and facing screen | Lips moving + speech over ~300ms -> LISTENING; face lost -> IDLE |
| LISTENING | Trigger satisfied; ring buffer prepended | Silence + closed mouth ~500 to 800ms, end phrase, or nod -> TRANSCRIBING; shake -> CANCELED |
| TRANSCRIBING | Audio segment closed | Text ready (after cleanup) -> PREVIEW; error -> fail loudly, back to ARMED |
| PREVIEW | Cleaned text shown with countdown | Timeout or nod -> SENT; shake, Esc, "cancel", look away -> CANCELED |
| SENT | Text injected and Enter pressed | Immediately -> ARMED |
| CANCELED | User canceled | Immediately -> ARMED |

A manual hotkey can force LISTENING at any time, as an accessibility and fallback path.

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
