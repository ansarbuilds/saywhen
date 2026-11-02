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
- **Recommended model:** Gemini 4 (requires deep reasoning for Rust audio streaming with cpal, lock-free ring buffers, and thread safety).
- Rebuild the basics, studying Handy for patterns: cpal capture, Silero VAD, whisper.cpp / Parakeet, text injection, manual hotkey.
- Exit: reliable hotkey dictation into a terminal on macOS.

## Stage 3: Gated auto start/stop
- **Recommended model:** Gemini 4 (complex multimodal synchronization between vision IPC timestamps and audio buffer boundaries).
- Wire the visual gate into the state machine (IDLE > ARMED > LISTENING > TRANSCRIBING).
- Ring buffer so the first syllable is not clipped.
- Exit: start and stop work hands-free with a measured false-trigger rate and missed-start rate over a normal work session.

## Stage 4: Cleanup, preview, auto-Enter
- **Recommended model:** Gemini 3.8 Flash (fast iteration on preview UI components, countdown timers, and prompt engineering).
- AI cleanup pass (local or BYOK) that keeps code identifiers, paths and flags intact.
- Cancelable preview with countdown.
- Auto-Enter with per-app rules (for example: auto-send in terminals and agents, paste-only in chat apps and email).
- Exit: a full day of real use with zero unwanted sends.

## Stage 5: Windows, polish, go public
- **Recommended model:** Gemini 3.8 Flash (cross-platform configuration tweaks, docs, onboarding UI).
- Windows support (WebView2, injection quirks).
- Onboarding, camera permission UX, settings.
- Record the 7-second demo GIF.
- Make the repo public.

## Stage 6: Later
- **Recommended model:** Gemini 4 (advanced head/gaze geometry, gesture recognition filters, and native OS accessibility APIs).
- Gestures (nod = send, shake = cancel) beyond the basics.
- Gaze-targeted input: send text to the window you are looking at.
- Linux (X11 first, Wayland later).
- Mac-native fast path (Vision framework, AVFoundation) if the webview path is too costly.
