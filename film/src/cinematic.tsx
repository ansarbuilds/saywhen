import React from "react";
import { Audio, Easing, Sequence, interpolate, staticFile } from "remotion";

const ease = Easing.inOut(Easing.cubic);

const clamp = {
  extrapolateLeft: "clamp",
  extrapolateRight: "clamp",
} as const;

// Ava neural reads. Frame counts are the mp3 lengths at 30 fps.
export const VOICE_CUES = [
  { id: "01", start: 14, frames: 92, text: "Voice input. Without a hotkey." },
  { id: "02", start: 118, frames: 91, text: "Look. Talk. Sent." },
  {
    id: "03",
    start: 230,
    frames: 67,
    text: "It can see you looking at the screen.",
  },
  {
    id: "04",
    start: 318,
    frames: 101,
    text: "So you just talk. No button to hold.",
  },
  {
    id: "05",
    start: 500,
    frames: 99,
    text: "It writes the real command, and lets you check.",
  },
  {
    id: "06",
    start: 604,
    frames: 87,
    text: "Then it types it in, and presses enter.",
  },
  {
    id: "07",
    start: 720,
    frames: 119,
    text: "You speak like a person. It writes like a developer.",
  },
  { id: "08", start: 990, frames: 109, text: "Pause. Nod. Or say send it." },
  { id: "09", start: 1132, frames: 64, text: "And you can always stop it." },
  {
    id: "10",
    start: 1288,
    frames: 92,
    text: "Risky commands never press enter for you.",
  },
  {
    id: "11",
    start: 1536,
    frames: 65,
    text: "Nothing leaves this computer.",
  },
  { id: "12", start: 1688, frames: 91, text: "Look. Talk. Sent." },
] as const;

export const FilmSubtitles: React.FC<{ frame: number }> = ({ frame }) => {
  const cue = VOICE_CUES.find(
    (item) => frame >= item.start && frame < item.start + item.frames,
  );
  if (!cue) return null;
  const local = frame - cue.start;
  const fadeIn = interpolate(local, [0, 7], [0, 1], { ...clamp, easing: ease });
  const fadeOut = interpolate(local, [cue.frames - 8, cue.frames], [1, 0], {
    ...clamp,
    easing: ease,
  });
  return (
    <div
      style={{
        position: "absolute",
        left: 160,
        right: 160,
        bottom: 28,
        display: "flex",
        justifyContent: "center",
        opacity: fadeIn * fadeOut,
        pointerEvents: "none",
      }}
    >
      <div
        style={{
          fontFamily: "Plus Jakarta Sans",
          fontWeight: 500,
          fontSize: 26,
          lineHeight: 1.2,
          letterSpacing: "-0.01em",
          color: "#f7f4f7",
          textAlign: "center",
          textShadow: "0 2px 16px rgba(18, 2, 14, 0.95)",
        }}
      >
        {cue.text}
      </div>
    </div>
  );
};

export const VoiceOver: React.FC = () => {
  return (
    <>
      {VOICE_CUES.map((cue) => (
        <Sequence key={cue.id} from={cue.start} durationInFrames={cue.frames + 4}>
          <Audio src={staticFile(`vo/${cue.id}.mp3`)} />
        </Sequence>
      ))}
    </>
  );
};
