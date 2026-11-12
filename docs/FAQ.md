# Questions people ask

Short answers first. Each one links to the page with the detail.

## Is saywhen free?

Yes. saywhen is MIT licensed. There is no account, no subscription and no paid tier. The only optional cost is an AI endpoint if you turn on cloud clean-up, and the default is a local Ollama server.

## Does it work offline?

Yes, after the speech model and the camera runtime have downloaded. The first setup needs the internet. After that, dictation, the camera gate, the check step and typing into other apps run on your computer. Optional cloud clean-up is off by default and sends text only, never audio or camera pictures.

## Does it record my camera or my voice?

No. Camera frames are read in memory and thrown away. Audio is turned into text and discarded after transcription. The text you said is kept for 7 days in local history unless you turn history off. The one exception is the opt-in "Save recordings" switch in Diagnostics, which is off by default, turns itself off on restart and writes 16 kHz WAV files to the `recordings/` folder.

## How is this different from Wispr Flow?

Wispr Flow is a polished cloud dictation app. It uses a hotkey, supports many languages and devices, and keeps its models and clean-up in the cloud. saywhen is local-first, camera-gated and built for terminals and coding agents. It has no hotkey in the default path, shows a cancelable preview before Enter, and is free and open source. Neither one is better at everything. If you want a cloud keyboard everywhere, Wispr Flow is the more mature product. If you want your voice and camera to stay on your computer and you mostly type into a terminal or an agent, saywhen is the more honest fit.

## Can I try it before allowing Accessibility?

Yes. The welcome screen has a practice box. Record a short phrase and saywhen shows you the raw transcript and the cleaned text inside the welcome screen. It does not type into another app, does not need Accessibility and does not save the practice to history. Allow Accessibility only when you are ready for saywhen to type into your terminal or editor.

## Does it work without a camera?

Yes. Press **Option + Space** on a Mac or **Alt + Space** on Windows to start and stop by hand. On Home, choose **Type only** so Enter is never pressed until you press it.

## Can I make the text easier to read?

Open **Settings > Appearance**. You can choose Light, Dark or System, turn on **High contrast** for black, white and amber, scale the whole window with **Text size** (Small, Default, Large or Extra large). The choice is remembered.

## Can I change the keyboard shortcut?

Yes. Open **Settings > Sending** and click the shortcut button. Press the new combination, or press Escape to cancel. The default is Option + Space on a Mac and Alt + Space on Windows. If another app already owns the combination, saywhen keeps the old one and tells you what happened.

## Which apps can it type into?

Any app that accepts pasted text, subject to the operating system's permissions. Enter is only pressed in apps on the trusted list in Settings. Terminals and code editors are trusted by default. Chat, email and browsers only receive typed text.

## Does it support languages other than English?

Not yet. The current model list is English only (`tiny.en` and `base.en`). The app does not claim otherwise. If you need another language, do not install saywhen expecting it to work.

## Which Macs and Windows versions are supported?

The tested build is Apple Silicon macOS. A 64-bit Windows build is produced by CI, but hands-on checks on Windows hardware are still open. An Intel Mac build is produced by CI but has not been checked on Intel hardware. The [launch checklist](LAUNCH_CHECKLIST.md) lists exactly what has been verified and what has not.

## Why does my computer warn me about the installer?

The beta is not signed by Apple or Microsoft yet, so Gatekeeper or SmartScreen adds an extra prompt. On a Mac, the README explains the `xattr` command. On Windows, click **More info**, then **Run anyway**. This warning is about the missing signature, not about the app's behaviour. Signing and notarization are required before a stable release.

## Where does saywhen keep my data?

Settings, models and the 7-day text history live in the app data directory for your user. The exact path depends on the platform. History can be cleared from Home. There is no cloud account and no server that receives your transcripts.

## Can I turn off the check step?

No. The preview is the safety promise. Every automatic send passes through a short, cancelable check before Enter. If you want a faster path, use **Nod to send** or **Type only**; both keep a way out.

## Something is broken. What should I send?

Open **Problems** on Home, click **Copy** on the message, and include:

- what you said and which app was in front
- the operating system and app version
- whether the camera and microphone were on
- the exact text from Problems

Never send a recording with private content unless you choose to. Do not paste API keys or tokens into an issue.

## Still stuck?

Read [when something goes wrong](TROUBLESHOOTING.md), or [open an issue](https://github.com/ansarbuilds/saywhen/issues/new/choose).
