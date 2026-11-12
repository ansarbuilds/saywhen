# Launch template

Copy for the day you tell people about saywhen. Fill the blanks, then paste. This file does not replace the [launch checklist](LAUNCH_CHECKLIST.md). The checklist still has to pass before you call a build stable or signed.

## Fill these in first

| Blank | Write this |
| --- | --- |
| `[VERSION]` | The tag you are actually shipping, such as `v0.2.0` |
| `[DATE]` | The day you post |
| `[ONE_USE]` | One real thing you did with it this week, in your own words. One sentence. |
| `[MAC_STATUS]` | `unsigned beta` until Apple signing and notarization are verified |
| `[WINDOWS_STATUS]` | `unsigned beta` until SmartScreen signing is verified |
| `[ASK]` | The one thing you want back: a home-day try, a missed first word, a false start |

Links, already live:

- Repo: https://github.com/ansarbuilds/saywhen
- This release: https://github.com/ansarbuilds/saywhen/releases/tag/v0.2.0
- Project page: https://ansarbuilds.dev/p/saywhen/
- 60-second film: https://github.com/ansarbuilds/saywhen/releases/download/v0.2.0/saywhen-demo.mp4
- 12-second teaser: https://github.com/ansarbuilds/saywhen/releases/download/v0.2.0/saywhen-announce.mp4

## The line

Use this as the first sentence everywhere.

Look at the screen. Say the command. saywhen types it, and can press Enter.

## Short post

For X, Bluesky, Mastodon, or a LinkedIn opener. Stay under 280 characters after you fill `[ONE_USE]`.

```text
Look at the screen. Say the command. saywhen types it, and can press Enter.

No hotkey. The camera picture is never saved. Speech stays on your computer. You see the words before they go anywhere.

[ONE_USE]

Beta for Mac and Windows. Not signed yet.
https://github.com/ansarbuilds/saywhen
```

A longer post, for LinkedIn or a blog intro:

```text
I made saywhen, a free desktop app for talking to the terminal or coding agent in front of you.

You look at the screen and talk. The camera checks that a face is there, facing the screen, with lips moving. The microphone checks for speech. Those have to agree before it listens, so there is no key to hold. The words are written on your computer. A short preview appears. Esc, a head shake, or talking again throws them away. If you let it go, saywhen types them and can press Enter.

[ONE_USE]

It is a beta for Mac and Windows. This build is not signed by Apple or Microsoft yet. After the speech model is downloaded, it works offline. The picture from the camera is never saved.

https://ansarbuilds.dev/p/saywhen/
```

## Show HN

Title:

```text
Show HN: saywhen – look at the screen and say the command
```

Body:

```text
saywhen is a free, open-source desktop app for Mac and Windows. You look at the screen and talk. It types the command into the focused app and can press Enter. You can stop it.

I built it because dictation tools made me hold a key, and then press Enter after I had already said the thing. For a short prompt to a coding agent, that was most of the work.

How it starts: a face is in frame, the head is turned toward the screen, the lips are moving, and the microphone hears speech. All four have to be true. The camera picture stays in memory for a moment and is thrown away. Speech is transcribed on the machine with Whisper. A preview shows the cleaned text. Esc, a head shake, or speaking again cancels it.

[ONE_USE]

What is honest about this beta:

- [MAC_STATUS] on Mac, [WINDOWS_STATUS] on Windows. macOS may say the app is damaged until you clear the quarantine flag. Windows SmartScreen asks an extra question. The README has the one step. You can also build it from the repo.
- Enter is pressed only in apps you mark as trusted. Risky commands such as rm -rf are typed with a warning. You press Enter yourself. That check is a heuristic, not a guarantee.
- If you switch apps during the preview, it does not type into the new app.
- The turn-taking numbers in the README are from generated clips and simulated camera traces, not from a study of live users.
- Real-device checks for a signed Mac build, Intel Macs, and Windows are still open: https://github.com/ansarbuilds/saywhen/blob/main/docs/LAUNCH_CHECKLIST.md

Try Type only first, so Enter is never pressed. The manual shortcut is Option+Space on Mac and Alt+Space on Windows.

Repo: https://github.com/ansarbuilds/saywhen
Project page, with the film: https://ansarbuilds.dev/p/saywhen/
Release [VERSION]: https://github.com/ansarbuilds/saywhen/releases/tag/[VERSION]

[ASK]
```

## Reddit

r/programming, r/rust, r/sideproject, or r/macapps. Drop the "Show HN" framing. Keep the limits.

Title:

```text
saywhen: look at the screen, say the command, it can press Enter
```

Use the Show HN body. Add this first line if the subreddit wants a personal lead:

```text
I have been using this on home days to talk to the agent in the terminal. [ONE_USE]
```

## Release notes

Paste under the GitHub release. Replace the draft if the tag already has notes that overclaim.

```text
## saywhen [VERSION]

Look at the screen. Say the command. saywhen types it, and can press Enter.

Beta for Mac and Windows. [MAC_STATUS]. [WINDOWS_STATUS].

### Try it

1. Install from the assets below, or build from the repo.
2. Download the Tiny speech model in Settings.
3. Allow the microphone, the camera, and Accessibility on Mac.
4. Pick Type only for the first try.
5. Calibrate while looking at the screen with your mouth closed.
6. Say "git status" to a terminal you chose.

### This build

- [DATE]
- [ONE_USE]

### Still true

- Camera pictures are not saved and are not sent.
- Audio is discarded after transcription unless you opt in to local tuning recordings.
- Optional cloud cleanup stays off until you turn it on.
- After the speech model download, dictation works offline.

Known limits and the open device gates are in docs/LAUNCH_CHECKLIST.md.
```

## One stat, if you want one

Use at most one of these, with the source in the same sentence. Do not stack them.

- In the 2026 Stack Overflow Developer Survey, 34.8% of the 24,602 people who described their current work situation said it was remote.
- That survey's analysis says remote work was 48% in the United States.
- Of the 17,464 people who answered Stack Overflow's 2026 question about AI at work, 65.9% said they use an AI coding assistant or coding agent.

Follow any of those with: Those numbers describe the survey. They do not count who wants voice input.

## Do not say

- Most developers want this, or any share of developers who dictate while coding. No survey here measured that.
- Most developers work from home. Remote was 34.8% in that 2026 table. Hybrid and flexible are not the same as home.
- A count of vibe coders.
- A measured time saving from skipping a hotkey or Enter.
- Signed, notarized, or "no security prompt" until that exact installer has been checked.
- Stable, for Intel Macs or for Windows, while those rows in the launch checklist are still pending.
- The film or the GIFs as proof of speed. They are illustrations. The measured numbers already live in the README table.

## Before you hit post

- The link in the post is the release you tested, not an older asset.
- The project page plays the film.
- The README install steps match what you just did on a machine.
- The post says the build is unsigned, if it is.
- You can cancel a send, and the post says so.
- `[ONE_USE]` is something that happened, not a slogan.
