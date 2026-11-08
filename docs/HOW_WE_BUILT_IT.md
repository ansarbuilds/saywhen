# How we built saywhen

This is the story of saywhen, told simply. Every number here was measured, and the full technical log lives in [ROADMAP.md](ROADMAP.md).

## The itch

Talking to a coding assistant should feel like talking to a colleague. But every voice tool had the same two annoyances:

1. **You had to press a key to start.** Your hands had to come back to the keyboard just to talk, which misses the point of using your voice.
2. **You had to press Enter at the end.** The tool typed your words and then waited. For a short command, that last key press is most of the work.

And tools that listened all the time without a key started recording every podcast, phone call and chat in the room.

The idea: **use the webcam as a second pair of ears.** If you are looking at the screen and your lips are moving while the microphone hears a voice, you are almost certainly talking to the computer.

## Stage 1: Can a webcam even tell? (1 November 2026)

Before building an app, we built a tiny test page to answer one question: can a normal laptop webcam tell, reliably and quickly, whether someone is facing the screen and talking?

We used Google's free MediaPipe face tracker, which finds about 470 points on a face. From those points we worked out the head angle and how open the mouth is.

What we learned:

- It is **fast**: 12 to 15 milliseconds per frame, so checking 15 times a second costs almost nothing.
- "Facing the screen or not" was separated **100%** of the time in our test sessions.
- "Mouth still" was correctly ignored **96.2%** of the time.
- But **the camera alone cannot tell chewing or smiling from talking.** That proved we needed the microphone too. Neither sensor is enough on its own; together they are.

## Stage 2: A plain voice typer (2 November 2026)

Next we built the boring but essential part: a desktop app (Tauri, with a Rust engine) that records your voice, turns it into text with Whisper on your own computer, and types it into your terminal when you press a hotkey.

Two important tricks went in here:

- **A half-second rewind buffer.** The microphone always keeps the last 0.5 seconds of sound, so the first word is never lost while the app decides you are talking.
- **Whisper on the graphics chip.** On an Apple Silicon Mac, 1.5 seconds of speech turns into text in 87 to 473 milliseconds.

## Stage 3: No more hotkey (2 November 2026)

Then we joined the camera and the microphone together. saywhen got its five moods: **Waiting, Ready, Listening, Writing, Check.** It starts listening only when your face, your lips and your voice all agree, and it stops when you go quiet. A nod finishes early, a head shake cancels.

## Stage 4: Tidy words and safe sending (2 November 2026)

Speech-to-text is bad at developer words. "cargo test dash dash release" is not what you want in a terminal. We wrote a clean-up step that turns spoken commands into real ones, and taught it the common mistakes Whisper makes ("cubectal" means `kubectl`).

We also made sending safe:

- A **countdown** before every send, so you can read and cancel.
- Enter is only pressed in **trusted apps** (terminals, editors). Chat apps and email only get typed text.
- **Risky commands** like `rm -rf` are never sent automatically.

## Stage 5: Windows and a first welcome screen (2 November 2026)

saywhen learned to run on Windows, got a welcome screen explaining permissions, and remembered your settings between launches.

## Making it really good (3 and 4 November 2026)

With the basics working, we spent two days on the hard details. A few highlights:

- **Starting and stopping at the right moment.** We recorded 56 real sentences and replayed them through the old and new logic. Starting by mistake went from **36.4% to 0%**, and cutting people off mid-sentence went from **52.9% to 0%**.
- **Developer words.** On 34 spoken commands, exact matches went from **44.1% to 100%**.
- **Typing reliably.** saywhen checks which app is in front before it pastes, waits a little longer for long text, puts your clipboard back afterwards, and refuses to type into the wrong window. Finding the front app on macOS went from about 8 milliseconds to about 0.002.
- **Real life.** Unplugging headphones, closing the laptop lid and locking the screen are all handled without sending anything by accident.
- **Spoken fixes.** "scratch that", "paste only" and "never mind" work while you talk.
- **History.** The last 7 days of what you said, searchable, on your computer only.

## Choosing how you send, and a new look (8 November 2026)

People asked for different ways to send. Some love nodding, some never want Enter pressed for them. So saywhen now has four **send styles** you pick with one click: Just pause, Nod to send, Say "send it", and Type only. You can also cancel a countdown just by talking again (saying "cancel" or "wait").

At the same time the app got a complete makeover. It now uses a warm, magazine-like design with bundled fonts that work offline, and the window gently changes colour with each mood so you can tell what saywhen is doing from across the room.

## Cameras off to the side (8 November 2026)

The first tester to try the new look had a laptop camera beside a big monitor, so the camera saw the side of their face. saywhen said "looking away" and "lips still" even while they were talking. Three fixes: calibration can now point up to 65 degrees away from the camera (it stopped at 45), a **Camera is off to the side** switch widens the allowed head turn, and the lip measurement now corrects for how a turned head makes the mouth look narrower.

## A logo and a calmer look (8 November 2026)

saywhen got its own logo: a ribbon "S" in silver and amber on deep plum. The app was redesigned around it, aiming for the calm, rounded feel of the best voice tools: soft white cards, rounded buttons, one strong colour (plum) and amber only for "live" moments like listening and the send countdown. The logo now sits in the sidebar, the welcome screen, the floating pill and the app icon, and a new **Guide** page inside the app explains what to do and what to avoid in plain words.

## What is next

- Sending text to the window you are *looking at*, not just the one in front.
- Linux support beyond the basics.
- Signed, one-click installers so nobody needs to build from source.

## The rules we kept the whole way

- **Local first.** Your voice and face never leave your computer.
- **Always cancelable.** Nothing is sent without a chance to stop it.
- **Fail loudly.** If something breaks, saywhen tells you instead of quietly doing something else.
- **Measure everything.** If we claim it is better, there is a number.
