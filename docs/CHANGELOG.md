# Changelog

## Unreleased

- The onboarding window keeps **Start talking** visible at the default window size, traps focus, closes with Escape and restores focus when it closes.
- The onboarding microphone and camera badges now reflect the device list and the camera state instead of being hard-coded.
- Model downloads show a real progress bar and percentage in Settings and on the setup checklist.
- The "Say something" setup step is only marked done after text is successfully injected.
- The check step scrolls into view when it appears, so it is visible at the minimum window size.
- The check step can no longer be turned off. Every automatic send goes through a cancelable review.
- Send-style cards support arrow-key navigation.
- Renamed Lab to Diagnostics and moved the practice box and live numbers under that name.
- Removed the marketing demo button from Home and hid the Problems panel until there is a real problem.
- The HUD now appears on the display under the pointer, and the tray has a "Move HUD to This Display" action. It stays click-through on purpose so it can never steal focus; send and cancel remain on the keyboard and in the main window.
- Added a searchable static docs hub in `docs-site/` (build with `npm run docs:build`).
- Added a command palette on Cmd+K / Ctrl+K for navigation, listening, camera, theme, contrast, text size and help actions.
- Added a skippable first-run intro, with a Replay the intro command.
- Added high contrast mode, an in-app reduce-motion switch and four text-size steps that scale the window through the webview.
- The dark theme now uses brighter text, stronger surfaces and contrast-checked labels; sent, cancel and error actions have short motion cues.
- The push-to-talk shortcut can be recorded and reset in Settings > Sending. The HUD and the in-app Guide follow the new binding. If the combination is already taken, the old shortcut is restored and the error is shown.
- Onboarding now has a Try it now practice box: record a short command and see the raw and cleaned text without typing into another app or needing Accessibility.
- Recent has a Paste last button, and Cmd+Ctrl+V on Mac or Ctrl+Alt+V on Windows types the last transcript again without pressing Enter.
- Settings now has a section navigation bar, a search field, an app picker for the trusted-apps list, and an About section with a privacy-safe diagnostics copy.
- Added a System, Light or Dark theme. The choice is remembered and follows the system when set to System.
- Moved the custom vocabulary into its own Words section and renamed Colours to Appearance.
- Added a user [FAQ](FAQ.md) and expanded [troubleshooting](TROUBLESHOOTING.md) for permissions, camera stalls, microphone loss, secure input, elevation and model downloads.
- Fixed the broken `#see-it-work` link in the documentation index.

## v0.2.0 beta

- First public beta for Apple Silicon macOS and 64-bit Windows.
- Local Whisper transcription, camera-gated hands-free listening, pre-roll, preview, send styles, 7-day history and the in-app Guide.
- Developer cleanup for CLI commands, custom vocabulary, "scratch that", "paste only", and a destructive-command guard.
