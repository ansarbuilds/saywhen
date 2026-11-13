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
  { id: "01", start: 46, frames: 66, text: "Voice input. Without a hotkey." },
  { id: "02", start: 140, frames: 72, text: "Look. Talk. Sent." },
  { id: "03", start: 244, frames: 50, text: "It can see you looking at the screen." },
  { id: "04", start: 364, frames: 81, text: "So you just talk. No button to hold." },
  { id: "05", start: 512, frames: 81, text: "It writes the real command, and lets you check." },
  { id: "06", start: 628, frames: 67, text: "Then it types it in, and presses Enter." },
  { id: "07", start: 748, frames: 101, text: "You speak like a person. It writes like a developer." },
  { id: "08", start: 980, frames: 90, text: "Pause. Nod. Or say send it." },
  { id: "09", start: 1142, frames: 45, text: "And you can always stop it." },
  { id: "10", start: 1292, frames: 74, text: "Risky commands never press Enter for you." },
  { id: "11", start: 1558, frames: 46, text: "Nothing leaves this computer." },
  { id: "12", start: 1673, frames: 72, text: "Look. Talk. Sent." },
];

export const FilmSubtitles: React.FC<{ frame: number }> = ({ frame }) => {
  const active = VOICE_CUES.find(
    (c) => frame >= c.start - 3 && frame <= c.start + c.frames + 8
  );
  // Beats 2 and 12 already feature large centered "Look. Talk. Sent." typography
  if (!active || active.id === "02" || active.id === "12") return null;

  const enterOpacity = interpolate(frame, [active.start - 3, active.start + 5], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const exitOpacity = interpolate(
    frame,
    [active.start + active.frames - 2, active.start + active.frames + 8],
    [1, 0],
    {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    }
  );
  const y = interpolate(frame, [active.start - 3, active.start + 5], [8, 0], {
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
        bottom: 24,
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
          padding: "8px 20px",
          borderRadius: 999,
          background: "rgba(26, 6, 23, 0.84)",
          border: "1px solid rgba(251, 179, 3, 0.24)",
          boxShadow: "0 10px 28px rgba(0, 0, 0, 0.36), 0 0 0 1px rgba(255,255,255,0.06) inset",
          backdropFilter: "blur(16px)",
          WebkitBackdropFilter: "blur(16px)",
          color: "#faf7f4",
          fontFamily: "Plus Jakarta Sans",
          fontSize: 19,
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
