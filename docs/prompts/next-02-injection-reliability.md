# Prompt: Improvement #2, reliable text injection

Copy everything below the line into Gemini 4, run from the root of the saywhen repo.

---

You are working on **saywhen**, a Tauri v2 + Rust + TypeScript desktop app for hands-free voice input for developers. It listens when the webcam sees you facing the screen and talking, transcribes locally with Whisper, runs a cleanup pass, shows a short cancelable preview, then pastes the text into the focused app and optionally presses Enter.

## Before you start

1. Read `CLAUDE.md`, `README.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` and `docs/RISKS.md`. Follow every rule in `CLAUDE.md` (no em dashes in docs or code comments, fail loudly, measure everything, auto-Enter is always cancelable, cross-platform macOS + Windows).
2. Work on a new branch `feat/injection-reliability` created from the latest `main` (or from `main-bjms27` if PR "floating HUD pill" is not merged yet).
3. Confirm the baseline builds before changing anything:
   - `npm run build`
   - `export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml`

## Goal

Make text injection **correct, safe and observable**: the text always lands in the app the user saw in the preview, Enter is only ever pressed in an allowlisted app, slow apps do not lose text or get Enter early, the user's clipboard is not clobbered, and every failure is shown clearly in the HUD pill and main window instead of failing silently.

## Files involved

- `src-tauri/src/inject.rs`: `detect_frontmost_app`, `default_auto_send_allowlist`, `evaluate_app_injection_policy`, `inject_text_into_focused_app`, `check_accessibility_permission`.
- `src-tauri/src/audio.rs`: `finalize_preview_and_inject` (calls injection after the preview countdown), `BeginPreview` (where the target policy is computed), `DictationConfig`.
- `src-tauri/src/lib.rs`: `manual_inject_text` command.
- `src/main.ts` and `src/hud.ts`: show injection results and errors (`utterance-complete`, `pipeline-error` events).

## Problems found in the audit (fix all of these)

1. **Target can change during the preview countdown (safety bug).** The target app and its auto-Enter policy are computed when the preview starts, but injection happens 1 to 2 seconds later into whatever is frontmost then. If the user switches from Terminal to Slack during the countdown, the text is pasted into Slack and **Enter is pressed there** using Terminal's policy.
   - Fix: re-detect the frontmost app immediately before injecting. If it differs from the preview target, do not press Enter: either paste-only into the new app or abort and keep the text on the clipboard (pick one, make it a config option `onTargetChanged: "abort" | "paste_only"`, default `"abort"`), and emit a clear error/notice.

2. **Allowlist matching is a loose substring match.** `app_lower.contains(allowed)` means `"Code"` matches `"Xcode"`, `"Claude"` matches anything containing that word, and so on. `"saywhen"` is also on the allowlist, so saywhen can paste into and press Enter on itself.
   - Fix: match on exact app names (case-insensitive) and, where available, on stable identifiers: macOS bundle id (for example `com.apple.Terminal`, `com.googlecode.iterm2`, `dev.warp.Warp-Stable`, `com.mitchellh.ghostty`, `com.microsoft.VSCode`, `com.todesktop.230313mzl4w4u92` for Cursor, `dev.zed.Zed`) and Windows process executable name (for example `WindowsTerminal.exe`, `Code.exe`, `Cursor.exe`). Remove `saywhen` from the default allowlist and never inject into saywhen's own windows (refuse with a clear message).
   - Keep old `settings.local.json` files loading (use `#[serde(default)]` for any new fields; there is already a test pattern for this at the bottom of `audio.rs`).

3. **Frontmost app detection is fragile and slow.**
   - macOS: parses `lsappinfo` text output with hand-rolled quote scanning. Replace with `NSWorkspace.sharedWorkspace.frontmostApplication` (via the `objc2` / `objc2-app-kit` crates) returning name, bundle id and pid.
   - Windows: spawns PowerShell on every call (hundreds of ms, may flash a console window). Replace with `GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageNameW` through the `windows` or `windows-sys` crate.
   - Return a struct like `FrontmostApp { name, bundle_id: Option<String>, exe: Option<String>, pid: Option<u32> }` and use it in the policy. Measure and log the detection time before and after (numbers in the PR description).

4. **Fixed timing breaks slow apps.** Enter is pressed a fixed 45 ms after Cmd/Ctrl+V. Electron apps (VS Code, Cursor, Claude desktop) and large pastes can process the paste later, so Enter arrives first, or on an empty prompt.
   - Fix: make the paste-to-Enter delay configurable (`enterDelayMs`, default around 120 ms) and scale it with text length (for example plus 1 ms per 20 characters, capped at 400 ms). Add a per-app override map if simple.

5. **Clipboard restore races and loses data.** The previous clipboard is restored after a fixed 160 ms. Slow apps may read the clipboard after that and paste the old contents. Only plain text is saved, so an image or rich content on the clipboard is destroyed. If the user copies something during that window it gets overwritten.
   - Fix: restore after a longer, configurable delay (default around 600 ms, after Enter). Before restoring, check the clipboard still contains exactly our transcript; if not, the user changed it, so do not restore. If the previous clipboard was not plain text, say so in the outcome message (and preserve it if `arboard` supports the format, for example images).

6. **Held modifier keys corrupt the paste.** If a modifier (Option/Alt, Shift, Ctrl) is still physically held when we send Cmd/Ctrl+V or Return, the app receives a different shortcut. Release or wait for modifiers to be released before injecting (enigo can release Alt, Shift, Control, Meta explicitly).

7. **Silent failures that must become loud.**
   - macOS Secure Event Input (password fields, Terminal "Secure Keyboard Entry", some password managers) silently blocks synthetic keystrokes. Detect it with `IsSecureEventInputEnabled()` (Carbon) and fail with a clear message, leaving the text on the clipboard.
   - Windows UIPI: an elevated (Run as administrator) terminal ignores input from a non-elevated saywhen. Detect when the target process is elevated and saywhen is not, and fail with a clear message.
   - `check_accessibility_permission` returns `true` on non-macOS unconditionally; keep that but make the Windows elevation check the equivalent guard.
   - Every failure must reach the HUD pill (`pipeline-error` and `utterance-complete.injectionError`) and must leave the transcript on the clipboard so nothing the user said is lost.

8. **Main-thread timeout.** Injection waits up to 3 s for the main thread. If it times out the keystrokes may still fire later. Track this case and report it honestly ("injection may have happened late").

## Constraints

- Do not change the preview / cancel flow or remove any cancel path (Esc, head shake, look-away, tray Cancel).
- Do not add network calls. Do not log transcript text in release builds beyond what already exists.
- Keep `inject.rs` focused: if it grows past about 500 lines, split platform code into `inject/macos.rs` and `inject/windows.rs`.
- No em dashes in comments or docs.

## Tests and verification

Add Rust unit tests (no real keyboard needed) for:
- Allowlist matching: `Xcode` must not match `Code`; bundle id and exe matching; saywhen itself is refused.
- Policy when the target changed between preview and injection (abort vs paste-only).
- Enter delay scaling with text length.
- Clipboard restore decision (restore only if the clipboard still equals our transcript).
- Legacy `settings.local.json` without the new fields still loads.

Then write a manual test checklist in `docs/ROADMAP.md` under a new section "Improvement #2: Injection reliability", and run what you can on macOS:
1. Dictate into Terminal, iTerm2 or Ghostty, VS Code or Cursor integrated terminal, and the Claude desktop app. Text lands, Enter fires after the text, not before.
2. Start a dictation in Terminal, switch to Notes during the countdown. Expected: aborted (or paste-only) with a clear HUD message, no Enter in Notes.
3. Copy an image, dictate, then paste manually afterwards: the image is still on the clipboard (or the outcome says it could not be preserved).
4. Turn on Terminal > Secure Keyboard Entry, dictate: clear error, text on clipboard.
5. Hold Shift while the preview counts down: paste is still correct.
6. Record frontmost detection time before and after the change (median of 20 calls).

Mark each item verified or not verified, with numbers. Do not claim Windows results unless you actually ran on Windows.

## Deliverables

1. Code changes on `feat/injection-reliability`, with `npm run build` and `cargo test` passing and no new compiler warnings.
2. Updated `docs/ARCHITECTURE.md` (injection section), `docs/ROADMAP.md` (status + checklist + measurements) and `CLAUDE.md` key files list if files were added.
3. A pull request into `main` describing: what changed, why, the measured numbers, what was verified on which OS, and what is still unverified.

At the end, tell me in plain language what changed and what I should test by hand, and say which model to use for the next step (Gemini 3.8 Flash vs Gemini 4) with a one-sentence reason.
