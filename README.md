# saywhen

> Hands-free, hotkey-free voice input for developers.

saywhen is a cross-platform desktop app (Tauri v2 + Rust + TypeScript) that lets you talk to your coding agent or terminal without touching the keyboard. It watches your webcam locally, notices when you are looking at the screen, your lips are moving, and the mic hears speech, then it listens, transcribes on-device, cleans up the text, shows a short cancelable preview, types it into the focused app and presses Enter for you.

No hotkey. No "review, then click Send". Just look, talk, done.

**Positioning:** hands-free voice for coding agents and terminals (Claude Code, Codex, shells). Not "another dictation app".

## Status

**Pre-prototype.** Nothing runs yet. This repo currently holds the vision, architecture, roadmap and risk notes. The first real milestone is a feasibility spike that measures whether webcam lip signals are reliable enough to gate voice input (see [docs/ROADMAP.md](docs/ROADMAP.md)).

Prior art research has not been done yet, so no claim is made that this idea is novel.

## Goals

- **No hotkey.** Intent is inferred from face, gaze direction, lip movement and speech together.
- **Any laptop with a webcam.** macOS and Windows first, Linux later. No special hardware.
- **100% local.** Camera frames stay in memory and are never saved or sent anywhere. Transcription runs on-device. AI cleanup can be local or bring-your-own-key.
- **Safe auto-submit.** A short, cancelable preview before Enter is pressed, with per-app rules.
- **Good AI cleanup.** Remove filler, fix punctuation, keep code identifiers and commands intact.
- **Learning project.** A real excuse to learn Tauri, Rust audio, on-device ML and computer vision, with everything measured.

## Non-goals

- Silent lip-reading (visual speech recognition without audio).
- Cloud hosting or a hosted transcription service.
- Mobile apps.

## Repo map

| Path | What it is |
| --- | --- |
| [docs/VISION.md](docs/VISION.md) | Problem, trigger and stop logic, user flow, the demo |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Stack, pipeline, state machine, privacy design, open questions |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Stages 0 to 6 with exit criteria |
| [docs/RISKS.md](docs/RISKS.md) | Risks and mitigations |
| [docs/PRIOR_ART.md](docs/PRIOR_ART.md) | Known tools in the space and the Stage 0 search checklist |
| [docs/IDEA_LOG.md](docs/IDEA_LOG.md) | How this idea was picked, and parked ideas |
| [docs/GEMINI_BRIEF.md](docs/GEMINI_BRIEF.md) | Paste-in brief for a research and spike assistant |
| [CLAUDE.md](CLAUDE.md) | Project context and principles for AI coding assistants |

## License

MIT. See [LICENSE](LICENSE).
