# saywhen: UI/UX, portfolio and docs review

Date: 2026-11-12
Scope: the desktop app, the live project page at ansarbuilds.dev, the public docs set, and the Wispr Flow comparison.
Method: read the repo, ran the app's browser demo at 1440x940, 1280x800, 1120x780 and 900x640, captured the live project page and portfolio at desktop and 390px mobile widths, and reviewed Wispr Flow's public site, help center and onboarding notes.

Status: the repo-side implementation is complete in this workspace. `docs/CHANGELOG.md` lists the slices: onboarding fit, focus, model download, practice box and live badges; setup and preview truth; a settings search, section navigation, app picker, dark mode and shortcut editor; a Home cleanup; a Diagnostics rename; HUD placement and Paste last; FAQ, expanded troubleshooting, a changelog, a docs link checker and a generated searchable docs hub in `docs-site/`. The remaining work is external or manual: Apple signing and notarization, Windows and Intel hardware checks, real-user evidence, a packaged-app microphone run, and wiring the docs hub into the portfolio site after approval.

## Verdict

The visual craft is already in the top decile of solo-built desktop apps. This does not look like a side project. The logo, the plum and amber palette, the orb, the HUD and the plain-language voice all read as one product.

The gap is not taste. It is focus, first-run confidence, and the unglamorous plumbing around install, permissions, settings and help. Wispr Flow is not winning because its screens are prettier. It is winning because a new person gets from download to a working dictation in a few confident minutes, and because every failure has a documented next step.

The single most important UX issue is not in the UI at all. The beta is unsigned, several launch gates are still pending, and the project page asks people to run `xattr -cr` or click through SmartScreen. That costs more installs than any color or card change. Signing, notarization, an updater and permission recovery belong at the top of the roadmap.

The strategic recommendation is to stop trying to out-Wispr Wispr Flow. Wispr wins on cloud polish, cross-device sync, 100+ languages, shortcuts, dictionary, snippets, onboarding and a mature help center. saywhen wins on hands-free intent, local-first privacy, developer-specific cleanup and a cancelable preview. Compete on that wedge, and match Wispr only on the operational polish that makes the wedge credible.

### My read, not a benchmark

| Area | Today | What is missing |
| --- | --- | --- |
| Brand and visual craft | 9/10 | Consistency, dark mode, fewer competing themes |
| First run and activation | 5/10 | Real checks, a guided first win, model download UX |
| Home focus | 6/10 | One primary job, less engineering chrome |
| Preview and HUD | 7/10 | One authoritative preview, timing, drag and follow-display |
| Settings | 5/10 | Information architecture, pickers, shortcuts, about and updates |
| Docs prose | 8/10 | Thin at the failure points |
| Docs delivery | 3/10 | No searchable docs hub, no changelog, no in-app deep links |
| Project page, desktop | 9/10 | Conversion plumbing, proof, docs navigation |
| Project page, mobile | 5/10 | Horizontal overflow and dense featured cards |
| Install and trust | 4/10 | Signing, notarization, updater, Windows and Intel evidence |

## What already works, and should not be lost

- The brand is distinct. The plum, silver and amber palette, the serif and grotesque pairing, and the "S" mark feel owned, not templated.
- The preview is the product. A short, cancelable check before Enter is the clearest difference from every dictation app that pastes instantly. It is also the trust feature that makes auto-Enter acceptable.
- The HUD is good. The bottom-center pill, the state words and the inline countdown are legible at a glance. It is much closer to Wispr Flow's flow bar than most local tools.
- The in-app Guide is genuinely good. It uses plain language, explains the tradeoffs and answers "what do I do when it goes wrong."
- The measurements page is honest. "34 of 34, on replay clips, not live users" is exactly the right tone. Honesty is a brand asset here.
- The principles are clear: local-first, privacy, cancelable auto-Enter, fail loudly, measure everything.
- Focus states and reduced-motion support exist. Many desktop apps skip both.

## App: the changes with the highest leverage

### 1. First run is a checklist, not a guided first win

This is the biggest gap against Wispr Flow.

Today, the app opens on Home with a "Get set up" card. The welcome modal is only reachable from "Setup guide" or the demo state. The checklist is a list of five tasks: download a model, allow Accessibility, turn on the camera, calibrate, say something. It is accurate, but it does not teach, and it asks for the highest-trust permission before the user has seen the product work.

Concrete problems:

- The welcome modal's microphone and camera badges are hard-coded to "Ready" and "Private". Only the Accessibility badge is live. A user can see "Ready" on a machine where the microphone is denied.
- There is no microphone test and no camera test. The first real failure happens after the user has already committed.
- The model download is a 75 MB or larger wait with a button that only changes to "Downloading...". There is no progress, no size, no cancel, no retry and no warmup state.
- The Accessibility request comes before the user has dictated anything. That is the highest-friction ask, and it arrives before the payoff.
- At the default window size (1120x780), the onboarding modal is taller than the viewport and its "Start talking" footer sits below the fold. The card can scroll, but there is no sticky footer and no visible scroll affordance.
- The modal has `role="dialog"` but no `aria-modal`, no focus trap, no initial focus and no Escape handler.
- The "Say something" setup step is marked done when any utterance completes, before the injection result is known. A failed injection can still produce a green check.
- The setup order asks for Accessibility before the camera, but the camera and calibration are where the product becomes different. The order optimizes for engineering dependencies, not for confidence.

A better first run, in five beats:

1. Welcome and privacy promise. No permission yet. One sentence on what happens to the camera and audio, with a "How it works" link.
2. Microphone check. Show the same live bars used in the HUD. The user speaks, sees the meter move and hears a soft confirmation. Allow a device picker here. This step is skippable.
3. Model download and first dictation, inside the app. Start the download in the background, show real progress and size, and when the model is ready, invite the user to say a phrase into an in-app practice box. No Accessibility permission and no camera are needed for this. Show the cleaned text, the raw "Heard" line and the preview countdown. This is the aha moment.
4. Camera and calibration, optional. Explain the benefit first ("no hotkey"), then request the camera. Use a guided overlay with a visible countdown, live head-angle readout and "hold still" and "look at the screen" states. Calibrating while turned away should switch side-camera mode on, as the code already tries to do. End with a clear "You are ready" state.
5. Typing permission, with a real test. Ask for Accessibility only after the user has seen the app transcribe and clean a phrase. Explain exactly what it enables ("type into your terminal and editor, and press Enter only where you allow it"). Poll the permission and flip the row to granted without sending the user back and forth.
6. Choose a send style. Default to "Just pause" for speed, explain "Nod" and "Type only", and let the user change it later on Home.

Acceptance criteria: a new user reaches one successful in-app dictation in under three minutes, and one real injection into a terminal in under five. The setup card should be a progress indicator, not a second onboarding.

Wispr Flow's onboarding is instructive here: it puts the real shortcut key in the card, lights it up when pressed, runs a microphone meter check, and makes the first dictation happen inside onboarding instead of telling the user to go try it later. saywhen can copy the structure without copying the hotkey.

### 2. Home is doing four jobs at once

The Home screen contains: the setup card, the hero status deck, the five-step state stepper, four proof chips, a "Play the 7 seconds" marketing button, the inline preview card, four large send-mode cards, the camera card, the recent history card and the Problems panel. That is a landing page, a control panel, a diagnostics screen and a demo all on one surface.

At the default 1120x780 window, the preview card is visible but the mode section is already at the fold. At the minimum 900x640 window, the preview card is entirely below the fold. If the user is in the Check state and the main window is focused, the most important content is off-screen. The floating HUD saves the moment, but the main window should not depend on it.

What to keep: the orb, the state word, the one-sentence hint, the camera toggle and the calibrate action. Those are the product.

What to move or remove:

- The five-step stepper duplicates the state word and the top-right badge. Remove it from Home. The Guide can explain the states.
- The four proof chips ("No hotkey to hold", "Stays on this computer", "Every send can cancel", "Enter only in trusted apps") are marketing. Move them to onboarding or the Guide, or show one rotating line.
- "Play the 7 seconds" belongs in the Guide, not in the daily control panel.
- The Problems panel should be hidden when empty. Show a small status dot in the header instead, and open a drawer when something goes wrong.
- The camera card takes a large amount of vertical space. Keep it when the camera is off or setup is incomplete. After setup, collapse it to a compact row with the live numbers and a "Show preview" toggle.
- The send-mode cards can become a compact segmented control with a one-line explanation, expandable to the full cards. Four large cards are a first-run choice, not a daily control.
- The recent list should show two or three items with a "View all" link. Move "Flag false trigger" and "Inject in 2s" into an overflow menu. Keep "Copy" visible.

Proposed Home, top to bottom:

1. Status deck: orb, friendly state, one sentence, primary camera or pause action, calibrate. The preview appears as an overlay card in this deck when the state is Check, with the target app and the countdown.
2. A single send-style row: the active style, a one-line reason and a "Change" control.
3. Recent, two items plus "View all", with copy and paste-last actions.
4. A collapsed "Camera and signal" row, expandable for the preview and numbers.
5. Nothing else. Problems, Lab and diagnostics live behind their own door.

Add a visible "Pause listening" action. Turning the camera off is a privacy action, not the same as "I am taking a call for 20 minutes." The tray has pause and resume, but the window does not. Add a pause duration and show it in the HUD and tray.

Add "Paste last" as a real command and shortcut. Wispr Flow has `Cmd+Ctrl+V`; saywhen has the transcript in history but no fast path. This is a small feature with outsized daily value.

### 3. The preview promise is undermined by a setting

This is the most important product consistency issue I found.

The project principles say every automatic send goes through a preview the user can cancel. The hero says "Every send can cancel." But Settings has "Show the check step", and turning it off sends straight away. The Rust path confirms it: when `preview_enabled` is false and the mode is not waiting for a nod, the app calls `finalize_preview_and_inject` directly.

That means a user can disable the one feature the product is built around, and the marketing promise becomes false for them. Worse, the setting is presented as a convenience, not as a risk.

Recommendation: remove the toggle, or rename it to "Skip the review step (not recommended)" and require a confirmation that explains the loss of cancelability. If it stays, the app should still offer a short global cancel grace period and a visible HUD countdown. The cleaner answer is to remove it. The preview is the product. Do not let users turn off the product.

The same section allows the preview countdown to be as short as 800 ms. For a 20-word command, that is not enough time to read, let alone cancel. Use a length-aware minimum review time, or add a "Hold to review" behavior where releasing the countdown pauses it. Wispr Flow's whole model is instant paste; saywhen's advantage is that it does not have to be instant.

The preview currently appears twice: as an inline card in the main window and as the floating HUD card. That is useful when the main window is hidden, but confusing when both are visible. Choose one authoritative surface. If the main window is visible and focused, show the review there and keep the HUD compact. If the main window is hidden, show the full HUD preview. Do not show two countdowns that can drift.

### 4. Settings is a long document, not a settings window

Settings is one scroll with four sections: Listening, Sending, Clean-up and privacy, and Colours. At the default window size, only about three controls are visible at a time. There are more than twenty controls, including a comma-separated allowlist, a comma-separated vocabulary field, two sliders and eight palette cards.

The biggest issues:

- No search, no sub-navigation and no anchors. Finding "wake word" means scrolling and reading. A settings window of this size needs a sidebar and a search field.
- The app allowlist is a raw comma-separated text input. This is a safety-critical setting. Users should see a list of apps, add the frontmost app with one click, see bundle IDs and exact-match behavior, and edit chips. Keep the text field as an advanced fallback.
- Custom vocabulary is also a comma-separated string. Make it a table or chip list with add, remove, import and export. Consider learning from manual corrections, like Wispr Flow's dictionary.
- The model selector does not show size, speed, accuracy or download state. "Bigger models are more accurate but a little slower" is not enough. Show "Fast, 75 MB" and "More accurate, 142 MB", a recommended badge and real download progress.
- The "Noise filter" label is vague and the direction is easy to get wrong. Call it "Voice detection" and show "More sensitive" and "Less sensitive", or use a simple Low, Medium, High control.
- The countdown slider shows raw milliseconds, not a human duration. Show "1.8 s" and a short explanation of what changes.
- The "AI clean-up" section has endpoint and model text fields but no "Test connection", no local vs remote badge and no clear statement that a remote endpoint sends text off the machine. The backend already classifies local loopback vs remote; surface it.
- There is no shortcut editor. Option+Space is fixed in Rust, and users cannot resolve a conflict, choose a different chord or see the current binding. This is table stakes for a dictation app.
- There is no dark mode. The app is light-only. Developers will notice immediately. Add a dark theme and a "match system" option.
- Eight palettes are a portfolio feature, not a product feature. They fragment the brand and multiply visual QA. Keep the logo palette, add a dark and a high-contrast variant, and make the rest an Easter egg or remove them.
- There is no About section, version number, update check, launch-at-login toggle, reset-all-settings action or export of settings and history.
- The "Changes save by themselves" note is at the very bottom. Add a quiet "Saved" indicator near the changed control.
- The base font is 13px and many labels are 11 to 12px. That reads small on a 1120px window. Move the base to 14px and let secondary text stay at 12.5px only when contrast is sufficient.

Proposed Settings architecture:

- Listening: microphone with a test meter, model with sizes and download, voice detection, camera, wake word, battery saver, pause behavior.
- Sending: send style, review time, target app rules, app picker, destructive command guard, Enter behavior, clipboard behavior.
- Words: custom vocabulary, pronunciation aliases, snippets, import and export.
- Shortcuts: push-to-talk and hands-free bindings, paste last, cancel, command palette, conflict detection, reset.
- Appearance: light, dark, system, high contrast, motion, sound.
- Privacy: history retention, clear data, export history, opt-in metrics, save recordings, cloud cleanup with local vs remote status.
- Advanced: Lab/Diagnostics, log level, copy diagnostics, file paths, reset.
- About: version, build, update check, release notes, docs, report an issue.

### 5. History, Lab and the Guide

History is useful but still reads like a debug feed. "Flag False Trigger" and "Inject in 2s" are support tools on every row. Move them to an overflow menu. Keep Copy, add Paste again, Pin, Delete and Reveal raw text. Add a full History view with search, date groups, filters (app, mode, failed sends), and export. Seven days is a reasonable default, but let users choose 1 day, 7 days, 30 days or off, and tell them where the file lives.

Lab is a good idea with a bad name. "Lab" sounds experimental and makes users wonder whether the core product is stable. Rename it "Diagnostics" or "Advanced". Keep the practice box, the meters and the session stats, but add "Copy diagnostics" and "Export session report" so a support conversation can start with data instead of a screenshot.

The Guide duplicates the docs. That is not fatal, but it will drift. Make the in-app Guide the short, contextual version, and give every section a "Read the full guide" link to the exact docs page. One source of truth for the long form, one short form in the app.

### 6. Accessibility and platform polish

- The body sets `user-select: none`, so Guide, Problems and history text cannot be selected. Copy buttons cover some cases, but readers cannot select a sentence or a log line. Allow text selection on content areas, or provide Copy on every meaningful block.
- The secondary text token `--faint` is 2.62:1 on white. That fails WCAG AA for normal text, and it is used on many small labels. `--muted` is 5.24:1 and is fine. Darken `--faint` or reserve it for decorative hairlines.
- The onboarding modal needs `aria-modal`, a focus trap, initial focus, Escape to close and focus restoration. The send-mode and palette radio groups need arrow-key navigation and roving tabindex.
- The HUD is a separate, non-focusable, click-through window. That is correct for typing, but it means mouse users cannot click "Cancel" on the floating preview. Global Esc works, but a click target or a larger cancel affordance would help. Consider making only the preview card interactive while it is open.
- The HUD is fixed to the primary monitor, 24px above the bottom. On a multi-monitor setup, the user may be looking at a different display. Follow the active display, remember a position and allow dragging.
- The code only builds a tray menu. There is no explicit application menu surface for About, Settings (`Cmd+,`), Check for Updates or Help. Verify what Tauri's default menu covers on each platform, then add a proper macOS menu and a Windows equivalent so these standard actions have a home.
- There is no updater and no launch-at-login toggle. For a beta, an in-app "update available" path is a trust feature, not a nice-to-have.
- The app is English-only and the model catalog is `tiny.en` and `base.en`. That is a legitimate scope choice, but the site and docs should say "English only for now" plainly. Do not imply parity with Wispr's 100+ languages.
- Windows is advertised on the site, but the launch checklist still lists Windows hardware checks as pending. Either test it or label the Windows build experimental.

### 7. The install and trust gap is bigger than the UI gap

The launch checklist says the v0.2.0 installers predate the checksum hardening, the installers are not signed, Windows and Intel Mac hands-on checks are pending, and several permission, sleep, target-change and risky-command gates are still open. The site tells users to bypass Gatekeeper with `xattr` or SmartScreen.

This is the real competitor gap. Wispr Flow's install is one click, signed, and the app updates itself. A user who has to open Terminal and run `xattr` before the app will open is already lost, no matter how good the orb looks.

Order of work:

1. Sign and notarize the macOS build. Configure the release secrets and verify with `codesign` and `spctl`.
2. Sign the Windows installer, or clearly label it unsigned and experimental on the site.
3. Test the install on a clean macOS account and a clean Windows VM, including deny, retry and grant for camera, microphone and Accessibility.
4. Add an updater with a visible version, release notes and a beta channel.
5. Add launch at login and a menu bar/tray state that shows pause and the current send mode.
6. Publish the checksum and signing status on the download page.

## Portfolio website

The project page is visually excellent. The editorial type, the cream and plum palette, the terminal illustration, the "Whisper mishears" table and the honest numbers section are all strong. It feels like a product with a point of view, not a template.

The problems are in the plumbing.

### Mobile

At a 390px viewport, the project page's layout viewport expands to 469px. The document scrolls horizontally by about 79px. The cause is at least partly the unbreakable `git clone https://github.com/...` code block in "Ask your agent", which has a min-content width of 389px. The fix is to let code wrap (`white-space: pre-wrap; overflow-wrap: anywhere`) and to set `min-width: 0` on the code containers.

The home page is responsive at 390px, but the four featured work cards stay two-up and are too narrow. Labels clip: "REFERENCE FAILURES" becomes "CE FAILURES", and the cards are cut at the fold. Use one column, or a horizontal snap rail with full-width cards, and give the section room to breathe.

Test at 360, 390, 414 and 768 before shipping. Add a Playwright viewport check so this does not regress.

### The raw title is empty

The project page's `<title>` element contains only whitespace. JavaScript fills it after hydration, so a browser tab looks fine, but link previews, crawlers and reader modes can see no title. The OG tags are present and good. Add a server-rendered title such as "saywhen: hands-free voice for your terminal and coding agent".

### Docs and changelog are not on the site

`/docs` and `/changelog` return 404. The footer sends people to GitHub for "How it works" and "Architecture", which loses the site's navigation, search and mobile typography. For a product that asks people to trust it with a camera and microphone, the help center should be part of the product site, not a separate GitHub tab.

### The page is good at explaining, weaker at proving

The "live illustration" is a video or static terminal. The strongest differentiator is the cancelable preview, and the page does not let a visitor feel it. Add a 15-second interactive simulator: a fake terminal, a "Look" and "Talk" control, a preview card with a countdown, an "Esc" cancel and a "Nod" send. No microphone or camera permission is needed. Let the visitor try all four send styles. This would do more for conversion than another section of copy.

The "For the days you work from home" statistic is honest but weak. "62% of developers use an AI coding assistant" does not prove that anyone wants voice input, and the page itself admits it. Replace or supplement it with a claim that is about this product: speaking speed vs typing speed, the 34/34 cleanup result, or a real beta user's quote after a week of use. If you keep the work-from-home section, make it a short vignette, not a statistical argument.

### Conversion gaps

- Both download buttons are equally weighted. Detect the OS and architecture, highlight the matching build, and put the other behind "Other platforms".
- There is no system requirements block, no version number, no release date, no checksum and no signing status near the button.
- There is no "No account. No subscription. No cloud." line. This is a major advantage over Wispr Flow, and it is buried in small text.
- There is no FAQ near the download. Add the six questions people actually ask: Is it free? Does it work offline? Is my camera recorded? Does it work on Windows? Does it work on Intel Macs? Does it support languages other than English?
- There is no comparison page. "saywhen vs Wispr Flow" is a search people will make. Answer it honestly: local vs cloud, free vs subscription, preview vs instant paste, hands-free vs hotkey.
- There is no social proof. If the repo is not public yet, use "built in public" updates and beta tester quotes when you have them. Do not invent logos.
- There is no docs link in the header. Add Docs, Changelog and GitHub to the project nav and footer.

### Suggested page map

- `/` portfolio index, keep the network and the work cards, fix mobile density.
- `/p/saywhen/` product page, keep the editorial story, add the live preview simulator.
- `/docs` searchable help center, versioned.
- `/changelog` release notes, linked from the app.
- `/compare/wispr-flow` honest comparison.
- `/privacy` and `/security` data flow, retention, permissions and threat model.
- `/install` system requirements, signing status, checksums, per-platform steps.
- `/status` known issues and current release health.

### Performance and SEO

- Add JSON-LD: `SoftwareApplication` on the project page, `FAQPage` on the FAQ, `BreadcrumbList` on docs.
- Add a sitemap, `robots.txt`, and `<link rel="alternate" type="application/rss+xml">` for the changelog.
- Add `llms.txt` and `llms-full.txt`. This audience points coding agents at the repo. A stable raw install guide is already a differentiator.
- Compress the README's 1.5 MB and 1.8 MB GIFs. Use MP4 or WebM with posters for the docs site and shorter, lower-fps GIFs only where video cannot be embedded.
- Set a budget: LCP under 2.5s on a mid-range phone, no layout shift from the hero, no autoplay video with sound.

## Docs

### What is good

The user-facing prose is better than most developer tools. "How saywhen works" explains the camera, the microphone, the pre-roll and the preview without jargon. The measurements page is careful about what was and was not measured. The install-with-AI guide is a genuinely smart artifact for this audience.

### What is missing

The docs are a folder of Markdown, not a documentation product.

- There is no docs site, no search, no sidebar, no version selector and no "last updated".
- Troubleshooting is 238 words. It covers eight symptoms. There is no decision tree, no platform-specific recovery, no error codes, no log locations and no "what to send support".
- There is no FAQ, no system requirements, no supported apps matrix, no language statement, no uninstall instructions, no data path reference and no security model.
- There is no changelog or known issues page.
- `docs/README.md` links to `../README.md#see-it-work`; that anchor does not exist. The README section is `#the-real-thing`. This is a small broken link, but it is the kind of thing a docs build with an anchor checker would catch.
- The same content is copied across the README, the project page, the in-app Guide and `docs/`. They will drift. The README says "the 60-second film" while the docs page calls it a demo; version numbers and benchmark phrasing already vary.
- There is no page for each failure mode: camera sees a face but never arms, calibration says "you look away", nothing is typed, text is typed but Enter is not pressed, the target changed, Secure Input is on, the app is elevated, the microphone disconnected, the model download failed, the app is in a pure Wayland session.
- There are no screenshots or short videos for each task. The existing visuals are marketing composites, not annotated UI help.
- There are no error identifiers. Every error is free text. Support and docs cannot map a message to a fix.
- There is no docs feedback loop. No search logs, no "was this helpful", no support-ticket tagging.

### Proposed information architecture

| Section | Pages |
| --- | --- |
| Start here | What is saywhen, Install, First five minutes, Permissions, System requirements |
| Using saywhen | Look, talk, send; the preview; send styles; gestures; wake word; the hotkey; the practice box |
| Make it yours | Dictionary, snippets, per-app rules, shortcuts, appearance, sound |
| Privacy and security | Data flow, retention, local models, optional cloud cleanup, security model, threat model |
| Troubleshooting | Nothing typed, never starts, starts by mistake, wrong app, Enter not pressed, camera stalled, model download, microphone lost, secure input, elevated app, sleep and wake |
| Reference | Settings reference, shortcuts, supported apps, models, file paths, environment variables, error codes |
| Releases | Changelog, known issues, roadmap, supported platforms |
| Compare | Wispr Flow, Superwhisper, MacWhisper, Apple Dictation |
| For AI agents | Install prompt, architecture, contributing, `llms.txt` |

### Page template

Every task page should have:

1. A one-line outcome: "Get saywhen to type into your terminal."
2. "Available on" badges: macOS, Windows.
3. Prerequisites: permissions, model, app.
4. Numbered steps with one screenshot or six-second clip per step.
5. "If it does not work" with three or four likely causes and exact fixes.
6. Related links and a "Was this helpful?" control.
7. Last updated date and an "Edit this page" link.

### Delivery

- Build the docs as a static site from the repo, with Pagefind or a hosted search. Keep the engineering docs in the same build under an "Engineering" section.
- Add a docs CI job: Markdown lint, link checker, anchor checker, spelling, screenshot freshness, no em dashes (the existing rule).
- Add a version selector: `v0.2 beta` and `next`. Keep the install guide's raw Markdown at a stable URL.
- Deep-link from the app: every error should have a "Open the guide" action that lands on the exact section, plus "Copy diagnostics".
- Add error codes to the Problems list and to the docs. For example `AX_DENIED`, `CAM_NO_FRAMES`, `MODEL_MISSING`, `TARGET_CHANGED`, `SECURE_INPUT`, `UIPI_BLOCKED`, `MIC_LOST`.
- Add a changelog generated from GitHub releases and show the latest three entries on the site. Link "What's new" in the app's About section.
- Add `llms.txt`, `llms-full.txt` and a stable `/install/ai.md`.
- If you use analytics, use a privacy-friendly tool and disclose it. Track download clicks, demo plays, docs searches and "was this helpful", not personal data.

## If this were my product, in order

### Now, the next 7 days

1. Sign and notarize the macOS build. Fix the Windows signing story or label the build experimental.
2. Remove or gate the "Show the check step" off switch so the cancelable preview promise is true.
3. Fix the project page mobile overflow and the two-up featured cards.
4. Add a server-rendered title and `/docs` and `/changelog` routes.
5. Make the onboarding modal fit the default window: sticky footer, focus trap, Escape, real mic and camera status, model progress.
6. Fix the setup completion bug: do not mark "Say something" done when injection failed.
7. Add a "Copy diagnostics" action and error codes.
8. Fix the broken docs anchor and add a link checker.

### Next 30 days

1. Build onboarding v2 with an in-app practice dictation and a background model download.
2. Simplify Home to the status deck, send style, recent and camera row.
3. Rebuild Settings with sub-navigation, search, an app picker, a dictionary table, shortcut editing and dark mode.
4. Add the HUD upgrades: follow active display, drag, clickable preview, pause, paste last, optional start and stop sounds.
5. Build the docs hub with search, the troubleshooting decision tree, FAQ, system requirements, supported apps and error codes.
6. Add the interactive preview simulator and an OS-aware download block to the project page.
7. Add Playwright viewport tests, axe accessibility checks and a visual regression pass for the app and the site.

### Next 90 days

1. Dictionary learning from corrections, snippets and per-app profiles.
2. Command palette (`Cmd+K`), native menu, About, launch at login and auto-update.
3. Language support or a clear English-only statement, not an ambiguous "Whisper" claim.
4. Public comparison pages and a real beta-user proof section.
5. Docs analytics, versioning, `llms.txt` and an AI-agent install path.
6. Real-user activation and reliability numbers, not only replay numbers.

## What not to do

- Do not add accounts, cloud sync, team workspaces, subscriptions or a meeting notetaker. Those are Wispr Flow's game. They would erase the local-first advantage and add a support surface you cannot staff.
- Do not chase 100+ languages or 4x-faster marketing claims before English quality, latency and battery are measured on real users.
- Do not remove the preview to feel faster. The preview is the reason a developer can trust auto-Enter.
- Do not add more palettes. Add dark mode and high contrast instead.
- Do not copy Wispr Flow's invisible, instant-paste model. saywhen's differentiator is that it shows you the text and lets you stop it.
- Do not add fake testimonials or company logos. "Built in public, measured honestly" is a stronger brand for this audience.
- Do not polish the website while the installer is still unsigned. The first minute of the product is the download, not the hero.

## Metrics that would tell you this is working

Activation:

- Time from install to first successful in-app dictation: target under 3 minutes.
- Time from install to first successful injection into a real terminal: target under 5 minutes.
- Setup completion rate: target 80% or higher.
- Permission denial recovery rate: target 70% of denied users reach a working state.
- Model download completion rate and p95 download time.

Quality:

- Unwanted sends per 100 utterances.
- Preview cancel rate and the reasons (Esc, shake, speech, target change).
- Injection failure rate by cause and by app.
- p95 time from last word to preview, and from preview to injection.
- Support issues per 100 installs.

Docs and site:

- Download click-through rate by platform.
- Demo play rate and completion.
- Docs search success rate, top zero-result queries, "was this helpful" score.
- Mobile bounce rate and mobile horizontal-scroll errors.
- Percentage of users who reach a docs page from an in-app error.

## Sources

- saywhen project page: https://ansarbuilds.dev/p/saywhen/
- saywhen repository docs: `README.md`, `docs/`, `index.html`, `src/style.css`, `src/main.ts`, `src-tauri/src/hud.rs`, `src-tauri/tauri.conf.json`
- Wispr Flow site and pricing: https://wisprflow.ai/ and https://wisprflow.ai/pricing
- Wispr Flow help center: https://docs.wisprflow.ai/
- Wispr Flow setup guide: https://docs.wisprflow.ai/articles/3152211871-setup-guide
- Onboarding lessons from Wispr Flow: https://github.com/moona3k/macparakeet/blob/be0e5737ccc3624563c42f940dc389fd574dd229/docs/design/2026-09-23-wispr-onboarding/note.md
- Wispr Flow UX reference: https://github.com/builderio/agent-native/blob/0b51d0604fd2b82eeaab7c267e62986c49853cac/templates/clips/desktop/design-refs/wispr-ux.md
