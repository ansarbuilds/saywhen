# Vision

## The problem

Talking to a coding agent or a terminal should be as easy as talking to a colleague sitting next to you. Today it is not.

1. **Hotkey friction.** Every dictation tool wants you to press and hold (or toggle) a key. Your hands have to come back to the keyboard just to start talking, which defeats the point of voice.
2. **Review-then-send friction.** Most tools paste the text and stop. You read it, then reach for Enter. For short agent prompts and shell commands, that second step is most of the effort.
3. **Audio-only VAD misfires.** Voice activity detection on audio alone cannot tell the difference between you talking to the computer, you talking to someone in the room, a podcast, or a call. So tools either need a hotkey or they trigger on everything.

## The idea

Use the webcam as a second sensor for intent. If you are in front of the screen, facing it, your lips are moving and the mic hears speech, you are very likely talking to the computer. Combine those signals and you can skip the hotkey entirely.

## Trigger (start listening)

All of these must hold at the same time:

- **Face present** in the camera frame.
- **Facing the screen**: head yaw and pitch within a calibrated range.
- **Lips moving**: mouth aspect ratio (MAR) shows variance over a sliding window of about 300ms. A static open mouth (yawn) or slow rhythmic motion (chewing) should not count.
- **Speech**: audio VAD (Silero) reports speech in the same window.

Audio is buffered in a short ring buffer, so the first syllable is not lost while the visual gate decides.

## Stop (end of utterance)

Any of:

- **Silence and closed mouth** for about 500 to 800ms (tunable).
- **End phrase**, for example "send it", which is stripped from the final text.
- **Gesture**: nod = send now, head shake = cancel.

## User flow

1. saywhen runs in the background. A small indicator shows the current state.
2. You look at your terminal or agent and start talking. The indicator switches to "listening".
3. You stop talking. Audio is transcribed locally.
4. The AI cleanup pass tidies the text (filler words, punctuation, code identifiers kept intact).
5. A short preview appears near the cursor with a countdown. You can cancel by shaking your head, saying "cancel", pressing Esc, or looking away.
6. If not canceled, the text is typed into the focused app and Enter is pressed.

## The 7-second demo GIF

The demo that sells it, with no narration needed:

1. Hands visibly off the keyboard.
2. The user looks at a terminal running Claude Code and says: "run the tests and fix any failures".
3. The preview pops up with the cleaned text.
4. The user nods.
5. The prompt is submitted and the agent starts running the tests.

Seven seconds, zero keys pressed.
