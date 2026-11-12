<p align="center">
  <img src="docs/visuals/hero.jpg" alt="saywhen. Look. Talk. Sent. Hands-free voice input for your terminal and coding agent, shown beside the app's Home screen with a preview of the command cargo test --release." width="100%">
</p>

<p align="center">
  <a href="https://github.com/ansarbuilds/saywhen/releases/download/v0.2.0/saywhen_0.2.0_aarch64.dmg"><img src="https://img.shields.io/badge/Download_for_Mac-Apple_Silicon-fbb303?style=for-the-badge&logo=apple&logoColor=22061c&labelColor=fbb303" alt="Download for Mac (Apple Silicon)"></a>
  <a href="https://github.com/ansarbuilds/saywhen/releases/download/v0.2.0/saywhen_0.2.0_x64-setup.exe"><img src="https://img.shields.io/badge/Download_for_Windows-64--bit-dfe5ec?style=for-the-badge&labelColor=dfe5ec" alt="Download for Windows (64-bit)"></a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-fbb303.svg" alt="License: MIT"></a>
  <a href="https://github.com/ansarbuilds/saywhen/releases/tag/v0.2.0"><img src="https://img.shields.io/github/v/release/ansarbuilds/saywhen?include_prereleases&color=fbb303&label=beta" alt="Latest beta release"></a>
  <a href="https://github.com/ansarbuilds/saywhen/actions/workflows/ci.yml"><img src="https://github.com/ansarbuilds/saywhen/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center">
  <a href="#see-it-work">See it work</a> ·
  <a href="#install">Install</a> ·
  <a href="#you-stay-in-control">Safety</a> ·
  <a href="#private-by-design">Privacy</a> ·
  <a href="docs/HOW_IT_WORKS.md">How it works</a> ·
  <a href="https://ansarbuilds.dev/p/saywhen/">Project page</a>
</p>

# saywhen

**You talk to a coding agent all day. Stop holding a key to do it.**

saywhen listens when you look at the screen and start talking. It writes down what you said, shows it to you for a moment, then types it into your terminal or editor and presses Enter. Everything runs on your own computer.

![Three steps. Look: the pill says Ready. Talk: the pill says Listening. Check, sent: a preview of cargo test --release counts down.](docs/visuals/steps.jpg)

## See it work

![A real screen recording. The saywhen pill goes from Ready to Listening, a preview of the spoken request counts down, and the request is typed into Cursor and sent.](docs/visuals/real-run.gif)

*A real screen recording, in real time: a spoken request goes into Cursor. No keys were pressed.*

**The 60-second film, with sound:**

https://github.com/user-attachments/assets/2ac61c5f-9fce-4fb5-b146-e4e08f66a520

No player? [Watch it on the project page](https://ansarbuilds.dev/p/saywhen/).

## Why you might want it

| | |
| --- | --- |
| **No hotkey** | Looking at the screen is the wake-up. The camera and the microphone both have to agree, so chewing, typing or talking to someone else does not set it off. |
| **Speaks developer** | "cargo test dash dash release" becomes `cargo test --release`. Add your own words for project names. |
| **You see it first** | Every send goes through a short preview. Esc, a head shake, or talking again throws it away. |
| **Stays on your computer** | Speech becomes text locally with Whisper. Camera pictures are never saved or sent. It works offline. |
| **Careful with Enter** | Enter is pressed only in apps you trust. `rm -rf` and friends are typed with a warning and left for you. |
| **A shortcut, too** | **Option + Space** on Mac, **Alt + Space** on Windows starts listening with no camera at all. |

![The saywhen Home screen while listening, with the camera check, the sound level and the five states from Waiting to Check.](docs/visuals/app.jpg)

## Install

### Point Claude or Cursor at this repo

Open this folder and paste:

```text
Set up saywhen on my computer.

This repository is already open. Follow INSTALL_WITH_AI.md from the top.
Do not clone it again.
Ask me before you install anything. Explain each step in one plain sentence.
On a Mac, do not start the app yourself. Tell me the command to run in the Terminal app.
When the window is open, walk me through First launch in the README, one step at a time.
```

You only have the link? Paste this instead:

```text
Clone https://github.com/ansarbuilds/saywhen and open that folder.
Then follow INSTALL_WITH_AI.md from the top.
Ask me before you install anything. Explain each step in one plain sentence.
On a Mac, do not start the app yourself. Tell me the command to run in the Terminal app.
```

The full checklist is [INSTALL_WITH_AI.md](INSTALL_WITH_AI.md).

### Download the app

Get the beta installer from the **[v0.2.0 release](https://github.com/ansarbuilds/saywhen/releases/tag/v0.2.0)**:

- **Mac:** the `.dmg` file (`aarch64` is Apple Silicon, `x64` is Intel when that build is included). Drag saywhen into Applications.
- **Windows:** the `-setup.exe` file.

If your Mac says saywhen "is damaged" or "cannot be opened", this beta is not signed by Apple yet. Run this once, then open the app again:

```bash
xattr -cr /Applications/saywhen.app
```

On Windows, SmartScreen appears for the same reason. Click **More info**, then **Run anyway**.

### Build it yourself

You need Node.js 20 or newer, Rust, and CMake. On Windows, also install Visual Studio Build Tools 2022 with the "Desktop development with C++" workload.

```bash
git clone https://github.com/ansarbuilds/saywhen.git
cd saywhen
npm ci
npx tauri dev
```

The first run downloads the camera model (about 4 MB) and can take 5 to 15 minutes to compile. On a Mac, run that last command in the **Terminal** app. A background shell usually cannot get camera permission. The development app is named **saywhen-dev**.

### First launch

1. **Download a speech model.** Settings, then Speech model, then Download. Tiny is about 75 MB and the fastest. Base is more accurate.
2. **Allow the microphone and camera** when your computer asks.
3. **Allow Accessibility (Mac only).** Setup guide in the sidebar, then the orange badge. Turn on **saywhen**, or **saywhen-dev** if you built it. Remove old saywhen entries first if macOS is confused.
4. **Pick Type only** on Home for a first try, so Enter is never pressed.
5. **Turn on the camera.** Look at the screen with your mouth closed and click **Calibrate**.
6. Open a terminal, look at it, and say "git status".

## You stay in control

Pick how words get sent. You can change it any time on Home.

| Send style | What you do | Good for |
| --- | --- | --- |
| **Just pause** (default) | Stop talking. A short countdown runs, then it sends. | Fast, hands-free work |
| **Nod to send** | Nothing is sent until you nod. | When you want to be sure |
| **Say "send it"** | End with "send it" to press Enter. Otherwise the text is only typed. | Mixing notes and commands |
| **Type only** | Your words are typed. Enter is never pressed. | The first time you try saywhen |

- **Cancel:** press **Esc**, shake your head, or start talking again during the countdown. You can also say "scratch that" or "never mind".
- **Trusted apps only:** terminals and code editors get Enter by default. In chat, email and browsers, saywhen only types.
- **Risky commands:** `rm -rf`, `git reset --hard`, `git push --force` and similar are typed with a warning. You press Enter yourself. This check is a heuristic, not a guarantee.
- **Switched apps?** If the app in front changes during the countdown, saywhen stops and types nothing.

## Private by design

- **Camera.** Each picture is looked at for a split second and thrown away. saywhen keeps a few numbers, such as head angle. Nothing is saved or sent.
- **Microphone.** Your voice becomes text on this computer, then the audio is discarded.
- **History.** The last 7 days of text stay on your computer. Turn it off or clear it with one click.
- **Internet.** After the speech model is downloaded, saywhen works offline. Cloud clean-up is off until you turn it on.
- **Open source.** Read the code, or [build it yourself](#build-it-yourself) and skip the downloaded app.

## Beta, honestly

saywhen is a working beta. It runs on an Apple Silicon Mac, and automated tests pass on macOS and Windows. The installers are not signed yet, and hands-on checks on Windows hardware and Intel Macs are still open. The [launch checklist](docs/LAUNCH_CHECKLIST.md) says exactly what has been verified, and [what has been measured](docs/MEASUREMENTS.md) has the numbers.

Something not working? See [when something goes wrong](docs/TROUBLESHOOTING.md) or [open an issue](https://github.com/ansarbuilds/saywhen/issues/new/choose).

## For coding assistants

A person opened this repository in Claude, Cursor, or a similar tool. Read this before you install, run, or change anything.

**They want it running.** Follow [INSTALL_WITH_AI.md](INSTALL_WITH_AI.md) from the top. That file is the procedure. Do not invent a shorter one.

- If you can read this file, the code is already here. Do not clone a second copy. Run commands from this folder.
- Ask before installing anything system-wide: Homebrew, Rust, Node.js, CMake, or Visual Studio Build Tools. Say what it is, why saywhen needs it, and roughly how big it is.
- Explain each step in one plain sentence before you run it. Say "the tool that builds the app" rather than "the Rust toolchain".
- Never type their password. If a prompt will appear, tell them and wait.
- If a step fails, stop. Show the last few lines of the error. Do not skip it.
- Do not change System Settings yourself. Tell them which switch to click.
- On a Mac, do not run `npx tauri dev` from your own background shell. macOS gives the camera and microphone to the program that started saywhen, and a background shell is usually blocked (`NotAllowedError`). Ask them to open the **Terminal** app and run the command there. The dev app is named **saywhen-dev**.
- When the window opens, walk them through [First launch](#first-launch), one step at a time, and wait for each answer.
- You are done when a speech model is downloaded, the camera and microphone are allowed, and a short phrase such as "git status" shows up in a terminal they chose.

**They want you to change the code.** Read [AGENTS.md](AGENTS.md) (same guidance as [CLAUDE.md](CLAUDE.md)), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and [docs/ROADMAP.md](docs/ROADMAP.md) first. Keep the app local-first and private. Every automatic send must stay cancelable. If the camera, microphone, model, or typing fails, say so. Claims about speed or accuracy need a measurement. Do not commit model files (`*.bin`, `*.gguf`, `*.onnx`), recordings, or `settings.local.json`.

**They want to understand it.** Explain [How it works](docs/HOW_IT_WORKS.md) in plain language. The longer version is [docs/HOW_IT_WORKS.md](docs/HOW_IT_WORKS.md).

## For developers

saywhen is a [Tauri v2](https://tauri.app) app. Rust runs the engine. TypeScript draws the window.

```bash
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

| Path | What it is |
| --- | --- |
| [src-tauri/src/audio.rs](src-tauri/src/audio.rs) | Microphone, 500 ms pre-roll, voice detection, the 5-state machine, settings |
| [src-tauri/src/turn_taking.rs](src-tauri/src/turn_taking.rs) | When to start, when you are done, send modes, speech-cancel |
| [src-tauri/src/transcribe.rs](src-tauri/src/transcribe.rs) | Whisper on the GPU, model download, "send it" detection |
| [src-tauri/src/cleanup.rs](src-tauri/src/cleanup.rs) | Developer word clean-up, "scratch that", custom words, risky-command guard |
| [src-tauri/src/inject.rs](src-tauri/src/inject.rs) | Typing into other apps, per-app rules, front-app detection |
| [src/main.ts](src/main.ts) | Camera, head gestures, main window |

**[How it works](docs/HOW_IT_WORKS.md)** · **[How we built it](docs/HOW_WE_BUILT_IT.md)** · **[Architecture](docs/ARCHITECTURE.md)** · **[Measurements](docs/MEASUREMENTS.md)** · **[Roadmap](docs/ROADMAP.md)** · **[Contributing](CONTRIBUTING.md)** · **[Code of Conduct](CODE_OF_CONDUCT.md)** · **[Security](SECURITY.md)**

## License

MIT. See [LICENSE](LICENSE). Built by [Mohamed Ansar](https://ansarbuilds.dev).
