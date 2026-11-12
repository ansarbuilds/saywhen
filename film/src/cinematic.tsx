import React from "react";
import { Audio, Easing, Sequence, interpolate, staticFile } from "remotion";

export interface VoiceCue {
  id: string;
  start: number;
  frames: number;
  text: string;
}

// Locked 60.0s (1800 frames @ 30fps) voiceover cue timeline synced to generate-audio.py
export const VOICE_CUES: VoiceCue[] = [
  { id: "01", start: 14, frames: 92, text: "Voice input. Without a hotkey." },
  { id: "02", start: 120, frames: 91, text: "Look. Talk. Sent." },
  { id: "03", start: 228, frames: 67, text: "It can see you looking at the screen." },
  { id: "04", start: 318, frames: 101, text: "So you just talk. No button to hold." },
  { id: "05", start: 498, frames: 99, text: "It writes the real command, and lets you check." },
  { id: "06", start: 604, frames: 87, text: "Then it types it in, and presses enter." },
  { id: "07", start: 718, frames: 119, text: "You speak like a person. It writes like a developer." },
  { id: "08", start: 916, frames: 98, text: "Pause. Nod. Say send it. Or type only." },
  { id: "09", start: 1130, frames: 64, text: "And you can always stop it." },
  { id: "10", start: 1284, frames: 92, text: "Risky commands never press enter for you." },
  { id: "11", start: 1498, frames: 65, text: "Nothing leaves this computer." },
  { id: "12", start: 1682, frames: 91, text: "Look. Talk. Sent." },
];

export const FilmSubtitles: React.FC<{ frame: number }> = ({ frame }) => {
  const active = VOICE_CUES.find(
    (c) => frame >= c.start - 4 && frame <= c.start + c.frames + 10
  );
  if (!active) return null;

  const enterOpacity = interpolate(frame, [active.start - 4, active.start + 5], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const exitOpacity = interpolate(
    frame,
    [active.start + active.frames - 2, active.start + active.frames + 10],
    [1, 0],
    {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    }
  );
  const y = interpolate(frame, [active.start - 4, active.start + 6], [10, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });

  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        bottom: 34,
        display: "flex",
        justifyContent: "center",
        pointerEvents: "none",
        opacity: Math.min(enterOpacity, exitOpacity),
        transform: `translateY(${y}px)`,
        zIndex: 80,
      }}
    >
      <div
        style={{
          padding: "10px 24px",
          borderRadius: 999,
          background: "rgba(14, 4, 14, 0.86)",
          border: "1px solid rgba(251, 179, 3, 0.28)",
          boxShadow: "0 14px 36px rgba(0, 0, 0, 0.48), 0 0 0 1px rgba(255,255,255,0.06) inset",
          backdropFilter: "blur(18px)",
          WebkitBackdropFilter: "blur(18px)",
          color: "#faf7f4",
          fontFamily: "Plus Jakarta Sans",
          fontSize: 22,
          fontWeight: 600,
          letterSpacing: "-0.01em",
          lineHeight: 1.25,
          textAlign: "center",
        }}
      >
        {active.text}
      </div>
    </div>
  );
};

export const VoiceOver: React.FC = () => {
  return (
    <>
      {VOICE_CUES.map((cue) => (
        <Sequence key={cue.id} from={cue.start} durationInFrames={cue.frames + 8}>
          <Audio src={staticFile(`vo/${cue.id}.mp3`)} />
        </Sequence>
      ))}
    </>
  );
};
