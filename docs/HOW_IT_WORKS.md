# How saywhen works

This page explains saywhen without any technical words. If you want the engineering detail, see [ARCHITECTURE.md](ARCHITECTURE.md).

## The big idea

Imagine talking to a friend who is helping you at your computer. You do not press a button before you speak. Your friend just *knows* you are talking to them, because you are looking at them and your mouth is moving. And they wait until you are finished before they act.

saywhen tries to be that friend for your computer.

## The four helpers inside saywhen

### 1. The watcher (camera)

The watcher looks through your webcam about 15 times a second and answers three yes-or-no questions:

- Is there a face?
- Is it looking at the screen?
- Are the lips moving like someone talking?

It does this by finding a few hundred dots on your face (corners of your eyes, edges of your lips) and doing a little maths on them. **The picture itself is thrown away straight after.** Nothing is saved, nothing leaves your computer. Only the yes-or-no answers are used.

**What if the camera is not in front of you?** Lots of people use a laptop camera next to a bigger monitor, so the camera sees the side of their face. Switch on **Camera is off to the side** in Settings, look at the screen you actually talk to, and click **Calibrate**. saywhen then treats that direction as "looking at the screen". It works until your head is turned about 65 degrees from the camera. Past that, the camera cannot see enough of your lips, so use the keyboard shortcut instead.

**Talking without looking (wake word).** Switch on **Wake word when you look away** in Settings. Then, when the camera does not see you facing the screen, start your sentence with the wake word: "Eevee, run the tests". saywhen turns the first couple of seconds into text on your computer. If they do not start with "Eevee", it throws them away straight away and nothing happens. If they do, it removes "Eevee" and carries on as usual, with the check step and the countdown. A soft chime tells you it caught the wake word (one note means "go on, I am listening for the rest"). You can pick your own wake word, but a short unusual name works best: everyday words like "alright" would start by mistake.

The watcher also notices two head movements:

- **A nod** means "yes, send it".
- **A shake** means "no, stop".

### 2. The listener (microphone)

The listener checks the microphone about 30 times a second and asks: "Is this a human voice, or just noise?"

It also keeps the last half second of sound in a small memory, like a tape that keeps rewinding. That way, when you start talking, your very first word is already saved, even though saywhen needed a moment to notice you.

**saywhen only starts recording when the watcher AND the listener both say yes.** So a podcast playing in the background, or someone else talking in the room, will not set it off. Your lips are not moving, so the watcher says no.

### 3. The writer (speech to text)

When you stop talking, the writer turns your recording into text. It uses a speech model called Whisper that runs on your own computer (on a Mac it uses the graphics chip). On an Apple Silicon Mac a short sentence takes well under half a second.

Then a tidy-up step fixes the words developers use a lot:

| You say | It writes |
| --- | --- |
| "cargo test dash dash release" | `cargo test --release` |
| "cubectal get pods" | `kubectl get pods` |
| "um, git status" | `git status` |

You can teach it your own words too (project names, tools) in Settings.

### 4. The sender (typing it for you)

The sender shows you the text for a moment, then types it into the app you are using, and presses Enter if it is allowed to.

How it decides to send depends on the **send style** you picked:

| Send style | When it sends |
| --- | --- |
| **Just pause** | After a short countdown (about 2 seconds) |
| **Nod to send** | Only when you nod. If you do not nod in 15 seconds, nothing is sent. |
| **Say "send it"** | Only presses Enter if your sentence ended with "send it" |
| **Type only** | Never presses Enter, only types |

And you can always stop it: shake your head, press Esc, or (in the countdown) just start talking again.

## The five moods of saywhen

You can see which mood saywhen is in by the colour of the window and the small floating pill. The colours below are the defaults, taken from the saywhen logo (plum, silver and amber). In **Settings > Colours** you can pick another palette and choose whether the glow changes with each mood. There is also a **Guide** page inside the app with the do's and don'ts.

| Mood | What it means | Colour |
| --- | --- | --- |
| **Waiting** | Nobody is looking at the screen | Calm plum |
| **Ready** | You are looking. Start talking any time. | A soft silver glow |
| **Listening** | You are talking and it is recording | Amber, the bars move with your voice |
| **Writing** | It is turning your voice into text | Lilac, with a spinning ring |
| **Check** | It is showing you the text before sending | Amber countdown bar |

## The safety rules

saywhen is careful about pressing Enter, because Enter makes things happen.

1. **Only in safe apps.** It presses Enter in terminals and code editors you trust. In chat apps, email and web browsers it only types, so a half-finished message is never sent to a person.
2. **Never risky commands.** If the text looks dangerous (deleting files with `rm -rf`, `git push --force` and similar), it only types it and shows a warning. You press Enter yourself if you really mean it.
3. **Never the wrong window.** If you switch to a different app while it is counting down, it stops instead of typing into the wrong place.
4. **Always a way out.** Every send can be cancelled. And there is always a keyboard shortcut (Option + Space on Mac, Alt + Space on Windows) if you do not want to use the camera at all.

## What is kept, and what is not

| Thing | Kept? |
| --- | --- |
| Camera pictures | **Never.** Thrown away a split second after they are looked at. |
| Your voice recordings | **No.** Deleted right after they become text. (Unless you switch on "Save recordings" in the Lab to help improve saywhen.) |
| The text you said | **7 days**, on your computer only, so you can search and copy it. You can turn this off or clear it. |
| Anything sent to the internet | **Nothing**, unless you switch on the optional AI clean-up and point it somewhere yourself. |
