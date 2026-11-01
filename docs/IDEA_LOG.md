# Idea log

## How this idea was chosen

1. Asked Gemini for 50 project ideas. The list was generic and not useful.
2. Asked Gemini again for a list tailored to my existing repos. That list was much better.
3. Looked closely at Handy (cjpais/Handy). It proved there is real demand for free, offline dictation, but that space is now crowded with clones (Pindrop, VoiceInk, Yap, Whispur, TypeNo and others).
4. Noticed the open slot: no-hotkey multimodal intent sensing (face + gaze + lips + speech) combined with safe auto-submit, aimed at coding agents and terminals.
5. Decided on this idea.
6. Stack decision: Tauri + MediaPipe, so the same vision code runs cross-platform in the webview.

## Parked ideas

Not dead, just not now.

- **QuickLook extension** for parquet, duckdb and sqlite files.
- **Chrome MV3 animation inspector.**
- **csv-autopsy**
- **webhook-bench**
- **blender-checkpoint**
- **calibrate-llm**
- **SceneBoardKit**

## Naming

- The first working name was dropped because it suggests lip-reading (a non-goal), makes the camera feel more invasive, and ties the brand to one signal.
- Considered and rejected: Sayso (taken by the Pindrop rebrand, a direct competitor), Earshot (existing "ears for coding agents" repo), nokey (reads as "no API key" or a password tool, and a manual hotkey fallback exists anyway), Heed, Utter, Hark (taken everywhere).
- Picked **saywhen** (2026-11-01): "say when" is the everyday phrase for "tell me when you are done", which is the auto-stop and auto-send idea. Backup was saydo.
