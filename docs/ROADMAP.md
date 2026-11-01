# Roadmap

Each stage has an exit criterion. Do not start the next stage until the current one passes, or the reason for skipping is written down.

## Stage 0: Prior-art search

- Work through the checklist in [PRIOR_ART.md](PRIOR_ART.md).
- Exit: a written summary of who has built webcam-gated or lip-triggered voice input, how it worked, and what is left that is worth building.

## Stage 1: Feasibility spike

- Minimal page running MediaPipe Face Landmarker, plotting MAR and head yaw live.
- Test at the real desk: with and without glasses, different lighting (day, night, backlit), different camera angles and distances.
- Record labeled sessions: talking, silent, yawning, chewing, smiling, turned away.
- Test whether a hidden Tauri webview keeps the camera and WebGL running.
- Measure CPU at 10 to 15 FPS.
- **Exit:** at least 90% separation of talking vs silent/yawn/chew/turned away, at under about 10% CPU.

## Stage 2: Handy-core dictation

- Rebuild the basics, studying Handy for patterns: cpal capture, Silero VAD, whisper.cpp / Parakeet, text injection, manual hotkey.
- Exit: reliable hotkey dictation into a terminal on macOS.

## Stage 3: Gated auto start/stop

- Wire the visual gate into the state machine (IDLE > ARMED > LISTENING > TRANSCRIBING).
- Ring buffer so the first syllable is not clipped.
- Exit: start and stop work hands-free with a measured false-trigger rate and missed-start rate over a normal work session.

## Stage 4: Cleanup, preview, auto-Enter

- AI cleanup pass (local or BYOK) that keeps code identifiers, paths and flags intact.
- Cancelable preview with countdown.
- Auto-Enter with per-app rules (for example: auto-send in terminals and agents, paste-only in chat apps and email).
- Exit: a full day of real use with zero unwanted sends.

## Stage 5: Windows, polish, go public

- Windows support (WebView2, injection quirks).
- Onboarding, camera permission UX, settings.
- Record the 7-second demo GIF.
- Make the repo public.

## Stage 6: Later

- Gestures (nod = send, shake = cancel) beyond the basics.
- Gaze-targeted input: send text to the window you are looking at.
- Linux (X11 first, Wayland later).
- Mac-native fast path (Vision framework, AVFoundation) if the webview path is too costly.
