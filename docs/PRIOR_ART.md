# Prior art

Status: the Stage 0 search has NOT been done. This page lists what is known so far, and what still needs to be checked. Nothing here supports a claim that saywhen is novel.

## Known tools in the space

These are mostly offline or local dictation apps. Details below are from memory and need verifying in Stage 0.

| Tool | Notes (to verify) |
| --- | --- |
| Handy (cjpais/Handy) | Tauri v2 + Rust, Whisper and Parakeet, Silero VAD, MIT, around 35k stars. Strong proof of demand for free offline dictation. Hotkey driven. Main code reference for Stage 2. |
| Pindrop | Local macOS dictation app (WhisperKit). Discontinued and replaced by Sayso (justsayso.app). |
| Yap | Local dictation app. |
| VoiceInk | Local dictation app for macOS. |
| Whispur | Local dictation app. |
| TypeNo | Local dictation app. |
| miniwhisper | Small Whisper-based dictation tool. |
| FluidVoice | Local dictation app. |
| OpenWhispr | Open source Whisper dictation app. |
| Voicetypr | Local dictation app. |
| Earshot (Leonxlnx/earshot) | "Ears for coding agents", around 3.4k stars. Close to our positioning, check in Stage 0. |

For each, record: platforms, trigger method (hotkey, wake word, always-on), whether it auto-submits, cleanup approach, license, activity.

## Stage 0 search checklist

- [ ] **Visual VAD**: research and code on visual voice activity detection (detecting speech from video of the face).
- [ ] **Webcam lip-triggered dictation**: any app or project that starts dictation when lips move on camera.
- [ ] **Look-to-talk**: gaze or attention-based activation (for example, assistants that activate when you look at the device).
- [ ] **MediaPipe mouth-open speech detection**: demos, blog posts or repos using MediaPipe face landmarks to detect speaking.
- [ ] **Talon Voice and OS voice control overlap**: what Talon, macOS Voice Control and Windows Voice Access already do for hands-free input, and whether any use the camera (for example eye tracking or face gestures).
- [ ] Patents on camera-gated speech recognition or lip-movement-triggered input.
- [ ] Each tool in the table above checked for any camera or no-hotkey mode.

Output: a short summary in this file with links, what each does, and what gap remains.
