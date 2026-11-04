# Prompt: Improvement #3, turn-taking robustness (when to start and stop listening)

Copy everything below the line into Gemini 4, run from the root of the saywhen repo.

---

You are working on **saywhen**, a Tauri v2 + Rust + TypeScript desktop app for hands-free voice input for developers. It arms when the webcam sees you facing the screen, starts listening when your lips move and Silero VAD hears speech, stops on silence plus closed mouth (or "send it" / nod), transcribes locally with Whisper, shows a cancelable preview in a floating HUD pill, then pastes into the focused app and optionally presses Enter.

## Before you start

1. Read `CLAUDE.md`, `README.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/RISKS.md`. Follow every rule in `CLAUDE.md`: no em dashes in docs or code comments, fail loudly, measure everything (say what was verified versus assumed), auto-Enter is always cancelable, cross-platform macOS + Windows, camera frames never leave memory, audio is discarded unless the user opts in.
2. Branch `feat/turn-taking` from the latest `main`.
3. Baseline must pass first: `npm run build` and `export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml`.

## Goal

Fewer false starts (coughs, talking to someone else, typing noise), fewer cut-off sentences (thinking pauses), no clipped last words, and **numbers** that prove it. Measurement comes first; tuning without data is not allowed.

## Where the logic lives

- `src-tauri/src/audio.rs`
  - `run_audio_worker`: the VAD loop. Auto-start is `ARMED && lips_moving && (vad >= threshold || voice within 320 ms)`. Auto-stop is `speech_frames_in_utterance >= 3 && mouth_quiet && silence >= silence_timeout_ms (650)`.
  - `UpdateVision`: `IDLE <-> ARMED` flips on every frame from `face_present && facing_screen` with no hysteresis.
  - `SharedCaptureBuffer::begin_listening`: prepends the 500 ms pre-roll. There is no post-roll.
  - `StopAndTranscribe`: transcription thread; emits `utterance-discarded` when the transcript is empty.
- `src/main.ts`: `processVisionFrame` computes `facingScreen` (`|yaw| <= 22`, `|pitch| <= 20`), `lipsMoving` (MAR variance, 300 ms hold), `mouthClosed`, head gestures.
- `src-tauri/src/transcribe.rs`: `is_likely_silence_hallucination` with `HALLUCINATION_TRUST_VAD = 0.85` (assumed, not measured).
- `src/hud.ts`: HUD pill; shows `utterance-discarded`, cancels and errors.

## Tasks

### 1. Measurement harness (do this first)

- Add an opt-in **session metrics log** (off by default, toggle in settings): one JSON line per utterance written to the app data dir (`app.path().app_data_dir()`), never to the repo. Fields: trigger source, time from first lip motion to LISTENING, utterance length, speech frames, peak VAD, end reason (silence, end phrase, nod, hotkey, max length), transcript length, discarded reason, outcome (sent, pasted, canceled, aborted), user "Flag False Trigger" clicks. **No transcript text and no audio** in this log.
- Add a separate, explicitly labelled opt-in **"save utterance audio for tuning"** switch that writes 16 kHz WAV files to a local `recordings/` folder (already gitignored) with a matching JSON sidecar of the metrics. Show a visible indicator in the HUD while it is on. Off by default, and it turns itself off when the app restarts.
- Add an offline replay test or binary (`cargo test` ignored-by-default test, or `src-tauri/src/bin/replay.rs`) that feeds saved WAVs plus recorded vision telemetry through the same start/stop decision code, so tuning changes can be compared on identical input. To make this possible, extract the start/stop decisions from `run_audio_worker` into pure functions (for example `fn should_auto_start(&GateInputs) -> bool`, `fn end_of_turn(&TurnState, now) -> Option<EndReason>`) and unit test them.

### 2. Arming hysteresis

`facing_screen` flickers near the yaw/pitch limits, so `ARMED` and `IDLE` thrash. Add separate enter and exit thresholds (for example arm at `|yaw| <= 20`, disarm only after `|yaw| > 28` for 300 ms) and a short arm delay so glancing past the screen does not arm. Make the thresholds config fields with `#[serde(default)]`.

### 3. Minimum speech evidence

Coughs and single clicks currently reach Whisper (3 VAD frames, about 96 ms, is enough). Require a minimum amount of speech (start with 250 ms of frames above threshold, tune with data) before transcribing; otherwise discard and emit `utterance-discarded` with a clear reason. Hotkey-started utterances keep a lower bar.

### 4. Adaptive end of turn

650 ms of silence cuts developers off mid-thought. Make end-of-turn adaptive:
- Keep waiting while lips are still moving (vision fresh) even if VAD dips.
- Extend the timeout when the partial utterance is long, or (if cheap) when a fast partial transcript ends in a connective like "and", "the", "to", or a comma.
- Keep "send it" / nod / hotkey as instant ends.
- Show the remaining end-of-turn countdown subtly in the HUD so the user learns the rhythm.

### 5. Post-roll

Keep capturing 150 to 250 ms after the end-of-turn decision before handing audio to Whisper, so the last word is not clipped. Measure clipped-word rate before and after with the replay set.

### 6. Tune the hallucination filter

Use the recorded set to choose `HALLUCINATION_TRUST_VAD` from data, and extend `SILENCE_HALLUCINATION_PHRASES` only with phrases that actually appear on non-speech audio. Never drop short real replies such as "yes", "no", "okay", "continue".

## Constraints

- Do not remove or weaken any cancel path (Esc during preview, head shake, look-away, tray Cancel, spoken "cancel").
- Do not change injection code in `src-tauri/src/inject*`.
- New config fields must be backward compatible with old `settings.local.json` (extend the test at the bottom of `audio.rs`).
- Never write camera frames anywhere. Audio is written only when the explicit opt-in switch is on.

## Verification and numbers

Record at least 50 utterances on macOS covering normal commands, thinking pauses, coughs, keyboard typing, talking to someone off-screen, and glancing away. Report before and after for: false start rate, cut-off rate (ended before the user finished), clipped last word rate, median time from last word to preview, and HUD arm/disarm flicker count per minute. Put the table in `docs/ROADMAP.md` under "Improvement #3: Turn-taking", mark each number verified or assumed, and do not report Windows numbers unless measured on Windows.

## Deliverables

1. Code on `feat/turn-taking` with `npm run build` and `cargo test` passing and no new warnings.
2. Docs updated: `docs/ARCHITECTURE.md` (state machine and gate), `docs/ROADMAP.md` (status, numbers), `CLAUDE.md` key files if files were added.
3. A pull request into `main` with what changed, why, the before/after table, and what is still unverified.

At the end, explain in plain language what changed and what I should test by hand, and say which model to use for the next step (Gemini 3.8 Flash vs Gemini 4) with a one-sentence reason.
