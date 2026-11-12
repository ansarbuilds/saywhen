# When something goes wrong

Start with the symptom. Each answer says what to check first and what to send if it still fails.

## Nothing gets typed

1. Check the app in front. saywhen types into the focused app, not into a window that is behind it.
2. On a Mac, open **System Settings > Privacy & Security > Accessibility** and allow **saywhen**, or **saywhen-dev** if you built it yourself. Remove old saywhen entries first if macOS shows more than one.
3. Look at **Problems** on Home. If the message mentions Accessibility, click **Open Accessibility settings**.
4. Try **Type only** on Home. If the text appears but Enter is not pressed, the problem is the app rule, not the microphone.
5. If the target app is running as administrator on Windows, Windows blocks input from a non-elevated app. Run saywhen the same way as the target, or use the app without elevation.

## It never starts listening

1. Turn the camera on. The pill should say **Ready** when you look at the screen.
2. Click **Calibrate** while looking at the screen you actually talk to, with your mouth closed. Calibration teaches saywhen what "facing the screen" means for your desk.
3. Check the camera preview. If it is black or frozen, see **The camera sees me but it never says Ready** below.
4. Try **Option + Space** on a Mac or **Alt + Space** on Windows. If that works, the audio path is fine and the problem is the camera gate.
5. If you use a laptop camera next to a larger monitor, turn on **Camera is off to the side** in Settings and calibrate while looking at the larger screen.

## The camera sees me but it never says Ready

- Make sure your face is inside the frame and the room is not backlit. A bright window behind you can hide your face.
- Look directly at the screen you want to talk to. Looking at a second monitor while the camera expects the first one is the most common cause.
- Click **Calibrate** again. Do not talk or smile during the 1.5 seconds.
- If you wear glasses, try calibrating with them on and without them. Pick the one you use most.
- If the app says **I can see you, but you look away**, the calibration is wrong for this position. Calibrate again, or turn on side-camera mode.
- If the camera is off to the side, the useful range is about 65 to 70 degrees of head turn. Past that, use the hotkey.

## It starts when I am not talking to it

- Turn up **Settings > Noise filter**.
- Pick **Nod to send**. Nothing is sent until you nod, even if it starts listening.
- If another person or a podcast is triggering it, the camera and microphone should both agree. Check that your lips are not in view while the other voice is speaking.
- Say **"cancel"** or press **Esc** during the check step. Talking again during the countdown also cancels.
- If it starts from a cough or a keyboard click, that is a false start. Open **Problems**, or use **Flag False Trigger** on the recent item, so the session log records it.

## It typed but did not press Enter

1. The app may not be on the trusted list. Open **Settings > Apps where Enter can be pressed** and add it.
2. The command may have looked risky. `rm -rf`, `git reset --hard`, `git push --force` and similar commands are only typed, with a warning, so you can press Enter yourself.
3. The target app may have changed during the countdown. saywhen stops instead of pressing Enter in the wrong place.
4. **Type only** mode never presses Enter. Choose **Just pause**, **Nod to send** or **Say "send it"** if you want automatic Enter.
5. On a Mac, Terminal's **Secure Keyboard Entry** blocks synthetic input. Turn it off while using saywhen.

## It typed into the wrong app

- Check whether you switched windows during the countdown. By default, saywhen aborts when the front app changes.
- The default is to abort. saywhen does not yet expose a setting to paste without Enter when the app changes, so do not switch windows during the countdown if you want the text delivered.
- If the wrong app is consistently detected, the frontmost app detection is the thing to report. Copy the Problems message and include the app names.

## The model download fails or stalls

1. Check your internet connection and try again. The download resumes from a temporary file only after the old partial file is removed, so a retry starts clean.
2. Make sure there is enough free disk space. Tiny is about 75 MB; Base is about 142 MB.
3. If it fails twice, try the other model. A smaller model is enough to finish setup.
4. On a managed network, the download host may be blocked. The model comes from a pinned Hugging Face revision; ask your network administrator to allow it.
5. Do not rename or edit files inside the models folder by hand. Download the model again from Settings instead.

## My microphone disappeared

- Unplug and replug the microphone or headset. saywhen rebuilds the audio stream and falls back to the default device.
- Open **Settings > Microphone** and pick the device again. The list refreshes when devices change.
- If the microphone is a Bluetooth headset, check that it is connected and not in a call that has taken exclusive control.
- On macOS, check **System Settings > Privacy & Security > Microphone**.
- On Windows, check **Settings > Privacy & security > Microphone** and the sound control panel.

## The camera stopped after I closed the window

Closing the main window keeps saywhen running in the menu bar or tray. The camera gate should keep working. If the pill says **Camera gate stalled**, open saywhen from the tray, turn the camera off and on, and check the camera permission. On a laptop, closing the lid or locking the screen also pauses the gate until you wake it.

## The installer says the app is damaged or cannot be opened

This beta is not signed by Apple yet.

```bash
xattr -cr /Applications/saywhen.app
```

Then open the app again. If you built the app yourself, use the `saywhen-dev.app` name and remove old saywhen entries from **System Settings > Privacy & Security > Accessibility** before allowing the new build.

On Windows, SmartScreen shows **More info**, then **Run anyway**. This is the missing signature, not a virus warning.

## How to send a useful report

Open **Problems** on Home and click **Copy** on the message. Include:

- what you said, and which app was in front
- macOS or Windows version, and the saywhen version
- whether the camera and microphone were on
- the exact Problems text
- whether it happens every time or only sometimes

Open an issue at <https://github.com/ansarbuilds/saywhen/issues/new/choose>. Do not paste API keys, tokens or private transcripts.

## Still stuck?

Read the [FAQ](FAQ.md) or [how it works](HOW_IT_WORKS.md). If the problem is a permission prompt, the [install guide](../INSTALL_WITH_AI.md) walks through each one.
