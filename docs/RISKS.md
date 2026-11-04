# Risks

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Unreliable lip signal | Glasses, beards, masks, bad lighting, side angles or low-res webcams may make MAR noisy. The whole idea depends on it. | Stage 1 spike with real-desk tests and a hard 90% separation exit. Combine multiple features. Per-user calibration. Kill or pivot the idea if it fails. |
| Auto-Enter sends the wrong thing | A misheard shell command could do real damage. | Cancelable preview with countdown. Per-app rules (paste-only by default for risky apps). Never auto-send in apps not on the allow list. Optional confirm for destructive-looking commands. |
| Always-on camera trust | Users are rightly wary of an app that watches them. | Frames in memory only, never saved or sent. Visible indicator, one-click pause. Open source code. Clear privacy doc. |
| Webview throttling | A hidden webview may throttle camera, timers or WebGL, killing the visual gate. | Test in Stage 1. Mitigated (unverified on hardware): Worker-timer vision loop, `backgroundThrottling: disabled` on macOS 14+, WebView2 anti-throttling args and Web Lock on Windows. The HUD pill flags a stalled gate. Remaining fallbacks: move vision into the always-visible HUD window, or native vision fast path. |
| CPU and battery | Continuous vision at 10 to 15 FPS could drain laptops. | Measure early. Adaptive frame rate (low FPS in IDLE, higher in ARMED). Pause on battery saver or when the screen locks. |
| Linux / Wayland | Wayland restricts global input injection and window focus info. | Defer Linux to Stage 6. X11 first. Investigate portals and ydotool-style approaches later. |
| Crowded market | Many free offline dictation apps already exist. | Position narrowly: hands-free voice for coding agents and terminals. Compete on no-hotkey intent and safe auto-submit, not on transcription. |
| Scope creep | Gestures, gaze targeting, cleanup tuning and multi-platform could swallow the project. | Strict staged roadmap with exit criteria. Park ideas in IDEA_LOG.md. |
| Accessibility and fairness | Lip-movement detection may work worse for some faces, speech patterns, disabilities or cultural differences. | A manual hotkey fallback is always available. Test with varied people before going public. Make thresholds tunable and calibration easy. |
