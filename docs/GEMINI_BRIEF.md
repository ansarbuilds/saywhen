# Brief for Antigravity / Gemini

Paste everything below this line into the assistant.

---

You are helping with **saywhen**, a cross-platform desktop app built with Tauri v2 + Rust + TypeScript. It provides hands-free, hotkey-free voice input for developers. It watches the webcam locally (frames in memory only, never saved or sent), detects when the user is facing the screen AND their lips are moving AND the mic hears speech, then records, transcribes locally (Whisper or Parakeet), cleans the text with an AI pass, shows a short cancelable preview, injects the text into the focused app and presses Enter. Target users: developers talking to coding agents (Claude Code, Codex) and terminals.

## Rules

- Be honest about what you verified versus what you assume. Label every claim as VERIFIED (with a link or measurement) or ASSUMED.
- Give numbers wherever possible: percentages, milliseconds, CPU %, FPS, star counts, dates.
- Do not claim the idea is novel unless the search supports it, and even then say what you searched.
- If something fails or is unclear, say so plainly.

## Task A: Prior-art search

Find existing tools, research and projects that overlap with saywhen. Cover at least:

- Visual voice activity detection (speech detection from face video).
- Webcam lip-triggered dictation apps or scripts.
- Look-to-talk or gaze-activated assistants.
- MediaPipe mouth-open or mouth-movement speech detection demos.
- Talon Voice, macOS Voice Control, Windows Voice Access: what overlaps.
- Offline dictation apps: Handy, Pindrop, Yap, VoiceInk, Whispur, TypeNo, miniwhisper, FluidVoice, OpenWhispr, Voicetypr. For each: platforms, trigger method, auto-submit yes/no, license, last activity.

Deliver: a table plus a one-paragraph verdict on what gap, if any, is left.

## Task B: Stage 1 MediaPipe spike

Build a minimal web page (plain TypeScript, no framework) that:

1. Runs MediaPipe Tasks Vision Face Landmarker on the webcam at 10 to 15 FPS.
2. Computes and plots live: mouth aspect ratio (MAR), MAR variance over a ~300ms window, head yaw and pitch, and the jawOpen / mouthClose blendshapes.
3. Lets me press keys to label segments: talking, silent, yawning, chewing, smiling, turned away.
4. Exports labeled feature data (numbers only, no frames) as CSV.
5. Shows current CPU-relevant stats (actual FPS, inference ms per frame).

Then report measurements:

- Separation between talking and each non-talking class for each feature (accuracy at a chosen threshold, or AUC).
- Inference time per frame and approximate CPU usage.
- Whether the page keeps running at the same FPS when the tab or window is hidden.

Exit target: at least 90% separation at under about 10% CPU.

## Task C: Study Handy's code

Read cjpais/Handy and summarize:

- Project structure and how the Tauri frontend and Rust backend split work.
- Audio capture (cpal), buffering and Silero VAD integration.
- How whisper.cpp and Parakeet are loaded and run, and how models are downloaded.
- How text is injected into other apps on each platform, and known problems.
- Anything reusable for saywhen (it is MIT licensed) and anything to avoid.

Deliver: a short report with file paths and line references.
