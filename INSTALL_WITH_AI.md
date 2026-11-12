# Install saywhen with an AI assistant

This file is written for an AI coding assistant (Claude Code, Cursor, Codex, Windsurf and similar) to follow on behalf of a person who may not be technical. If you are a person, you can follow it too: every step is a command you can copy.

If this repository is already open, you are in the right place. Do not clone it again. Start at Step 1 and run every command from this folder.

## Rules for the assistant

- **Ask before installing anything system-wide** (Homebrew, Rust, CMake, Node.js, Visual Studio Build Tools). Say what it is, why saywhen needs it, and roughly how big it is.
- **Explain each step in one plain sentence** before running it. Avoid jargon. Say "the tool that builds the app" rather than "the Rust toolchain".
- **Never type passwords for the person.** If a step needs an admin password (for example the Homebrew installer), tell them a password prompt will appear in their terminal and wait for them.
- **Stop and explain if a step fails.** Show the last few lines of the error and suggest the next thing to try. Do not skip a failed step silently.
- **Do not change system security settings yourself.** Guide the person to click the right switches (camera, microphone, Accessibility).

## Step 1: Check the computer

Find out the operating system and which tools already exist:

```bash
uname -s; node --version; cargo --version; cmake --version; git --version
```

On Windows (PowerShell), use:

```powershell
node --version; cargo --version; cmake --version; git --version
```

saywhen needs:

| Tool | Version | Why |
| --- | --- | --- |
| Git | any | Downloads the saywhen code |
| Node.js | 20 or newer | Builds the app window |
| Rust (cargo) | 1.85 or newer | Builds the app engine |
| CMake | any recent | Builds the speech-to-text engine |
| Windows only: Visual Studio Build Tools with "Desktop development with C++" | 2022 | The C++ compiler Rust uses on Windows |

## Step 2: Install what is missing (ask first)

### macOS

1. If `brew` is missing, ask, then install Homebrew from [brew.sh](https://brew.sh). The person will need to type their Mac password in the terminal.
2. Then install the rest:

```bash
brew install node cmake git
```

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

```bash
source "$HOME/.cargo/env"
```

3. Xcode Command Line Tools are needed. If `xcode-select -p` fails, run `xcode-select --install` and ask the person to click Install in the popup.

### Windows

Install, in this order, after asking:

1. **Visual Studio Build Tools 2022** from [visualstudio.microsoft.com/downloads](https://visualstudio.microsoft.com/downloads/) with the "Desktop development with C++" workload.
2. **Node.js LTS** from [nodejs.org](https://nodejs.org).
3. **Rust** from [rustup.rs](https://rustup.rs) (choose the default options).
4. **CMake** from [cmake.org/download](https://cmake.org/download/) (tick "Add CMake to the system PATH").
5. **Git** from [git-scm.com](https://git-scm.com/download/win).

Close and reopen the terminal afterwards so the new tools are found. WebView2 is already part of Windows 10 and 11.

## Step 3: Get the code

If `INSTALL_WITH_AI.md` is already in the current folder, the code is here. Stay in this folder.

If it is not, download it:

```bash
git clone https://github.com/ansarbuilds/saywhen.git
```

```bash
cd saywhen
```

Then install the app's own packages (this does not install system tools):

```bash
npm ci
```

## Step 4: Start it

```bash
npx tauri dev
```

The first run downloads and verifies the camera model (about 4 MB) and prepares its runtime automatically. If this fails, check the connection and retry; do not skip asset preparation.

The first build compiles everything and can take 5 to 15 minutes. Tell the person this is normal. When it finishes, the saywhen window opens, plus a small status pill near the bottom of the screen.

On macOS the development build runs as **saywhen-dev.app**. That name is what will appear in permission prompts.

**Important on macOS:** macOS gives camera and microphone access to the app that *started* saywhen. If you (the assistant) start it from your own background shell, the camera is usually blocked with "NotAllowedError". So do not start it yourself. Ask the person to open the **Terminal** app, then run `cd saywhen && npx tauri dev` there, so the permission prompts appear for them.

## Step 5: Walk the person through first launch

Say these steps out loud, one at a time, and wait for them to confirm each one:

1. **Download a speech model:** open **Settings** (the second item in the sidebar), find **Speech model** and click **Download**. "Tiny" (about 75 MB) is fastest. "Base" is more accurate.
2. **Allow the microphone and camera** when the computer asks.
3. **macOS only, allow Accessibility:** this lets saywhen type into other apps. Click **Setup guide** in the sidebar, then the orange badge next to Accessibility. In System Settings, turn on the switch for saywhen-dev. If there are old saywhen entries, remove them first.
4. **Pick a send style** on the Home screen. Suggest **Type only** for the very first try, so nothing is ever sent by accident.
5. **Turn on the camera** on Home, look at the screen with a closed mouth and click **Calibrate**.
6. **Try it:** open a terminal, look at it, say "git status", and watch the text appear.

## Step 6: Confirm it works

Ask the person:

- Did the status pill turn from "Waiting" to "Ready" when they looked at the screen?
- Did it show "Listening" when they talked?
- Did the words appear in the terminal?

If something did not work, open the **Problems** list on the Home screen, read the message together, and use the table in [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md).

## Updating later

```bash
cd saywhen && git pull && npm ci && npx tauri dev
```
