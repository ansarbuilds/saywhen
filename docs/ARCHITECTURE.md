# Architecture

Status: draft, pre-prototype. Everything here is a plan, not a measured result.

## Stack

- **Shell:** Tauri v2 (Rust backend, TypeScript frontend in the system webview).
- **Vision:** MediaPipe Tasks Vision (Face Landmarker + Hand Landmarker) running in the webview via WASM/WebGL. Chosen because it works the same on macOS, Windows and Linux without native per-platform vision code.
- **Audio capture:** `cpal` in Rust.
- **Voice activity detection:** Silero VAD (ONNX).
- **Transcription:** whisper.cpp (via Rust bindings) and NVIDIA Parakeet, user-selectable. Models downloaded on first run, never committed.
- **AI cleanup:** local LLM or bring-your-own-key provider. Optional, off by default until configured.
- **Text injection:** `enigo` for keystrokes, clipboard paste as the default path for longer text, then a synthetic Enter.
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
