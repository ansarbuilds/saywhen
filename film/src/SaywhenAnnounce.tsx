import React from "react";
import {
  AbsoluteFill,
  Easing,
  Img,
  interpolate,
  staticFile,
  useCurrentFrame,
} from "remotion";
import { loadFont } from "@remotion/fonts";
import instrumentSerif400 from "@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2";
import plusJakarta500 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-500-normal.woff2";

// Bundled from @fontsource. loadFont waits before the first frame is drawn.
loadFont({
  family: "Instrument Serif",
  url: instrumentSerif400,
  weight: "400",
  style: "normal",
});
loadFont({
  family: "Plus Jakarta Sans",
  url: plusJakarta500,
  weight: "500",
  style: "normal",
});

// Palette from the app. Plum black, the deeper plum, amber, silver.
const PLUM = "#22061c";
const PLUM_DEEP = "#330a2b";
const AMBER = "#fbb303";
const SILVER = "#dfe5ec";

const WIDTH = 1920;
const HEIGHT = 1080;
const CX = WIDTH / 2;
const CY = HEIGHT / 2;

// 2.2 seconds each, at 30 fps.
const LOOK = 66;
const TALK = 132;
const SENT = 198;
const LOGO = 264;
const BEAT = 66;

const WORD_SIZE = 188;
const WORD_IN = 18;
const WORD_OUT = 18;
const LIGHT = 10;
// The point sits just above the word, which is itself at the center.
const LIGHT_Y = CY - WORD_SIZE / 2 - 22 - LIGHT / 2;

const ease = Easing.inOut(Easing.cubic);

const mix = (
  frame: number,
  inputStart: number,
  inputEnd: number,
  outputStart: number,
  outputEnd: number,
) =>
  interpolate(frame, [inputStart, inputEnd], [outputStart, outputEnd], {
    easing: ease,
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

// Fade in, hold, fade out. The rise happens only on the way in.
const wordMotion = (frame: number, start: number) => {
  const end = start + BEAT;
  const opacity =
    mix(frame, start, start + WORD_IN, 0, 1) *
    mix(frame, end - WORD_OUT, end, 1, 0);
  const y = mix(frame, start, start + WORD_IN, 12, 0);
  return { opacity, y };
};

// One breath: fade up to 0.45, rise to 0.9, return to 0.45, then rest.
const lightOpacity = (frame: number) => {
  if (frame < 16) return mix(frame, 0, 16, 0, 0.45);
  if (frame < 41) return mix(frame, 16, 41, 0.45, 0.9);
  if (frame < LOOK) return mix(frame, 41, LOOK, 0.9, 0.45);
  if (frame < LOOK + 48) return 0.45;
  return mix(frame, LOOK + 48, TALK, 0.45, 0);
};

const Word: React.FC<{ text: string; opacity: number; y: number }> = ({
  text,
  opacity,
  y,
}) => (
  <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
    <div
      style={{
        fontFamily: "Instrument Serif",
        fontStyle: "normal",
        fontWeight: 400,
        fontSize: WORD_SIZE,
        lineHeight: 1,
        letterSpacing: "-0.04em",
        color: AMBER,
        opacity,
        transform: `translateY(${y}px)`,
        whiteSpace: "nowrap",
        fontSynthesis: "none",
        WebkitFontSmoothing: "antialiased",
      }}
    >
      {text}
    </div>
  </AbsoluteFill>
);

// A single thin ring. Radius eases out to a stop. Opacity eases away.
const Ring: React.FC<{
  frame: number;
  start: number;
  grow: number;
  life: number;
  radiusTo: number;
  peak: number;
  stroke: number;
}> = ({ frame, start, grow, life, radiusTo, peak, stroke }) => {
  const local = frame - start;
  if (local < 0 || local > life) return null;
  const radius = mix(frame, start, start + grow, 2, radiusTo);
  const riseEnd = start + Math.round(life * 0.22);
  const opacity =
    mix(frame, start, riseEnd, 0, peak) * mix(frame, riseEnd, start + life, 1, 0);
  if (opacity <= 0.004) return null;
  return (
    <svg
      width={WIDTH}
      height={HEIGHT}
      style={{ position: "absolute", inset: 0 }}
    >
      <circle
        cx={CX}
        cy={CY}
        r={radius}
        fill="none"
        stroke={AMBER}
        strokeWidth={stroke}
        opacity={opacity}
      />
    </svg>
  );
};

export const SaywhenAnnounce: React.FC = () => {
  const frame = useCurrentFrame();

  const look = wordMotion(frame, LOOK);
  const talk = wordMotion(frame, TALK);
  const sent = wordMotion(frame, SENT);

  const point = lightOpacity(frame);

  // Logo and name arrive together, then hold through the last frame.
  const logoOpacity = mix(frame, LOGO, LOGO + 32, 0, 1);
  const logoScale = mix(frame, LOGO, LOGO + 32, 0.96, 1);

  return (
    <AbsoluteFill style={{ backgroundColor: PLUM }}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(ellipse 130% 110% at 50% 46%, ${PLUM_DEEP}38 0%, ${PLUM}00 74%)`,
        }}
      />

      {frame < TALK && point > 0.004 ? (
        <div
          style={{
            position: "absolute",
            left: CX - LIGHT / 2,
            top: LIGHT_Y - LIGHT / 2,
            width: LIGHT,
            height: LIGHT,
            borderRadius: "50%",
            backgroundColor: AMBER,
            opacity: point,
            boxShadow:
              "0 0 10px 3px rgba(251, 179, 3, 0.55), 0 0 22px 8px rgba(251, 179, 3, 0.16)",
          }}
        />
      ) : null}

      <Ring
        frame={frame}
        start={TALK}
        grow={14}
        life={32}
        radiusTo={300}
        peak={0.55}
        stroke={1.5}
      />
      <Ring
        frame={frame}
        start={SENT}
        grow={16}
        life={36}
        radiusTo={230}
        peak={0.88}
        stroke={1.75}
      />

      {frame >= LOOK && frame < TALK ? (
        <Word text="Look." opacity={look.opacity} y={look.y} />
      ) : null}
      {frame >= TALK && frame < SENT ? (
        <Word text="Talk." opacity={talk.opacity} y={talk.y} />
      ) : null}
      {frame >= SENT && frame < LOGO ? (
        <Word text="Sent." opacity={sent.opacity} y={sent.y} />
      ) : null}

      {frame >= LOGO ? (
        <AbsoluteFill
          style={{
            alignItems: "center",
            justifyContent: "center",
            flexDirection: "column",
          }}
        >
          <Img
            src={staticFile("logo.png")}
            style={{
              width: 180,
              height: 180,
              opacity: logoOpacity,
              transform: `scale(${logoScale})`,
            }}
          />
          <div
            style={{
              marginTop: 28,
              fontFamily: "Plus Jakarta Sans",
              fontStyle: "normal",
              fontWeight: 500,
              fontSize: 28,
              lineHeight: 1,
              letterSpacing: "-0.02em",
              color: SILVER,
              opacity: logoOpacity,
              fontSynthesis: "none",
              WebkitFontSmoothing: "antialiased",
            }}
          >
            saywhen
          </div>
        </AbsoluteFill>
      ) : null}
    </AbsoluteFill>
  );
};
