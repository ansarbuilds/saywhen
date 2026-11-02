# CLAUDE.md

## Project context

saywhen is a Tauri v2 + Rust + TypeScript desktop app for hands-free, hotkey-free voice input aimed at developers using coding agents and terminals. It gates listening on webcam signals (face present, facing the screen, lips moving) plus audio speech detection, transcribes locally, runs an AI cleanup pass, shows a short cancelable preview, injects the text into the focused app and presses Enter.

Status: pre-prototype. Read `docs/VISION.md`, `docs/ARCHITECTURE.md` and `docs/ROADMAP.md` before making changes.

## Principles

- **Local-first.** Transcription and vision run on-device. Cloud is only ever an opt-in BYOK cleanup step.
- **Privacy.** Camera frames live in memory only. Never write frames to disk, never send them over the network, never log them. Audio is discarded after transcription unless the user opts in.
- **Cross-platform.** macOS and Windows first, Linux later. Prefer approaches that work everywhere (MediaPipe in the webview) over platform-specific shortcuts, unless it is an explicit fast path.
- **Auto-Enter is always cancelable.** Every auto-submit goes through a preview window the user can cancel. A manual hotkey fallback must always exist.
- **Fail loudly.** If the camera, mic, model or injection fails, surface it clearly. Never silently fall back to a mode the user did not choose.
- **Measure everything.** Claims about accuracy, latency, CPU or battery need numbers. Note what was verified versus assumed.

## Conventions

- Do not use em dashes in docs or code comments.
- Model files (`*.bin`, `*.gguf`, `*.onnx`) and recordings never get committed.
- **Model selection guidance:** At the end of each response proposing or guiding next steps, always specify which model to run for the next action (Gemini 3.8 Flash vs Gemini 4) and explain why in one clear sentence.
