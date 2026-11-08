# saywhen

> **Look. Talk. Sent.** Talk to your terminal or coding assistant without touching the keyboard.

saywhen is a free, open source desktop app for macOS and Windows. You look at your screen, say what you want, and saywhen types it into the app you are using (Terminal, Cursor, VS Code, Claude Code and more). Then it presses Enter for you, unless you stop it.

No hotkey to hold. No "click Send". Everything runs on your own computer.

![saywhen main window](docs/screenshots/stage4-cleanup-preview.png)

---

## How it works, in one minute

1. **Look** at your screen. saywhen uses your webcam to notice you are facing it. *(It only looks at a few numbers, like how your head is turned. Pictures are never saved or sent anywhere.)*
2. **Talk.** When your lips move and the microphone hears your voice, it starts listening.
3. **Stop talking.** saywhen writes down what you said, tidies it up (`dash dash release` becomes `--release`), and shows it to you for a moment.
4. **It sends.** The text is typed into your app and Enter is pressed. Changed your mind? Say "cancel", shake your head or press **Esc**.

Want the longer, friendly explanation? Read **[How saywhen works](docs/HOW_IT_WORKS.md)**.

---

## Pick how you send

You choose this once on the Home screen. You can change it any time.

| Send style | What you do | Good for |
| --- | --- | --- |
| **Just pause** (default) | Stop talking. A short countdown runs, then it sends. | Fast, hands-free work |
| **Nod to send** | Nothing is sent until you nod your head. | People who want to be sure |
| **Say "send it"** | End with "send it" to press Enter. Otherwise the text is only typed. | Mixing notes and commands |
| **Type only** | Your words are typed, Enter is never pressed. | Trying saywhen for the first time |

In every style you can always stop a send:

- **Shake your head** (no) or press **Esc** to cancel.
- **Nod** (yes) or press **Enter** to send right away.
- **Start talking again** during the countdown to cancel it (you can turn this off in Settings).
- Press **Option + Space** (Mac) or **Alt + Space** (Windows) to start or stop by hand, no camera needed.

### Handy things to say

| Say this | What happens |
| --- | --- |
| "...send it" | Sends and presses Enter |
| "...paste only" | Types it without pressing Enter, just this once |
| "scratch that" | Throws away what you said just before it |
| "cancel that" / "never mind" | Throws away the whole thing |

### Safety built in

- Enter is only pressed in apps you trust (terminals and code editors by default). In chat apps, email and browsers, saywhen only types.
- Risky commands such as `rm -rf`, `git reset --hard` or `git push --force` are **never** sent automatically. They are only typed, with a warning.
- If you switch apps during the countdown, saywhen stops instead of typing into the wrong place.

---

## Install

### The easy way: let your AI assistant do it

If you use **Claude Code**, **Cursor** or another coding assistant, open it in an empty folder and paste this:

```text
Install saywhen for me by following the steps in
https://github.com/kmsmohamedansar/saywhen/blob/main/INSTALL_WITH_AI.md
Ask me before installing anything on my computer, and explain each step simply.
```

It will check your computer, install what is missing (after asking you), download saywhen and start it. The full step list it follows is in **[INSTALL_WITH_AI.md](INSTALL_WITH_AI.md)**.

### The manual way

You need three free tools first:

- **Node.js 20 or newer:** [nodejs.org](https://nodejs.org)
- **Rust:** [rustup.rs](https://rustup.rs)
- **CMake:** on a Mac run `brew install cmake`, on Windows install it from [cmake.org](https://cmake.org/download/)

Then, in a terminal:

```bash
git clone https://github.com/kmsmohamedansar/saywhen.git
```

```bash
cd saywhen && npm install
```

```bash
npx tauri dev
```

The first build takes a few minutes. After that it starts in seconds.

### First launch checklist

1. **Download a speech model.** Go to **Settings > Speech model** and click **Download** (Tiny is about 75 MB and the fastest).
2. **Allow the microphone and camera** when your computer asks.
3. **Allow Accessibility (Mac only).** This lets saywhen type into other apps. The Setup guide in the sidebar has a button that opens the right settings page.
4. **Turn on the camera** on the Home screen and click **Calibrate** while looking at your screen with your mouth closed.
5. Open your terminal, look at it, and say something like "git status".

---

## Is it private?

Yes, by design.

- **Camera:** pictures stay in memory for a split second and are thrown away. Only numbers (head angle, how open your mouth is) are used. Nothing is saved or sent.
- **Microphone:** your voice is turned into text on your own computer. The audio is deleted right after.
- **History:** the last 7 days of text are kept on your computer so you can search and copy them. Turn this off in Settings, or clear it with one click.
- **Internet:** saywhen works fully offline once the speech model is downloaded. The optional "AI clean-up" setting is off by default, and if you turn it on you choose where the text goes (a local Ollama is the default).

---

## When something goes wrong

| Problem | Try this |
| --- | --- |
| Nothing gets typed | On a Mac, check **System Settings > Privacy & Security > Accessibility** and allow saywhen. |
| It never starts listening | Make sure the camera is on and click **Calibrate**. Or use the hotkey. |
| I want to talk while looking somewhere else | Turn on **Settings > Wake word when you look away** and start with "Eevee", like "Eevee, run the tests". |
| My webcam is to the side of my screen | Look at the screen you talk to, keep your mouth closed and click **Calibrate**. saywhen notices the camera is off to the side and adjusts by itself. It works up to about a 65 to 70 degree head turn. |
| It starts when I do not want it to | Turn up **Settings > Noise filter**, or pick **Nod to send**. |
| My first or last word is cut off | Speak a little more slowly at the end. Report it so we can tune it. |
| It typed but did not press Enter | The app may not be on the trusted list (Settings > Apps where Enter can be pressed), or the command looked risky. |
| Something else | Look at **Problems** on the Home screen. You can copy the message and open an issue. |

---

## Learn more

- **[How saywhen works](docs/HOW_IT_WORKS.md):** the simple explanation of the camera, the listening and the sending.
- **[How we built it](docs/HOW_WE_BUILT_IT.md):** the story, stage by stage, with the measurements.
- **[Vision](docs/VISION.md)**, **[Architecture](docs/ARCHITECTURE.md)**, **[Roadmap](docs/ROADMAP.md)**, **[Risks](docs/RISKS.md)**, **[Prior art](docs/PRIOR_ART.md)**: the deeper technical documents.

---

## For developers

saywhen is a [Tauri v2](https://tauri.app) app. The backend is Rust, the window is TypeScript, HTML and CSS.

### Commands

```bash
npm run build
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin replay -- recordings
```

```bash
./scripts/capture-screenshots.sh
```

### Measured numbers (Apple Silicon)

| Part | What we measured | Result |
| --- | --- | --- |
| Face tracking (MediaPipe) | Time per camera frame | 12 to 15 ms |
| "Facing the screen" check | Telling facing from turned away | 100% on our test sessions |
| "Mouth still" check | Ignoring a closed or still mouth | 96.2% rejected correctly |
| Start and end of speech | 56 recorded sentences | 0% false starts (was 36.4%), 0% cut-offs (was 52.9%) |
| Developer word clean-up | 34 spoken commands | 34 of 34 exact (was 44.1%) |
| Speech to text (Whisper tiny.en, GPU) | 1.5 s of audio | 87 to 473 ms |
| Finding the front app (macOS) | Median of 20 calls | 1 to 2 microseconds |

### Repo map

| Path | What it is |
| --- | --- |
| [src-tauri/src/audio.rs](src-tauri/src/audio.rs) | Microphone capture, 500 ms pre-roll, voice detection, the 5-state machine, send modes, settings |
| [src-tauri/src/turn_taking.rs](src-tauri/src/turn_taking.rs) | Pure decision logic: when to start, when you are done, send mode rules, speech-cancel |
| [src-tauri/src/bin/replay.rs](src-tauri/src/bin/replay.rs) | Offline replay benchmark over recorded sentences |
| [src-tauri/src/transcribe.rs](src-tauri/src/transcribe.rs) | Whisper on the GPU, model downloader, "send it" detection |
| [src-tauri/src/cleanup.rs](src-tauri/src/cleanup.rs) | Developer word clean-up, "scratch that", custom words, risky command guard |
| [src-tauri/src/history.rs](src-tauri/src/history.rs) | 7-day local history |
| [src-tauri/src/inject.rs](src-tauri/src/inject.rs) + [inject/](src-tauri/src/inject/) | Typing into other apps, per-app rules, macOS, Windows and Linux front-app detection |
| [src-tauri/src/hud.rs](src-tauri/src/hud.rs) | Floating status pill, Esc to cancel, tray menu |
| [src/main.ts](src/main.ts) | Camera face tracking, head gestures, the main window logic |
| [src/ui.ts](src/ui.ts), [src/style.css](src/style.css), [index.html](index.html) | Main window look and navigation |
| [src/hud.ts](src/hud.ts), [src/hud.css](src/hud.css), [hud.html](hud.html) | Floating status pill and check card |
| [src/fonts.ts](src/fonts.ts) | Bundled fonts (Plus Jakarta Sans, Instrument Serif, JetBrains Mono, Outfit) |

Contributions are welcome. Please keep the principles in [AGENTS.md](AGENTS.md): local-first, private, always cancelable, fail loudly, measure everything.

## License

MIT. See [LICENSE](LICENSE).
