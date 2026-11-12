# AGENTS.md

## Project context

saywhen is a Tauri v2 + Rust + TypeScript desktop app for hands-free, hotkey-free voice input aimed at developers using coding agents and terminals. It gates listening on webcam signals (face present, facing the screen, lips moving) plus audio speech detection, transcribes locally, runs an AI cleanup pass, shows a short cancelable preview, injects the text into the focused app and presses Enter.

Status: Stages 1 through 5 and Improvements #2 to #16 are implemented (see `docs/ROADMAP.md`). Read `README.md`, `docs/VISION.md`, `docs/ARCHITECTURE.md` and `docs/ROADMAP.md` before making changes.

If the person asked you to install, run, or try saywhen, do that before any code change. Follow **For coding assistants** in `README.md`, then `INSTALL_WITH_AI.md` from the top. The repository is already open, so do not clone it again. On macOS, do not launch `npx tauri dev` from your own background shell. Ask them to run it in the Terminal app so the camera and microphone prompts appear for them.

## Architecture & Key Files

- `src-tauri/src/audio.rs`: 16 kHz `cpal` capture worker thread, 500 ms (`8,000`-sample) circular pre-roll ring buffer, Silero VAD v5 ONNX (`voice_activity_detector`), 5-state multimodal machine (`IDLE -> ARMED -> LISTENING -> TRANSCRIBING -> PREVIEW`), head gesture handling (`nod` / `shake`), and `settings.local.json` persistence.
- `src-tauri/src/turn_taking.rs`: Pure turn-taking decision logic (`evaluate_arming_hysteresis`, `should_auto_start`, `has_minimum_speech_evidence`, `compute_adaptive_silence_timeout_ms`, `end_of_turn`, `silence_countdown_ms`), send modes (`SendMode`: `pause` / `nod` / `phrase` / `paste_only`, `preview_deadline`, `should_cancel_preview_on_speech`), privacy-safe `session-metrics.jsonl` logger, and opt-in `recordings/` 16 kHz WAV + JSON sidecar writer.
- `src-tauri/src/bin/replay.rs`: Offline replay and 56-utterance benchmark binary (`cargo run --bin replay -- [recordings_dir]`, `--generate-corpus`) comparing legacy vs Improved turn-taking on identical WAV + vision traces.
- `src-tauri/src/wake_word.rs`: Opt-in wake word fallback (`strip_wake_word`, Whisper spelling aliases for the default `eevee`, filler words) used when the camera does not see the user facing the screen.
- `src-tauri/src/transcribe.rs`: `whisper-rs` (`0.16.0`) Metal GPU engine (`use_gpu(true)`), model downloader, hallucination filter, and `"send it"` / `"submit it"` end-phrase stripping.
- `src-tauri/src/cleanup.rs`: Local deterministic developer CLI/code cleanup pass (`run_local_developer_cleanup_with_vocab`), `"scratch that"` / `"paste only"` voice overrides, custom vocabulary (`apply_custom_vocab_terms`), and destructive command guard (`check_destructive_command`).
- `src-tauri/src/history.rs`: Local 7-day auto-pruned utterance history (`<app_data_dir>/utterance-history.jsonl`, `HISTORY_RETENTION_MS = 7 days`, `MAX_HISTORY_ENTRIES = 200`) and one-click purge.
- `src-tauri/src/inject.rs` (+ `src-tauri/src/inject/policy.rs`, `src-tauri/src/inject/macos.rs`, `src-tauri/src/inject/windows.rs`, `src-tauri/src/inject/linux.rs`, `src-tauri/src/inject/tests.rs`): Main-thread `enigo` + `arboard` clipboard injection (`run_on_main_thread`), native frontmost app detection (`NSWorkspace.frontmostApplication` on macOS, Win32 `GetForegroundWindow` + `QueryFullProcessImageNameW` on Windows), bundle-id/exe allowlist matching, self-injection block, target-change verification before paste, length-scaled Enter delay, conditional text/image clipboard restore, modifier release, macOS Secure Event Input detection, and Windows UIPI elevation guard.
- `src-tauri/src/hud.rs`: Floating HUD pill window placement and click-through, preview-only global `Esc` cancel, tray menu, close-to-background.
- `src/hud.ts` + `hud.html`: Transparent, non-focusable HUD pill and preview card (event listener only).
- `src/main.ts`: 15 FPS MediaPipe Face Landmarker vision gate, head gesture detector, live VAD/RMS oscilloscope, and cancelable Preview HUD.
- `src/style.css` + `src/fonts.ts` + `src/assets/logo.png`: Main window theme built on the logo (plum / silver / amber, rounded white cards, registered `@property` palette colors, a per-state "mood" glow), the in-app Guide page styles, and bundled `@fontsource` fonts so the UI renders offline. App icons in `src-tauri/icons/` are generated from `icon-source.png` with `npx tauri icon`.
- `INSTALL_WITH_AI.md`, `docs/HOW_IT_WORKS.md`, `docs/HOW_WE_BUILT_IT.md`: Plain-language install guide for coding assistants and non-technical explainers. Keep user-facing copy at that reading level.
- `scripts/capture-screenshots.sh`: Headless Chrome screenshot generator for `docs/screenshots/stage1..stage5.png`.

## Commands

- Build frontend: `npm run build`
- Run Rust unit & Metal benchmark tests: `export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml`
- Run turn-taking offline replay benchmark: `export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH" && cargo run --manifest-path src-tauri/Cargo.toml --bin replay -- recordings`
- Run desktop app in dev mode: `export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH" && npx tauri dev`
- Regenerate documentation screenshots: `./scripts/capture-screenshots.sh`

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
