import React, { useMemo } from "react";
import {
  AbsoluteFill,
  Audio,
  Easing,
  Img,
  interpolate,
  staticFile,
  useCurrentFrame,
} from "remotion";
import { loadFont } from "@remotion/fonts";
import instrumentSerif400 from "@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2";
import instrumentSerif400Italic from "@fontsource/instrument-serif/files/instrument-serif-latin-400-italic.woff2";
import plusJakarta400 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-400-normal.woff2";
import plusJakarta500 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-500-normal.woff2";
import plusJakarta600 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-600-normal.woff2";
import plusJakarta700 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-700-normal.woff2";
import plusJakarta800 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-800-normal.woff2";
import { AUDIO_PEAK, AUDIO_RMS } from "./audioData";
import {
  AcousticTerrain3DCanvas,
  BiometricFaceMesh3DCanvas,
  FibonacciVortexCanvas,
  HexShieldForcefieldCanvas,
  HorizonGrid3DCanvas,
  LightSpeedWarpCanvas,
  ParametricRibbon3DCanvas,
  ShatteringKeycaps3DCanvas,
} from "./cinemaGraphics";
import { FilmSubtitles } from "./cinematic";

loadFont({ family: "Instrument Serif", url: instrumentSerif400, weight: "400" });
loadFont({
  family: "Instrument Serif",
  url: instrumentSerif400Italic,
  weight: "400",
  style: "italic",
});
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta400, weight: "400" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta500, weight: "500" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta600, weight: "600" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta700, weight: "700" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta800, weight: "800" });
loadFont({
  family: "JetBrains Mono",
  url: staticFile("jetbrains-mono.woff2"),
  weight: "100 800",
});

// ============================================================================
// BRAND TOKENS (Synced with saywhen src/style.css + obsidian studio aesthetic)
// ============================================================================
export const C = {
  plum: "#140412",
  plumDeep: "#22061c",
  plumInk: "#1a0717",
  obsidian: "#0d040c",
  cardDark: "rgba(22, 8, 22, 0.88)",
  amber: "#fbb303",
  amberSoft: "#ffd25a",
  cyan: "#82e8ff",
  emerald: "#3ddc97",
  silver: "#dfe5ec",
  ivory: "#faf7f4",
  mutedDark: "#baa8b6",
  mutedLight: "#655162",
  danger: "#ff5c6c",
  white: "#ffffff",
  borderLight: "rgba(34, 6, 28, 0.11)",
  borderDark: "rgba(223, 229, 236, 0.14)",
  borderAmber: "rgba(251, 179, 3, 0.42)",
} as const;

export const F = {
  serif: "'Instrument Serif', Georgia, serif",
  sans: "'Plus Jakarta Sans', -apple-system, sans-serif",
  mono: "'JetBrains Mono', monospace",
} as const;

const clamp = (x: number, a: number, b: number) => Math.max(a, Math.min(b, x));
export const seg = (t: number, a: number, b: number) =>
  b <= a ? (t >= b ? 1 : 0) : clamp((t - a) / (b - a), 0, 1);

export const eio = (u: number) =>
  u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2;

const eo = (u: number) => 1 - Math.pow(1 - clamp(u, 0, 1), 3);

// Cinema Mac window frame wrapper for sharp 2x captured UI screens
const AppWindowFrame: React.FC<{
  src: string;
  width: number;
  tiltY?: number;
  tiltX?: number;
  glowColor?: string;
}> = ({ src, width, tiltY = 0, tiltX = 0, glowColor = "rgba(251, 179, 3, 0.22)" }) => {
  return (
    <div
      style={{
        width,
        borderRadius: 20,
        overflow: "hidden",
        background: "#120610",
        border: "1px solid rgba(255, 255, 255, 0.14)",
        boxShadow: `0 44px 110px rgba(0, 0, 0, 0.72), 0 0 90px ${glowColor}`,
        transform: `perspective(1800px) rotateY(${tiltY}deg) rotateX(${tiltX}deg)`,
      }}
    >
      <Img src={staticFile(src)} style={{ width: "100%", height: "auto", display: "block" }} />
    </div>
  );
};

// Studio lighting atmospheric vignette & grain-free chromatic orbs
const StudioAtmosphere: React.FC<{
  mode: "obsidian" | "warm" | "danger";
  intensity: number;
}> = ({ mode, intensity }) => {
  const glowA =
    mode === "danger"
      ? "rgba(255, 76, 98, 0.24)"
      : mode === "warm"
      ? "rgba(251, 179, 3, 0.22)"
      : "rgba(180, 74, 210, 0.24)";
  const glowB =
    mode === "danger"
      ? "rgba(251, 179, 3, 0.14)"
      : "rgba(130, 232, 255, 0.12)";
  const s = 1 + intensity * 0.22;

  return (
    <AbsoluteFill style={{ pointerEvents: "none" }}>
      <div
        style={{
          position: "absolute",
          inset: 0,
          background:
            mode === "warm"
              ? "radial-gradient(1200px 760px at 50% 42%, #290b24 0%, #140412 58%, #090208 100%)"
              : mode === "danger"
              ? "radial-gradient(1200px 760px at 50% 45%, #280814 0%, #120309 62%, #070205 100%)"
              : "radial-gradient(1200px 780px at 52% 44%, #260922 0%, #120410 60%, #090208 100%)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: "18%",
          top: "12%",
          width: 780,
          height: 620,
          borderRadius: "50%",
          background: `radial-gradient(circle, ${glowA} 0%, transparent 70%)`,
          transform: `scale(${s})`,
          filter: "blur(55px)",
        }}
      />
      <div
        style={{
          position: "absolute",
          right: "10%",
          bottom: "8%",
          width: 680,
          height: 540,
          borderRadius: "50%",
          background: `radial-gradient(circle, ${glowB} 0%, transparent 70%)`,
          filter: "blur(60px)",
        }}
      />
    </AbsoluteFill>
  );
};

// Beat visibility envelope with smooth cross-dissolve on cut boundaries
const beatEnvelope = (f: number, start: number, end: number, fadeIn = 8, fadeOut = 8) => {
  if (f < start - 2 || f > end + 2) return 0;
  const enter = fadeIn > 0 ? eo(seg(f, start, start + fadeIn)) : 1;
  const exit = fadeOut > 0 ? 1 - eio(seg(f, end - fadeOut, end)) : 1;
  return enter * exit;
};

export const SaywhenDemo: React.FC = () => {
  const f = useCurrentFrame();
  const rms = AUDIO_RMS[Math.min(f, AUDIO_RMS.length - 1)] ?? 0.08;
  const peak = AUDIO_PEAK[Math.min(f, AUDIO_PEAK.length - 1)] ?? 0.1;

  // Deterministic beat switches & local normalized progress
  const activeMode: "obsidian" | "warm" | "danger" =
    f >= 1274 && f < 1484
      ? "danger"
      : (f >= 118 && f < 224) || f >= 1668
      ? "warm"
      : "obsidian";

  // BEAT 1 (0..118): Option + Space shatter -> "Without a hotkey."
  const b1Alpha = beatEnvelope(f, 0, 118, 1, 8);
  // BEAT 2 (118..224): Look. Talk. Sent.
  const b2Alpha = beatEnvelope(f, 118, 224, 8, 8);
  // BEAT 3 (224..314): Biometric Vision Gate + ui-dark-ready
  const b3Alpha = beatEnvelope(f, 224, 314, 8, 8);
  // BEAT 4 (314..492): Acoustic Terrain + Listening HUD + Live Transcript
  const b4Alpha = beatEnvelope(f, 314, 492, 8, 8);
  // BEAT 5 (492..600): Rack Focus ui-dark-preview + hud-preview macro
  const b5Alpha = beatEnvelope(f, 492, 600, 8, 8);
  // BEAT 6 (600..714): Hyperspace + Terminal 47 passed
  const b6Alpha = beatEnvelope(f, 600, 714, 8, 8);
  // BEAT 7 (714..904): Dev Cleanup Compiler + Diagnostics/Light UI Showcase
  const b7Alpha = beatEnvelope(f, 714, 904, 8, 8);
  // BEAT 8 (904..1124): Four Hands-Free Send Modes
  const b8Alpha = beatEnvelope(f, 904, 1124, 8, 8);
  // BEAT 9 (1124..1274): Always Cancelable Shield
  const b9Alpha = beatEnvelope(f, 1124, 1274, 8, 8);
  // BEAT 10 (1274..1484): Destructive Command Guard Vault
  const b10Alpha = beatEnvelope(f, 1274, 1484, 8, 8);
  // BEAT 11 (1484..1668): Local Privacy & Speed Specs
  const b11Alpha = beatEnvelope(f, 1484, 1668, 8, 8);
  // BEAT 12 (1668..1800): Golden Brand Finale Lockup
  const b12Alpha = beatEnvelope(f, 1668, 1800, 10, 0);

  // Clean memoized slot ticker data for Beat 11
  const archSpecs = useMemo(
    () => [
      { label: "Whisper Metal GPU", sub: "118 ms transcription", accent: C.amber },
      { label: "Silero VAD + 15 FPS Gaze", sub: "On-device attention gate", accent: C.cyan },
      { label: "Cloud Off By Default", sub: "0 telemetry · 0 frame uploads", accent: C.ivory },
      { label: "47 Rust Tests Passing", sub: "Cursor · Ghostty · Claude Code", accent: C.emerald },
    ],
    []
  );

  // Pre-compute Beat 6 terminal reveal
  const b6CmdChars = Math.floor(interpolate(f, [606, 638], [0, 31], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }));
  const b6Cmd = "cargo test --release --all".slice(0, b6CmdChars);

  return (
    <AbsoluteFill style={{ background: C.obsidian, overflow: "hidden", fontFamily: F.sans }}>
      <Audio src={staticFile("demo-soundtrack.wav")} />
      <StudioAtmosphere mode={activeMode} intensity={rms + peak * 0.4} />

      {/* ===================================================================== */}
      {/* BEAT 1 (0..118): Anodized Keycaps Shatter -> "Without a hotkey."      */}
      {/* ===================================================================== */}
      {b1Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b1Alpha }}>
          <ShatteringKeycaps3DCanvas f={f} explodeFrame={26} />

          {/* Pre-shatter sleek Option + Space cluster (frames 0..27) */}
          {f < 32 && (
            <div
              style={{
                position: "absolute",
                inset: 0,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 22,
                opacity: 1 - eo(seg(f, 22, 31)),
                transform: `scale(${interpolate(f, [0, 26], [0.92, 1.06], { extrapolateRight: "clamp" })})`,
              }}
            >
              {[
                { sym: "⌥", sub: "option", w: 126 },
                { sym: "space", sub: "hold to dictate", w: 290 },
              ].map((k, idx) => (
                <div
                  key={idx}
                  style={{
                    width: k.w,
                    height: 112,
                    borderRadius: 18,
                    background: "linear-gradient(160deg, rgba(48, 22, 46, 0.94), rgba(16, 6, 16, 0.96))",
                    border: "1px solid rgba(251, 179, 3, 0.45)",
                    boxShadow: "0 28px 65px rgba(0, 0, 0, 0.65), 0 0 36px rgba(251, 179, 3, 0.16)",
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    justifyContent: "center",
                    gap: 4,
                  }}
                >
                  <span style={{ color: C.ivory, fontSize: 30, fontWeight: 700, fontFamily: F.mono }}>
                    {k.sym}
                  </span>
                  <span style={{ color: C.mutedDark, fontSize: 12, letterSpacing: "0.08em", textTransform: "uppercase" }}>
                    {k.sub}
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* Post-shatter bold hero typography (frames 28..118) */}
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 18,
              opacity: eo(seg(f, 28, 42)),
              transform: `scale(${interpolate(f, [28, 118], [0.94, 1.03], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })})`,
            }}
          >
            <div
              style={{
                fontFamily: F.mono,
                fontSize: 16,
                letterSpacing: "0.24em",
                color: C.amber,
                textTransform: "uppercase",
              }}
            >
              INTRODUCING SAYWHEN
            </div>
            <div
              style={{
                fontFamily: F.serif,
                fontStyle: "italic",
                fontSize: 146,
                lineHeight: 0.96,
                color: C.ivory,
                letterSpacing: "-0.02em",
                textShadow: "0 18px 60px rgba(0,0,0,0.65)",
              }}
            >
              No hotkey.
            </div>
            <div
              style={{
                fontFamily: F.sans,
                fontSize: 28,
                fontWeight: 500,
                color: C.mutedDark,
                opacity: eo(seg(f, 44, 62)),
              }}
            >
              Look at the screen. Start talking.
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 2 (118..224): Look. Talk. Sent. + Live Check Pill               */}
      {/* ===================================================================== */}
      {b2Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b2Alpha }}>
          <ParametricRibbon3DCanvas f={f} progress={seg(f, 118, 200)} color={C.amber} />
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 34,
            }}
          >
            <div style={{ display: "flex", alignItems: "baseline", gap: 28 }}>
              {[
                { word: "Look.", at: 120, color: C.ivory },
                { word: "Talk.", at: 140, color: C.ivory },
                { word: "Sent.", at: 162, color: C.amber },
              ].map((w, idx) => {
                const u = eo(seg(f, w.at, w.at + 14));
                return (
                  <span
                    key={idx}
                    style={{
                      fontFamily: F.serif,
                      fontStyle: "italic",
                      fontSize: 138,
                      lineHeight: 1,
                      color: w.color,
                      opacity: u,
                      transform: `translateY(${(1 - u) * 28}px)`,
                      textShadow: w.color === C.amber ? "0 12px 44px rgba(251, 179, 3, 0.35)" : "0 16px 40px rgba(0,0,0,0.6)",
                    }}
                  >
                    {w.word}
                  </span>
                );
              })}
            </div>

            {/* Sleek floating status lock pill */}
            <div
              style={{
                opacity: eo(seg(f, 152, 170)),
                transform: `translateY(${(1 - eo(seg(f, 152, 170))) * 20}px)`,
                display: "flex",
                alignItems: "center",
                gap: 14,
                padding: "13px 26px",
                borderRadius: 999,
                background: "rgba(24, 8, 22, 0.94)",
                border: `1.5px solid ${C.borderAmber}`,
                boxShadow: "0 20px 52px rgba(0,0,0,0.62), 0 0 28px rgba(251,179,3,0.22)",
              }}
            >
              <Img src={staticFile("logo.png")} style={{ width: 24, height: 24, borderRadius: 6 }} />
              <span style={{ width: 9, height: 9, borderRadius: "50%", background: C.emerald, boxShadow: `0 0 10px ${C.emerald}` }} />
              <span style={{ fontSize: 17, fontWeight: 700, color: C.amber }}>Sent to Cursor</span>
              <span style={{ fontSize: 16, color: C.silver, fontFamily: F.mono }}>· 0.9s hands-free</span>
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 3 (224..314): Biometric Face Lock + Real App Ready UI            */}
      {/* ===================================================================== */}
      {b3Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b3Alpha }}>
          <BiometricFaceMesh3DCanvas f={f} cx={440} cy={418} scale={0.96} isLocked />

          {/* Clean biometric telemetry badges below sculpted reticle */}
          <div
            style={{
              position: "absolute",
              left: 130,
              top: 740,
              width: 620,
              display: "grid",
              gridTemplateColumns: "repeat(3, 1fr)",
              gap: 12,
              opacity: eo(seg(f, 236, 256)),
            }}
          >
            {[
              { k: "HEAD YAW", v: "-1.4°", sub: "Facing display", c: C.cyan },
              { k: "LIPS", v: "MOVING", sub: "Visual gate open", c: C.amber },
              { k: "PRE-ROLL", v: "500 ms", sub: "First syllable kept", c: C.emerald },
            ].map((m, i) => (
              <div
                key={i}
                style={{
                  padding: "14px 16px",
                  borderRadius: 14,
                  background: "rgba(20, 7, 20, 0.88)",
                  border: "1px solid rgba(255, 255, 255, 0.13)",
                }}
              >
                <div style={{ fontSize: 11, fontFamily: F.mono, letterSpacing: "0.1em", color: C.mutedDark }}>
                  {m.k}
                </div>
                <div style={{ fontSize: 23, fontWeight: 800, color: m.c, marginTop: 4, fontFamily: F.mono }}>
                  {m.v}
                </div>
                <div style={{ fontSize: 12, color: C.silver, marginTop: 2 }}>{m.sub}</div>
              </div>
            ))}
          </div>

          {/* Crisp 2x Retina Obsidian Ready UI positioned cleanly on right */}
          <div
            style={{
              position: "absolute",
              right: 84,
              top: 108,
              opacity: eo(seg(f, 230, 248)),
              transform: `translateX(${(1 - eo(seg(f, 230, 252))) * 60}px)`,
            }}
          >
            <div
              style={{
                fontFamily: F.mono,
                fontSize: 13,
                letterSpacing: "0.14em",
                color: C.cyan,
                marginBottom: 14,
                textTransform: "uppercase",
              }}
            >
              15 FPS ON-DEVICE FACE LANDMARKER · ZERO PICTURES SAVED
            </div>
            <AppWindowFrame
              src="screens/ui-dark-ready.png"
              width={1060}
              tiltY={-7}
              tiltX={2}
              glowColor="rgba(130, 232, 255, 0.22)"
            />
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 4 (314..492): Acoustic Terrain + Live Dictation + Floating HUD   */}
      {/* ===================================================================== */}
      {b4Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b4Alpha }}>
          <AcousticTerrain3DCanvas f={f} rms={rms} color={C.amber} />

          {/* Background tilted App Listening UI */}
          <div
            style={{
              position: "absolute",
              right: 92,
              top: 76,
              opacity: 0.42 * eo(seg(f, 320, 344)),
              transform: "scale(0.86)",
              transformOrigin: "top right",
            }}
          >
            <AppWindowFrame src="screens/ui-dark-listening.png" width={920} tiltY={-8} glowColor="rgba(251, 179, 3, 0.16)" />
          </div>

          <div
            style={{
              position: "absolute",
              left: 110,
              right: 110,
              top: 230,
              display: "flex",
              flexDirection: "column",
              gap: 24,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
              <Img
                src={staticFile("screens/hud-listening.png")}
                style={{
                  width: 470,
                  height: "auto",
                  opacity: eo(seg(f, 322, 338)),
                  transform: `translateY(${(1 - eo(seg(f, 322, 338))) * 16}px)`,
                }}
              />
            </div>

            {/* Razor-sharp full transcript card with zero edge cropping */}
            <div
              style={{
                maxWidth: 1580,
                padding: "36px 44px",
                borderRadius: 24,
                background: "rgba(18, 6, 18, 0.90)",
                border: "1px solid rgba(251, 179, 3, 0.34)",
                boxShadow: "0 36px 90px rgba(0, 0, 0, 0.65)",
                opacity: eo(seg(f, 330, 348)),
              }}
            >
              <div
                style={{
                  fontFamily: F.mono,
                  fontSize: 13,
                  letterSpacing: "0.14em",
                  color: C.mutedDark,
                  marginBottom: 14,
                  textTransform: "uppercase",
                }}
              >
                500 MS AUDIO PRE-ROLL + SILERO VAD V5 · LIVE CAPTURE
              </div>
              <div
                style={{
                  fontFamily: F.sans,
                  fontSize: 44,
                  fontWeight: 700,
                  lineHeight: 1.24,
                  color: C.ivory,
                  letterSpacing: "-0.02em",
                }}
              >
                {"run cargo test workspace release and fix the failing assertion".slice(
                  0,
                  Math.floor(
                    interpolate(f, [336, 462], [6, 60], {
                      extrapolateLeft: "clamp",
                      extrapolateRight: "clamp",
                    })
                  )
                )}
                <span
                  style={{
                    display: "inline-block",
                    width: 4,
                    height: 48,
                    background: C.amber,
                    marginLeft: 6,
                    verticalAlign: "-6px",
                    opacity: Math.sin(f * 0.45) > 0 ? 1 : 0.25,
                  }}
                />
              </div>
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 5 (492..600): Real Obsidian Preview UI + Macro Floating Check HUD */}
      {/* ===================================================================== */}
      {b5Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b5Alpha }}>
          <div
            style={{
              position: "absolute",
              left: 140,
              top: 72,
              opacity: 0.72 * eo(seg(f, 492, 510)),
              transform: `scale(${interpolate(f, [492, 600], [0.95, 1.0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })})`,
            }}
          >
            <AppWindowFrame src="screens/ui-dark-preview.png" width={1320} tiltX={3} glowColor="rgba(251, 179, 3, 0.24)" />
          </div>

          {/* Crisp Foreground Macro Floating Check Card */}
          <div
            style={{
              position: "absolute",
              right: 120,
              bottom: 112,
              opacity: eo(seg(f, 504, 524)),
              transform: `translateY(${(1 - eo(seg(f, 504, 524))) * 44}px) scale(1.04)`,
              filter: "drop-shadow(0 38px 80px rgba(0, 0, 0, 0.85))",
            }}
          >
            <Img src={staticFile("screens/hud-preview.png")} style={{ width: 880, height: "auto" }} />
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 6 (600..714): Hyperspace Delivery -> Terminal 47 Passed          */}
      {/* ===================================================================== */}
      {b6Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b6Alpha }}>
          <LightSpeedWarpCanvas f={f} speed={1.2} streakCount={80} />

          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <div
              style={{
                width: 1180,
                borderRadius: 20,
                background: "rgba(12, 4, 12, 0.94)",
                border: "1px solid rgba(251, 179, 3, 0.36)",
                boxShadow: "0 44px 120px rgba(0,0,0,0.82), 0 0 80px rgba(251,179,3,0.16)",
                overflow: "hidden",
                transform: `scale(${interpolate(f, [600, 622], [0.92, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })})`,
              }}
            >
              <div
                style={{
                  padding: "16px 24px",
                  borderBottom: "1px solid rgba(255,255,255,0.08)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  fontFamily: F.mono,
                  fontSize: 14,
                  color: C.mutedDark,
                }}
              >
                <span>ghostty · ~/Projects/saywhen</span>
                <span
                  style={{
                    padding: "4px 12px",
                    borderRadius: 999,
                    background: "rgba(251, 179, 3, 0.16)",
                    color: C.amber,
                    fontWeight: 700,
                  }}
                >
                  PASTE + AUTO-ENTER
                </span>
              </div>
              <div style={{ padding: "36px 40px", fontFamily: F.mono, fontSize: 28, lineHeight: 1.7 }}>
                <div style={{ color: C.ivory }}>
                  <span style={{ color: C.amber }}>$ </span>
                  {b6Cmd}
                  <span style={{ color: C.amber, opacity: f < 645 ? 1 : 0 }}>_</span>
                </div>
                {f >= 646 && (
                  <div style={{ marginTop: 18, fontSize: 22, color: C.mutedDark, opacity: eo(seg(f, 646, 658)) }}>
                    Compiling saywhen v0.2.0 (/Users/dev/saywhen/src-tauri)
                  </div>
                )}
                {f >= 662 && (
                  <div style={{ fontSize: 22, color: C.cyan, opacity: eo(seg(f, 662, 672)) }}>
                    Finished `release` profile [optimized] in 1.12s
                  </div>
                )}
                {f >= 674 && (
                  <div
                    style={{
                      marginTop: 10,
                      fontSize: 25,
                      fontWeight: 700,
                      color: C.emerald,
                      opacity: eo(seg(f, 674, 684)),
                    }}
                  >
                    test result: ok. 47 passed; 0 failed; 3 ignored
                  </div>
                )}
              </div>
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 7 (714..904): Dev Cleanup Compiler + Dark Diagnostics / Light UI */}
      {/* ===================================================================== */}
      {b7Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b7Alpha }}>
          {/* Sub-scene A (714..826): Spoken English -> Exact Code Compiler */}
          {f < 834 && (
            <div
              style={{
                position: "absolute",
                inset: 0,
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                gap: 26,
                opacity: 1 - eio(seg(f, 818, 834)),
              }}
            >
              <div style={{ textAlign: "center" }}>
                <div style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.18em", color: C.amber }}>
                  LOCAL DETERMINISTIC CLEANUP · CLEANUP.RS
                </div>
                <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 66, color: C.ivory, marginTop: 6 }}>
                  Speak naturally. <span style={{ color: C.amber }}>Write like a developer.</span>
                </div>
              </div>

              {[
                {
                  raw: "uh git checkout dash b feat slash voice gate and push upstream origin head",
                  clean: "git checkout -b feat/voice-gate && git push -u origin HEAD",
                  at: 718,
                },
                {
                  raw: "in source tauri source turn taking dot rs update evaluate arming hysteresis",
                  clean: "In `src-tauri/src/turn_taking.rs`, update `evaluate_arming_hysteresis`",
                  at: 738,
                },
                {
                  raw: "run claude dash p fix the race condition inside src inject dot rs",
                  clean: 'claude -p "fix the race condition inside src/inject.rs"',
                  at: 758,
                },
              ].map((row, idx) => {
                const reveal = eo(seg(f, row.at, row.at + 20));
                const compiled = eo(seg(f, row.at + 18, row.at + 34));
                return (
                  <div
                    key={idx}
                    style={{
                      width: 1240,
                      padding: "22px 32px",
                      borderRadius: 18,
                      background: "rgba(22, 8, 22, 0.90)",
                      border: `1px solid ${compiled > 0.6 ? C.borderAmber : C.borderDark}`,
                      display: "flex",
                      flexDirection: "column",
                      gap: 10,
                      opacity: reveal,
                      transform: `translateY(${(1 - reveal) * 20}px)`,
                    }}
                  >
                    <div
                      style={{
                        fontFamily: F.mono,
                        fontSize: 16,
                        color: C.mutedDark,
                        textDecoration: compiled > 0.5 ? "line-through" : "none",
                        opacity: compiled > 0.5 ? 0.55 : 0.9,
                      }}
                    >
                      HEARD · {row.raw}
                    </div>
                    <div
                      style={{
                        fontFamily: F.mono,
                        fontSize: 26,
                        fontWeight: 700,
                        color: compiled > 0.5 ? C.amberSoft : C.silver,
                        opacity: 0.35 + compiled * 0.65,
                      }}
                    >
                      {row.clean}
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* Sub-scene B (822..904): Crisp Dual UI (Dark Diagnostics + Light Home) */}
          {f >= 818 && (
            <AbsoluteFill
              style={{
                opacity: eo(seg(f, 818, 836)),
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <div style={{ position: "relative", width: 1560, height: 820 }}>
                <div
                  style={{
                    position: "absolute",
                    left: 30,
                    top: 30,
                    transform: `translateX(${(1 - eo(seg(f, 822, 848))) * -50}px)`,
                  }}
                >
                  <AppWindowFrame
                    src="screens/ui-dark-diagnostics.png"
                    width={960}
                    tiltY={6}
                    glowColor="rgba(130, 232, 255, 0.22)"
                  />
                </div>
                <div
                  style={{
                    position: "absolute",
                    right: 30,
                    bottom: 24,
                    transform: `translateX(${(1 - eo(seg(f, 828, 854))) * 50}px)`,
                  }}
                >
                  <AppWindowFrame
                    src="screens/ui-light-ready.png"
                    width={820}
                    tiltY={-7}
                    glowColor="rgba(251, 179, 3, 0.26)"
                  />
                </div>
              </div>
            </AbsoluteFill>
          )}
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 8 (904..1124): FOUR HANDS-FREE SEND MODES (Unified Obsidian)     */}
      {/* ===================================================================== */}
      {b8Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b8Alpha }}>
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 44,
              padding: "0 86px",
            }}
          >
            <div style={{ textAlign: "center" }}>
              <div style={{ fontFamily: F.mono, fontSize: 15, letterSpacing: "0.22em", color: C.amber }}>
                FOUR WAYS TO SEND
              </div>
              <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 82, color: C.ivory, marginTop: 8 }}>
                Pause. Nod. Say send it. <span style={{ color: C.amber }}>Or type only.</span>
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 24, width: "100%", maxWidth: 1680 }}>
              {[
                {
                  num: "01",
                  title: "Just pause",
                  desc: "Stops after adaptive silence (~0.6s to 1.3s), then sends after preview.",
                  pill: "Adaptive pause · Enter",
                  start: 908,
                  activeStart: 908,
                  activeEnd: 964,
                },
                {
                  num: "02",
                  title: "Nod to send",
                  desc: "Think silently as long as you want. Dip your chin once to fire Enter.",
                  pill: "Head nod +11° · Enter",
                  start: 916,
                  activeStart: 964,
                  activeEnd: 1016,
                },
                {
                  num: "03",
                  title: 'Say "send it"',
                  desc: 'Strips "send it" cleanly from the end and triggers immediate submit.',
                  pill: "Trigger stripped · Enter",
                  start: 924,
                  activeStart: 1016,
                  activeEnd: 1068,
                },
                {
                  num: "04",
                  title: "Type only",
                  desc: "Pastes clean text into your editor and never presses Enter automatically.",
                  pill: "Safest start · No Enter",
                  start: 932,
                  activeStart: 1068,
                  activeEnd: 1126,
                },
              ].map((card, idx) => {
                const enter = eo(seg(f, card.start, card.start + 18));
                const isLit = f >= card.activeStart && f < card.activeEnd;
                return (
                  <div
                    key={idx}
                    style={{
                      padding: "32px 28px",
                      borderRadius: 22,
                      background: isLit
                        ? "linear-gradient(155deg, rgba(64, 18, 56, 0.94), rgba(24, 7, 22, 0.96))"
                        : "rgba(20, 7, 20, 0.84)",
                      border: isLit ? `1.5px solid ${C.amber}` : `1px solid ${C.borderDark}`,
                      boxShadow: isLit
                        ? "0 28px 70px rgba(0, 0, 0, 0.72), 0 0 42px rgba(251, 179, 3, 0.22)"
                        : "0 18px 46px rgba(0, 0, 0, 0.45)",
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      minHeight: 330,
                      opacity: enter,
                      transform: `translateY(${(1 - enter) * 28 + (isLit ? -8 : 0)}px)`,
                    }}
                  >
                    <div>
                      <div style={{ fontFamily: F.mono, fontSize: 13, letterSpacing: "0.14em", color: isLit ? C.amber : C.mutedDark }}>
                        MODE {card.num}
                      </div>
                      <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 44, color: C.ivory, marginTop: 10, lineHeight: 1.08 }}>
                        {card.title}
                      </div>
                      <div style={{ fontSize: 18, lineHeight: 1.5, color: C.mutedDark, marginTop: 14 }}>
                        {card.desc}
                      </div>
                    </div>
                    <div
                      style={{
                        marginTop: 22,
                        padding: "10px 14px",
                        borderRadius: 12,
                        background: isLit ? "rgba(251, 179, 3, 0.16)" : "rgba(255, 255, 255, 0.04)",
                        border: isLit ? `1px solid ${C.borderAmber}` : "1px solid rgba(255,255,255,0.08)",
                        fontFamily: F.mono,
                        fontSize: 14,
                        fontWeight: 700,
                        color: isLit ? C.amberSoft : C.silver,
                      }}
                    >
                      {card.pill}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 9 (1124..1274): Always Cancelable Before Send                    */}
      {/* ===================================================================== */}
      {b9Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b9Alpha }}>
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 32,
            }}
          >
            <div style={{ textAlign: "center" }}>
              <div style={{ fontFamily: F.mono, fontSize: 15, letterSpacing: "0.2em", color: C.amber }}>
                ALWAYS CANCELABLE PREVIEW
              </div>
              <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 78, color: C.ivory, marginTop: 6 }}>
                Every send can stop before Enter.
              </div>
            </div>

            {/* Active preview interrupted cleanly by Canceled pill */}
            {f < 1188 ? (
              <div style={{ opacity: 1 - eo(seg(f, 1176, 1188)) }}>
                <Img src={staticFile("screens/hud-preview.png")} style={{ width: 840, height: "auto" }} />
              </div>
            ) : (
              <div
                style={{
                  opacity: eo(seg(f, 1184, 1198)),
                  transform: `scale(${interpolate(f, [1184, 1200], [0.92, 1.0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })})`,
                  padding: "24px 44px",
                  borderRadius: 999,
                  background: "rgba(28, 7, 16, 0.94)",
                  border: "1.5px solid rgba(255, 92, 108, 0.72)",
                  boxShadow: "0 28px 80px rgba(0, 0, 0, 0.72), 0 0 50px rgba(255, 92, 108, 0.25)",
                  display: "flex",
                  alignItems: "center",
                  gap: 18,
                }}
              >
                <span style={{ fontFamily: F.mono, fontSize: 16, padding: "6px 14px", borderRadius: 999, background: "rgba(255, 92, 108, 0.18)", color: C.danger, fontWeight: 700 }}>
                  ABORTED BEFORE INJECT
                </span>
                <span style={{ fontSize: 30, fontWeight: 700, color: C.ivory }}>
                  Canceled. Nothing typed.
                </span>
              </div>
            )}

            <div style={{ display: "flex", gap: 16, opacity: eo(seg(f, 1146, 1168)) }}>
              {["Shake head left-right", 'Say "scratch that"', "Keep talking", "Tap Esc"].map((chip, idx) => (
                <div
                  key={idx}
                  style={{
                    padding: "11px 20px",
                    borderRadius: 999,
                    background: "rgba(255, 255, 255, 0.05)",
                    border: "1px solid rgba(255, 255, 255, 0.14)",
                    fontSize: 17,
                    color: C.silver,
                    fontWeight: 600,
                  }}
                >
                  {chip}
                </div>
              ))}
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 10 (1274..1484): Dark Destructive Guard Vault                    */}
      {/* ===================================================================== */}
      {b10Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b10Alpha }}>
          <HexShieldForcefieldCanvas f={f} pulse={rms} />

          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "0 92px",
              gap: 54,
            }}
          >
            {/* Left narrative + crimson HUD plate */}
            <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 22 }}>
              <div style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.18em", color: C.danger }}>
                DESTRUCTIVE COMMAND GUARD · CHECK_DESTRUCTIVE_COMMAND()
              </div>
              <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 70, lineHeight: 1.04, color: C.ivory }}>
                Risky commands <span style={{ color: C.danger }}>never press Enter.</span>
              </div>
              <Img
                src={staticFile("screens/hud-destructive.png")}
                style={{
                  width: 720,
                  height: "auto",
                  opacity: eo(seg(f, 1296, 1316)),
                  filter: "drop-shadow(0 28px 70px rgba(255, 52, 74, 0.28))",
                }}
              />
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap", opacity: eo(seg(f, 1320, 1344)) }}>
                {["rm -rf", "git reset --hard", "git push --force", "DROP TABLE", "kubectl delete"].map((pat, i) => (
                  <span
                    key={i}
                    style={{
                      padding: "8px 14px",
                      borderRadius: 10,
                      background: "rgba(255, 92, 108, 0.14)",
                      border: "1px solid rgba(255, 92, 108, 0.42)",
                      fontFamily: F.mono,
                      fontSize: 15,
                      color: "#ffb4bd",
                    }}
                  >
                    {pat}
                  </span>
                ))}
              </div>
            </div>

            {/* Right crisp 2x Retina Dark Destructive Guard window */}
            <div
              style={{
                opacity: eo(seg(f, 1286, 1310)),
                transform: `translateX(${(1 - eo(seg(f, 1286, 1312))) * 48}px)`,
              }}
            >
              <AppWindowFrame
                src="screens/ui-dark-destructive.png"
                width={820}
                tiltY={-7}
                glowColor="rgba(255, 92, 108, 0.28)"
              />
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 11 (1484..1668): High-Contrast Local Privacy & Speed Specs       */}
      {/* ===================================================================== */}
      {b11Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b11Alpha }}>
          <HorizonGrid3DCanvas f={f} tiltX={0.46} color={C.amber} alpha={0.22} />

          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 32,
            }}
          >
            <div style={{ fontFamily: F.mono, fontSize: 15, letterSpacing: "0.22em", color: C.amber }}>
              PRIVATE ARCHITECTURE BY DEFAULT
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 620px)", gap: 20 }}>
              {archSpecs.map((spec, idx) => {
                const enter = eo(seg(f, 1492 + idx * 14, 1514 + idx * 14));
                return (
                  <div
                    key={idx}
                    style={{
                      padding: "28px 34px",
                      borderRadius: 20,
                      background: "rgba(18, 6, 18, 0.90)",
                      border: "1px solid rgba(255, 255, 255, 0.14)",
                      boxShadow: "0 24px 60px rgba(0, 0, 0, 0.55)",
                      opacity: enter,
                      transform: `translateY(${(1 - enter) * 24}px)`,
                    }}
                  >
                    <div style={{ fontSize: 34, fontWeight: 800, color: spec.accent, letterSpacing: "-0.02em" }}>
                      {spec.label}
                    </div>
                    <div style={{ fontSize: 18, color: C.silver, marginTop: 6, fontFamily: F.mono }}>
                      {spec.sub}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* ===================================================================== */}
      {/* BEAT 12 (1668..1800): Golden Fibonacci Finale Lockup                  */}
      {/* ===================================================================== */}
      {b12Alpha > 0.01 && (
        <AbsoluteFill style={{ opacity: b12Alpha }}>
          <FibonacciVortexCanvas f={f} converge={eo(seg(f, 1668, 1735))} />

          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 20,
            }}
          >
            <Img
              src={staticFile("logo.png")}
              style={{
                width: 112,
                height: 112,
                borderRadius: 26,
                boxShadow: "0 24px 68px rgba(0,0,0,0.7), 0 0 60px rgba(251, 179, 3, 0.35)",
                transform: `scale(${interpolate(eo(seg(f, 1672, 1700)), [0, 1], [0.8, 1])})`,
              }}
            />
            <div
              style={{
                fontFamily: F.sans,
                fontSize: 92,
                fontWeight: 800,
                letterSpacing: "-0.03em",
                color: C.ivory,
                lineHeight: 1,
                opacity: eo(seg(f, 1680, 1704)),
              }}
            >
              saywhen
            </div>
            <div
              style={{
                fontFamily: F.serif,
                fontStyle: "italic",
                fontSize: 52,
                color: C.amber,
                opacity: eo(seg(f, 1690, 1714)),
              }}
            >
              Look. Talk. Sent.
            </div>
            <div
              style={{
                marginTop: 12,
                padding: "12px 26px",
                borderRadius: 999,
                background: "rgba(24, 8, 22, 0.9)",
                border: `1px solid ${C.borderAmber}`,
                fontFamily: F.mono,
                fontSize: 18,
                color: C.silver,
                opacity: eo(seg(f, 1702, 1726)),
              }}
            >
              Open source · macOS & Windows · ansarbuilds.dev/p/saywhen
            </div>
          </div>
        </AbsoluteFill>
      )}

      {/* Synchronized Broadcast Subtitles */}
      <FilmSubtitles frame={f} />
    </AbsoluteFill>
  );
};
