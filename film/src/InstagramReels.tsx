import React from "react";
import {
  AbsoluteFill,
  Audio,
  Easing,
  Img,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { loadFont } from "@remotion/fonts";
import instrumentSerif400 from "@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2";
import instrumentSerif400Italic from "@fontsource/instrument-serif/files/instrument-serif-latin-400-italic.woff2";
import plusJakarta400 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-400-normal.woff2";
import plusJakarta500 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-500-normal.woff2";
import plusJakarta600 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-600-normal.woff2";
import plusJakarta700 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-700-normal.woff2";
import plusJakarta800 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-800-normal.woff2";
import { AUDIO_RMS } from "./audioData";
import {
  AcousticTerrain3DCanvas,
  BiometricFaceMesh3DCanvas,
  FibonacciVortexCanvas,
  HexShieldForcefieldCanvas,
  HorizonGrid3DCanvas,
  ParametricRibbon3DCanvas,
  ShatteringKeycaps3DCanvas,
} from "./cinemaGraphics";
import { C, F, eio, seg } from "./SaywhenDemo";

loadFont({ family: "Instrument Serif", url: instrumentSerif400, weight: "400" });
loadFont({ family: "Instrument Serif", url: instrumentSerif400Italic, weight: "400", style: "italic" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta400, weight: "400" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta500, weight: "500" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta600, weight: "600" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta700, weight: "700" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta800, weight: "800" });
loadFont({ family: "JetBrains Mono", url: staticFile("jetbrains-mono.woff2"), weight: "100 800" });

const REEL_BG =
  "radial-gradient(960px 880px at 50% 18%, rgba(108, 22, 90, 0.48), transparent 68%), radial-gradient(840px 760px at 50% 82%, rgba(251, 179, 3, 0.16), transparent 64%), linear-gradient(175deg, #1b0518 0%, #0e020c 52%, #160414 100%)";

const ease = (t: number, a: number, b: number) => eio(seg(t, a, b));

interface ReelCaptionCue {
  startFrame: number;
  endFrame: number;
  text: string;
  highlight?: string;
}

const ReelStage: React.FC<{
  badge: string;
  children: React.ReactNode;
  captions: ReelCaptionCue[];
  audioFile: string;
}> = ({ badge, children, captions, audioFile }) => {
  const frame = useCurrentFrame();
  const activeCue = captions.find((c) => frame >= c.startFrame - 3 && frame <= c.endFrame + 6);

  return (
    <AbsoluteFill
      style={{
        background: REEL_BG,
        color: C.ivory,
        fontFamily: F.sans,
        overflow: "hidden",
      }}
    >
      <Audio src={staticFile(audioFile)} />

      {/* Subtle ambient grid lines */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          backgroundImage:
            "linear-gradient(rgba(255,255,255,0.025) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.025) 1px, transparent 1px)",
          backgroundSize: "72px 72px",
          maskImage: "radial-gradient(ellipse 80% 70% at 50% 45%, black 30%, transparent 90%)",
          WebkitMaskImage: "radial-gradient(ellipse 80% 70% at 50% 45%, black 30%, transparent 90%)",
          pointerEvents: "none",
        }}
      />

      {/* Safe-Zone Header Pill */}
      <div
        style={{
          position: "absolute",
          top: 138,
          left: 68,
          right: 68,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          zIndex: 30,
        }}
      >
        <div
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 14,
            padding: "10px 20px 10px 12px",
            borderRadius: 999,
            background: "rgba(34, 8, 30, 0.86)",
            border: "1px solid rgba(255, 255, 255, 0.15)",
            boxShadow: "0 12px 32px rgba(0,0,0,0.45)",
          }}
        >
          <Img src={staticFile("logo.png")} style={{ width: 38, height: 38, borderRadius: 10 }} />
          <span style={{ fontSize: 24, fontWeight: 700, letterSpacing: "-0.02em", color: C.ivory }}>
            saywhen
          </span>
        </div>

        <div
          style={{
            padding: "10px 20px",
            borderRadius: 999,
            background: "rgba(251, 179, 3, 0.14)",
            border: "1px solid rgba(251, 179, 3, 0.42)",
            fontFamily: F.mono,
            fontSize: 16,
            fontWeight: 700,
            letterSpacing: "0.12em",
            color: C.amber,
          }}
        >
          {badge}
        </div>
      </div>

      {/* Main Content Stage inside 9:16 Safe Zone */}
      <div
        style={{
          position: "absolute",
          top: 230,
          bottom: 410,
          left: 68,
          right: 68,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
        }}
      >
        {children}
      </div>

      {/* Burned-in Dynamic Reel Subtitles (Above Instagram Bottom UI Overlay) */}
      {activeCue ? (
        <div
          style={{
            position: "absolute",
            left: 64,
            right: 64,
            bottom: 275,
            display: "flex",
            justifyContent: "center",
            zIndex: 60,
            opacity:
              interpolate(frame, [activeCue.startFrame - 3, activeCue.startFrame + 5], [0, 1], {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
              }) *
              interpolate(frame, [activeCue.endFrame - 2, activeCue.endFrame + 6], [1, 0], {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
              }),
            transform: `translateY(${interpolate(
              frame,
              [activeCue.startFrame - 3, activeCue.startFrame + 5],
              [12, 0],
              { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: Easing.out(Easing.cubic) }
            )}px)`,
          }}
        >
          <div
            style={{
              padding: "18px 34px",
              borderRadius: 999,
              background: "rgba(22, 5, 20, 0.92)",
              border: "1.5px solid rgba(251, 179, 3, 0.42)",
              boxShadow: "0 20px 48px rgba(0, 0, 0, 0.68), 0 0 28px rgba(251, 179, 3, 0.18)",
              fontFamily: F.sans,
              fontSize: 32,
              fontWeight: 700,
              letterSpacing: "-0.015em",
              color: C.ivory,
              textAlign: "center",
              lineHeight: 1.25,
            }}
          >
            {activeCue.text}{" "}
            {activeCue.highlight ? (
              <span style={{ color: C.amber }}>{activeCue.highlight}</span>
            ) : null}
          </div>
        </div>
      ) : null}
    </AbsoluteFill>
  );
};

const FinaleOverlay: React.FC<{ progress: number; localSec: number }> = ({ progress, localSec }) => {
  if (progress <= 0.01) return null;
  const lookOn = ease(localSec, 0.05, 0.4);
  const talkOn = ease(localSec, 0.65, 1.0);
  const sentOn = ease(localSec, 1.25, 1.6);

  return (
    <AbsoluteFill
      style={{
        background: REEL_BG,
        opacity: progress,
        zIndex: 80,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "0 72px",
      }}
    >
      <Img
        src={staticFile("logo.png")}
        style={{
          width: 164,
          height: 164,
          borderRadius: 38,
          boxShadow: "0 28px 72px rgba(0,0,0,0.7), 0 0 64px rgba(251,179,3,0.28)",
          marginBottom: 42,
          transform: `scale(${0.92 + 0.08 * progress})`,
        }}
      />

      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 10,
          fontFamily: F.serif,
          fontStyle: "italic",
          fontSize: 128,
          lineHeight: 0.96,
          letterSpacing: "-0.03em",
        }}
      >
        <span
          style={{
            color: C.ivory,
            opacity: lookOn,
            transform: `translateY(${(1 - lookOn) * 18}px)`,
          }}
        >
          Look.
        </span>
        <span
          style={{
            color: C.silver,
            opacity: talkOn,
            transform: `translateY(${(1 - talkOn) * 18}px)`,
          }}
        >
          Talk.
        </span>
        <span
          style={{
            color: C.amber,
            opacity: sentOn,
            transform: `translateY(${(1 - sentOn) * 18}px)`,
            textShadow: "0 0 42px rgba(251, 179, 3, 0.45)",
          }}
        >
          Sent.
        </span>
      </div>

      <div
        style={{
          marginTop: 52,
          padding: "16px 32px",
          borderRadius: 999,
          background: "rgba(255, 255, 255, 0.06)",
          border: "1px solid rgba(251, 179, 3, 0.42)",
          fontFamily: F.mono,
          fontSize: 22,
          fontWeight: 700,
          color: C.ivory,
          letterSpacing: "0.04em",
        }}
      >
        ansarbuilds.dev/p/saywhen
      </div>
      <div
        style={{
          marginTop: 18,
          fontFamily: F.mono,
          fontSize: 17,
          color: C.mutedDark,
          letterSpacing: "0.1em",
        }}
      >
        FREE & OPEN SOURCE · MAC + WINDOWS
      </div>
    </AbsoluteFill>
  );
};

// ============================================================================
// REEL 1: "Look. Talk. Sent." (354 frames = 11.8s @ 30fps, 1080x1920)
// ============================================================================
export const ReelLookTalkSent: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = frame / fps;

  // Scene progression
  // Scene A (0.0 - 2.4s): Hook - No hotkey to hold
  // Scene B (2.4 - 6.8s): Face lock + Live spoken waveform ("cargo test --release")
  // Scene C (6.8 - 9.2s): Preview countdown -> Ghostty terminal execution (47 passed)
  // Scene D (9.2 - 11.8s): Finale
  const inSceneB = ease(t, 2.25, 2.65);
  const inSceneC = ease(t, 6.7, 7.1);
  const inFinale = ease(t, 9.1, 9.5);

  const strikePct = ease(t, 0.55, 1.15) * 100;
  const spokenProgress = ease(t, 4.2, 6.2);
  const fullSpoken = "cargo test dash dash release";
  const shownSpoken = fullSpoken.slice(0, Math.floor(spokenProgress * fullSpoken.length));
  const testsDone = t >= 8.15;

  return (
    <ReelStage
      badge="HANDS-FREE VOICE"
      audioFile="reel-1-audio.wav"
      captions={[
        { startFrame: 5, endFrame: 70, text: "Voice input.", highlight: "Without a hotkey." },
        { startFrame: 74, endFrame: 124, text: "It can see you", highlight: "looking at the screen." },
        { startFrame: 126, endFrame: 207, text: "So you just talk.", highlight: "No button to hold." },
        { startFrame: 209, endFrame: 276, text: "Then it types it in,", highlight: "and presses Enter." },
      ]}
    >
      {/* SCENE A: Hook (0.0 - 2.4s) */}
      {inSceneB < 0.99 ? (
        <div
          style={{
            opacity: 1 - inSceneB,
            transform: `scale(${1 - inSceneB * 0.06})`,
            display: "flex",
            flexDirection: "column",
            gap: 36,
          }}
        >
          <div>
            <div style={{ fontFamily: F.mono, fontSize: 19, letterSpacing: "0.14em", color: C.amber, marginBottom: 16 }}>
              FOR TERMINALS & CODING AGENTS
            </div>
            <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 96, lineHeight: 0.96, color: C.ivory }}>
              Voice input.
              <br />
              <span style={{ color: C.amber }}>Without a hotkey.</span>
            </div>
          </div>

          {/* Crossed-out Hotkey Card */}
          <div
            style={{
              position: "relative",
              padding: "38px 40px",
              borderRadius: 28,
              background: "rgba(26, 8, 24, 0.92)",
              border: "1.5px solid rgba(255, 107, 94, 0.5)",
              boxShadow: "0 28px 64px rgba(0,0,0,0.65)",
              overflow: "hidden",
            }}
          >
            <div style={{ fontFamily: F.mono, fontSize: 16, color: C.danger, letterSpacing: "0.12em", marginBottom: 20 }}>
              OLD WAY · HOLD PUSH-TO-TALK KEY
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 20, position: "relative" }}>
              {["⌥ Option", "+", "Space"].map((k, i) => (
                <div
                  key={i}
                  style={{
                    padding: k === "+" ? "0 6px" : "18px 28px",
                    borderRadius: 16,
                    background: k === "+" ? "transparent" : "rgba(255,255,255,0.07)",
                    border: k === "+" ? "none" : "1px solid rgba(255,255,255,0.18)",
                    fontFamily: F.mono,
                    fontSize: 32,
                    fontWeight: 700,
                    color: C.silver,
                  }}
                >
                  {k}
                </div>
              ))}
              {/* Animated red strike bar */}
              <div
                style={{
                  position: "absolute",
                  left: 0,
                  top: "50%",
                  width: `${strikePct}%`,
                  height: 6,
                  background: C.danger,
                  boxShadow: `0 0 24px ${C.danger}`,
                  borderRadius: 99,
                  transform: "translateY(-50%) rotate(-3deg)",
                }}
              />
            </div>
          </div>

          {/* App Window Preview */}
          <div
            style={{
              borderRadius: 24,
              overflow: "hidden",
              border: "1px solid rgba(255, 255, 255, 0.16)",
              boxShadow: "0 32px 80px rgba(0, 0, 0, 0.72), 0 0 48px rgba(251, 179, 3, 0.16)",
            }}
          >
            <Img src={staticFile("screens/ui-dark-ready.png")} style={{ width: "100%", display: "block" }} />
          </div>
        </div>
      ) : null}

      {/* SCENE B: Biometric Gaze Lock + Speech Capture (2.4 - 6.8s) */}
      {inSceneB > 0.01 && inSceneC < 0.99 ? (
        <div
          style={{
            opacity: inSceneB * (1 - inSceneC),
            transform: `translateY(${(1 - inSceneB) * 24 - inSceneC * 24}px)`,
            display: "flex",
            flexDirection: "column",
            gap: 28,
          }}
        >
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 78, lineHeight: 0.98, color: C.ivory }}>
            {t < 4.15 ? (
              <>
                Locks to your <span style={{ color: C.cyan }}>gaze.</span>
              </>
            ) : (
              <>
                Speak <span style={{ color: C.amber }}>naturally.</span>
              </>
            )}
          </div>

          {/* Step 1: Gaze Sensor Card */}
          <div
            style={{
              padding: "30px 34px",
              borderRadius: 26,
              background: "rgba(20, 8, 22, 0.94)",
              border: `1.5px solid ${t < 4.15 ? C.cyan : "rgba(130, 232, 255, 0.35)"}`,
              boxShadow: "0 24px 56px rgba(0,0,0,0.6)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div>
              <div style={{ fontFamily: F.mono, fontSize: 15, letterSpacing: "0.12em", color: C.cyan, marginBottom: 10 }}>
                01 · WEBCAM ATTENTION GATE
              </div>
              <div style={{ fontSize: 32, fontWeight: 700, color: C.ivory }}>Facing screen · Ready</div>
              <div style={{ fontFamily: F.mono, fontSize: 18, color: C.mutedDark, marginTop: 8 }}>
                Yaw +1.4° · Lip motion active · 500ms pre-roll
              </div>
            </div>
            <div
              style={{
                width: 96,
                height: 96,
                borderRadius: 22,
                background: "rgba(130, 232, 255, 0.12)",
                border: `1.5px solid ${C.cyan}`,
                display: "grid",
                placeItems: "center",
              }}
            >
              <svg width="60" height="60" viewBox="0 0 60 60">
                <circle cx="30" cy="30" r="22" fill="none" stroke={C.cyan} strokeWidth="2.2" />
                <circle cx="22" cy="26" r="2.8" fill={C.cyan} />
                <circle cx="38" cy="26" r="2.8" fill={C.cyan} />
                <ellipse
                  cx="30"
                  cy="38"
                  rx="8"
                  ry={2.5 + Math.abs(Math.sin(t * 9)) * 3.5}
                  fill="none"
                  stroke={C.amber}
                  strokeWidth="2.4"
                />
              </svg>
            </div>
          </div>

          {/* Step 2: Live HUD Pill & Waveform */}
          <div
            style={{
              padding: "32px 34px",
              borderRadius: 26,
              background: "rgba(28, 8, 24, 0.96)",
              border: `1.8px solid ${C.amber}`,
              boxShadow: "0 28px 64px rgba(0,0,0,0.68), 0 0 42px rgba(251, 179, 3, 0.22)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 18 }}>
              <span style={{ fontFamily: F.mono, fontSize: 16, fontWeight: 700, color: C.amber, letterSpacing: "0.12em" }}>
                02 · LISTENING LIVE
              </span>
              <div style={{ display: "flex", alignItems: "center", gap: 6, height: 28 }}>
                {[0, 1, 2, 3, 4, 5, 6, 7, 8].map((b) => (
                  <div
                    key={b}
                    style={{
                      width: 5,
                      height: 8 + Math.abs(Math.sin(t * 11 + b * 0.8)) * 20,
                      borderRadius: 99,
                      background: C.amber,
                    }}
                  />
                ))}
              </div>
            </div>
            <div style={{ fontFamily: F.mono, fontSize: 32, fontWeight: 700, color: C.ivory, minHeight: 46 }}>
              “{shownSpoken}”
            </div>
          </div>

          {/* Real App Listening Screenshot */}
          <div
            style={{
              borderRadius: 22,
              overflow: "hidden",
              border: "1px solid rgba(255,255,255,0.16)",
              boxShadow: "0 24px 60px rgba(0,0,0,0.65)",
            }}
          >
            <Img src={staticFile("screens/ui-dark-listening.png")} style={{ width: "100%", display: "block" }} />
          </div>
        </div>
      ) : null}

      {/* SCENE C: Cancelable Preview -> Ghostty Terminal Paste & Enter (6.8 - 9.2s) */}
      {inSceneC > 0.01 ? (
        <div
          style={{
            opacity: inSceneC,
            transform: `translateY(${(1 - inSceneC) * 24}px)`,
            display: "flex",
            flexDirection: "column",
            gap: 28,
          }}
        >
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 78, lineHeight: 0.98, color: C.ivory }}>
            Cleaned & <span style={{ color: C.emerald }}>sent.</span>
          </div>

          {/* Cancelable Check Card */}
          <div
            style={{
              padding: "28px 34px",
              borderRadius: 24,
              background: "rgba(24, 9, 24, 0.95)",
              border: `1.5px solid ${C.amber}`,
              boxShadow: "0 24px 56px rgba(0,0,0,0.6)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", fontFamily: F.mono, fontSize: 16, color: C.amber, marginBottom: 12 }}>
              <span>03 · PREVIEW CHECK · AUTO-ENTER</span>
              <span>ESC TO CANCEL</span>
            </div>
            <div style={{ fontFamily: F.mono, fontSize: 36, fontWeight: 700, color: C.ivory }}>
              cargo test --release
            </div>
          </div>

          {/* Ghostty Terminal Window */}
          <div
            style={{
              padding: "32px 34px",
              borderRadius: 26,
              background: "rgba(10, 5, 12, 0.98)",
              border: `1.8px solid ${testsDone ? C.emerald : "rgba(255,255,255,0.18)"}`,
              boxShadow: "0 32px 76px rgba(0,0,0,0.75)",
              fontFamily: F.mono,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 22 }}>
              <div style={{ display: "flex", gap: 10 }}>
                <span style={{ width: 13, height: 13, borderRadius: "50%", background: "#ff5f56" }} />
                <span style={{ width: 13, height: 13, borderRadius: "50%", background: "#ffbd2e" }} />
                <span style={{ width: 13, height: 13, borderRadius: "50%", background: "#27c93f" }} />
              </div>
              <span style={{ fontSize: 16, color: C.mutedDark }}>ghostty -- ~/Projects/saywhen</span>
            </div>

            <div style={{ fontSize: 28, color: C.ivory, marginBottom: 18 }}>
              <span style={{ color: C.amber }}>$ </span>cargo test --release
            </div>

            <div style={{ fontSize: 20, color: C.silver, lineHeight: 1.6 }}>
              <div>Compiling saywhen v0.2.1 (src-tauri)</div>
              <div>Running unittests src/lib.rs</div>
              {testsDone ? (
                <div
                  style={{
                    marginTop: 14,
                    padding: "14px 20px",
                    borderRadius: 14,
                    background: "rgba(61, 220, 151, 0.14)",
                    border: `1px solid ${C.emerald}`,
                    color: C.emerald,
                    fontWeight: 700,
                    fontSize: 24,
                  }}
                >
                  ✓ test result: ok. 47 passed; 0 failed
                </div>
              ) : (
                <div style={{ color: C.amber, marginTop: 10 }}>Running 47 tests...</div>
              )}
            </div>
          </div>
        </div>
      ) : null}

      <FinaleOverlay progress={inFinale} localSec={Math.max(0, t - 9.2)} />
    </ReelStage>
  );
};

// ============================================================================
// REEL 2: "Speaks Developer" (300 frames = 10.0s @ 30fps, 1080x1920)
// ============================================================================
export const ReelSpeaksDeveloper: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = frame / fps;

  const inFinale = ease(t, 7.35, 7.75);

  const examples = [
    {
      heard: "“um cargo test dash dash release send it”",
      typed: "cargo test --release",
      tag: "FLAGS + END PHRASE STRIPPED",
      revealAt: 1.0,
      morphAt: 1.8,
    },
    {
      heard: "“cubectal get pods dash n prod”",
      typed: "kubectl get pods -n prod",
      tag: "CLI PHONETIC CORRECTION",
      revealAt: 2.7,
      morphAt: 3.5,
    },
    {
      heard: "“get stash pot && ntm run build”",
      typed: "git stash pop && npm run build",
      tag: "GIT + NPM SYNTAX CLEANUP",
      revealAt: 4.4,
      morphAt: 5.2,
    },
  ];

  return (
    <ReelStage
      badge="SPEAKS DEVELOPER"
      audioFile="reel-2-audio.wav"
      captions={[
        { startFrame: 5, endFrame: 106, text: "You speak like a person.", highlight: "It writes like a developer." },
        { startFrame: 112, endFrame: 193, text: "It writes the real command,", highlight: "and lets you check." },
      ]}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 26 }}>
        <div>
          <div style={{ fontFamily: F.mono, fontSize: 18, letterSpacing: "0.14em", color: C.amber, marginBottom: 12 }}>
            WHISPER MISHEARS · SAYWHEN FIXES IT
          </div>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 80, lineHeight: 0.96, color: C.ivory }}>
            Speak human.
            <br />
            <span style={{ color: C.amber }}>Type exact CLI.</span>
          </div>
        </div>

        {/* 3 Stacked Transformation Cards */}
        <div style={{ display: "flex", flexDirection: "column", gap: 22 }}>
          {examples.map((ex, idx) => {
            const show = ease(t, ex.revealAt - 0.3, ex.revealAt);
            const cleaned = ease(t, ex.morphAt - 0.25, ex.morphAt + 0.15) > 0.5;
            return (
              <div
                key={idx}
                style={{
                  opacity: show,
                  transform: `translateY(${(1 - show) * 24}px)`,
                  padding: "26px 32px",
                  borderRadius: 24,
                  background: cleaned
                    ? "linear-gradient(155deg, rgba(52, 16, 46, 0.96), rgba(18, 6, 18, 0.96))"
                    : "rgba(22, 8, 22, 0.92)",
                  border: cleaned ? `1.8px solid ${C.amber}` : "1px solid rgba(255,255,255,0.14)",
                  boxShadow: cleaned
                    ? "0 24px 56px rgba(0,0,0,0.65), 0 0 32px rgba(251, 179, 3, 0.18)"
                    : "0 16px 36px rgba(0,0,0,0.45)",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
                  <span style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.12em", color: cleaned ? C.amber : C.mutedDark }}>
                    0{idx + 1} · {ex.tag}
                  </span>
                  <span
                    style={{
                      padding: "4px 12px",
                      borderRadius: 999,
                      fontFamily: F.mono,
                      fontSize: 13,
                      fontWeight: 700,
                      background: cleaned ? "rgba(61, 220, 151, 0.16)" : "rgba(255,255,255,0.06)",
                      color: cleaned ? C.emerald : C.silver,
                    }}
                  >
                    {cleaned ? "✓ CLEANED" : "HEARD"}
                  </span>
                </div>

                <div
                  style={{
                    fontFamily: F.mono,
                    fontSize: 21,
                    color: C.mutedDark,
                    textDecoration: cleaned ? "line-through" : "none",
                    marginBottom: 8,
                  }}
                >
                  {ex.heard}
                </div>

                <div
                  style={{
                    fontFamily: F.mono,
                    fontSize: 33,
                    fontWeight: 700,
                    color: cleaned ? C.ivory : C.silver,
                  }}
                >
                  <span style={{ color: C.amber }}>$ </span>
                  {cleaned ? ex.typed : "..."}
                </div>
              </div>
            );
          })}
        </div>

        {/* Benchmark Footer Badge */}
        <div
          style={{
            padding: "20px 28px",
            borderRadius: 20,
            background: "rgba(61, 220, 151, 0.10)",
            border: "1px solid rgba(61, 220, 151, 0.38)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span style={{ fontFamily: F.mono, fontSize: 19, fontWeight: 700, color: C.emerald }}>
            34 / 34 spoken dev commands exact
          </span>
          <span style={{ fontFamily: F.mono, fontSize: 16, color: C.silver }}>cleanup.rs · On-device</span>
        </div>
      </div>

      <FinaleOverlay progress={inFinale} localSec={Math.max(0, t - 7.4)} />
    </ReelStage>
  );
};

// ============================================================================
// REEL 3: "Safe & Private by Design" (300 frames = 10.0s @ 30fps, 1080x1920)
// ============================================================================
export const ReelSafeAndPrivate: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = frame / fps;

  // Scene A (0.0 - 2.6s): Cancel anytime (Esc / Head shake / "scratch that")
  // Scene B (2.6 - 5.6s): Destructive command guard (rm -rf never presses Enter)
  // Scene C (5.6 - 7.4s): 100% On-device privacy (0 frames saved, 0 cloud)
  // Scene D (7.4 - 10.0s): Finale
  const inSceneB = ease(t, 2.45, 2.8);
  const inSceneC = ease(t, 5.45, 5.8);
  const inFinale = ease(t, 7.35, 7.75);

  const canceled = t >= 1.35;

  return (
    <ReelStage
      badge="SAFE & PRIVATE"
      audioFile="reel-3-audio.wav"
      captions={[
        { startFrame: 6, endFrame: 56, text: "And you can", highlight: "always stop it." },
        { startFrame: 78, endFrame: 156, text: "Risky commands", highlight: "never press Enter." },
        { startFrame: 168, endFrame: 218, text: "Nothing leaves", highlight: "this computer." },
      ]}
    >
      {/* SCENE A: Cancel Anytime (0.0 - 2.6s) */}
      {inSceneB < 0.99 ? (
        <div
          style={{
            opacity: 1 - inSceneB,
            transform: `translateY(${-inSceneB * 24}px)`,
            display: "flex",
            flexDirection: "column",
            gap: 28,
          }}
        >
          <div>
            <div style={{ fontFamily: F.mono, fontSize: 18, letterSpacing: "0.14em", color: C.amber, marginBottom: 12 }}>
              01 · CANCELABLE PREVIEW
            </div>
            <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 82, lineHeight: 0.96, color: C.ivory }}>
              You stay in <span style={{ color: C.amber }}>control.</span>
            </div>
          </div>

          <div
            style={{
              padding: "32px 36px",
              borderRadius: 26,
              background: canceled ? "rgba(42, 10, 18, 0.95)" : "rgba(24, 8, 24, 0.95)",
              border: `1.8px solid ${canceled ? C.danger : C.amber}`,
              boxShadow: "0 28px 64px rgba(0,0,0,0.68)",
            }}
          >
            {canceled ? (
              <div>
                <div style={{ fontFamily: F.mono, fontSize: 16, fontWeight: 700, color: C.danger, marginBottom: 12 }}>
                  ✕ ABORTED BEFORE ENTER
                </div>
                <div style={{ fontSize: 38, fontWeight: 700, color: C.ivory }}>Canceled. Nothing sent.</div>
                <div style={{ fontFamily: F.mono, fontSize: 19, color: C.silver, marginTop: 10 }}>
                  Clipboard untouched · Target verified
                </div>
              </div>
            ) : (
              <div>
                <div style={{ display: "flex", justifyContent: "space-between", fontFamily: F.mono, fontSize: 16, color: C.amber, marginBottom: 12 }}>
                  <span>SENDING TO CURSOR</span>
                  <span>{(1.6 - t * 0.8).toFixed(1)}s</span>
                </div>
                <div style={{ fontFamily: F.mono, fontSize: 34, fontWeight: 700, color: C.ivory }}>
                  cargo test --release
                </div>
              </div>
            )}
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {[
              { k: "Esc", label: "Tap Esc during countdown", active: canceled },
              { k: "← →", label: "Shake your head once", active: false },
              { k: "Say", label: "Say “scratch that” or keep talking", active: false },
            ].map((row, i) => (
              <div
                key={i}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 20,
                  padding: "20px 26px",
                  borderRadius: 20,
                  background: row.active ? "rgba(255, 107, 94, 0.16)" : "rgba(255,255,255,0.05)",
                  border: `1px solid ${row.active ? C.danger : "rgba(255,255,255,0.12)"}`,
                }}
              >
                <span
                  style={{
                    minWidth: 74,
                    padding: "8px 14px",
                    borderRadius: 12,
                    background: "rgba(0,0,0,0.45)",
                    fontFamily: F.mono,
                    fontSize: 20,
                    fontWeight: 700,
                    textAlign: "center",
                    color: row.active ? C.danger : C.amber,
                  }}
                >
                  {row.k}
                </span>
                <span style={{ fontSize: 24, fontWeight: 600, color: C.ivory }}>{row.label}</span>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      {/* SCENE B: Destructive Command Guard (2.6 - 5.6s) */}
      {inSceneB > 0.01 && inSceneC < 0.99 ? (
        <div
          style={{
            opacity: inSceneB * (1 - inSceneC),
            transform: `translateY(${(1 - inSceneB) * 24 - inSceneC * 24}px)`,
            display: "flex",
            flexDirection: "column",
            gap: 28,
          }}
        >
          <div>
            <div style={{ fontFamily: F.mono, fontSize: 18, letterSpacing: "0.14em", color: C.danger, marginBottom: 12 }}>
              02 · DESTRUCTIVE COMMAND GUARD
            </div>
            <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 80, lineHeight: 0.96, color: C.ivory }}>
              Risky commands
              <br />
              <span style={{ color: C.danger }}>never press Enter.</span>
            </div>
          </div>

          <div
            style={{
              padding: "32px 36px",
              borderRadius: 26,
              background: "rgba(38, 8, 16, 0.96)",
              border: `1.8px solid ${C.danger}`,
              boxShadow: "0 28px 64px rgba(0,0,0,0.7), 0 0 40px rgba(255, 107, 94, 0.22)",
            }}
          >
            <div style={{ fontFamily: F.mono, fontSize: 16, fontWeight: 700, color: C.danger, letterSpacing: "0.1em", marginBottom: 14 }}>
              ⚠ AUTO-ENTER BLOCKED · PASTE ONLY
            </div>
            <div style={{ fontFamily: F.mono, fontSize: 33, fontWeight: 700, color: C.ivory }}>
              $ git reset --hard && rm -rf dist/
            </div>
            <div
              style={{
                marginTop: 22,
                padding: "14px 20px",
                borderRadius: 14,
                background: "rgba(255, 107, 94, 0.16)",
                border: "1px solid rgba(255, 107, 94, 0.45)",
                fontFamily: F.mono,
                fontSize: 19,
                fontWeight: 700,
                color: "#ffc2bc",
              }}
            >
              🔒 Enter key locked · Left at prompt for your hand
            </div>
          </div>

          <div
            style={{
              borderRadius: 22,
              overflow: "hidden",
              border: "1px solid rgba(255,255,255,0.16)",
              boxShadow: "0 24px 60px rgba(0,0,0,0.65)",
            }}
          >
            <Img src={staticFile("screens/ui-dark-destructive.png")} style={{ width: "100%", display: "block" }} />
          </div>
        </div>
      ) : null}

      {/* SCENE C: 100% On-Device Privacy (5.6 - 7.4s) */}
      {inSceneC > 0.01 ? (
        <div
          style={{
            opacity: inSceneC,
            transform: `translateY(${(1 - inSceneC) * 24}px)`,
            display: "flex",
            flexDirection: "column",
            gap: 28,
          }}
        >
          <div>
            <div style={{ fontFamily: F.mono, fontSize: 18, letterSpacing: "0.14em", color: C.emerald, marginBottom: 12 }}>
              03 · PRIVATE BY DESIGN
            </div>
            <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 82, lineHeight: 0.96, color: C.ivory }}>
              Nothing leaves
              <br />
              <span style={{ color: C.cyan }}>this computer.</span>
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
            {[
              {
                title: "Camera frames stay in RAM",
                value: "0 pictures saved to disk",
                sub: "Reads head angle & lip motion, then discards the frame immediately.",
                accent: C.cyan,
              },
              {
                title: "On-device Whisper GPU speech",
                value: "0 audio bytes uploaded",
                sub: "Transcribes locally on Apple Silicon Metal / CPU. Works offline.",
                accent: C.amber,
              },
              {
                title: "Trusted app allowlist",
                value: "No accidental chat sends",
                sub: "Presses Enter only in terminals & IDEs you trust.",
                accent: C.emerald,
              },
            ].map((card, i) => (
              <div
                key={i}
                style={{
                  padding: "26px 32px",
                  borderRadius: 24,
                  background: "rgba(22, 8, 22, 0.94)",
                  border: `1.5px solid ${card.accent}66`,
                  boxShadow: "0 20px 48px rgba(0,0,0,0.55)",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                  <span style={{ fontSize: 26, fontWeight: 700, color: C.ivory }}>{card.title}</span>
                  <span style={{ fontFamily: F.mono, fontSize: 17, fontWeight: 700, color: card.accent }}>
                    {card.value}
                  </span>
                </div>
                <div style={{ fontSize: 19, color: C.mutedDark, lineHeight: 1.4 }}>{card.sub}</div>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      <FinaleOverlay progress={inFinale} localSec={Math.max(0, t - 7.4)} />
    </ReelStage>
  );
};

// ============================================================================
// REEL HERO: Flagship 15.0s (450 frames @ 30fps) All-in-One Launch Cut
// Synced to public/reel-hero-audio.wav (VO: 01, 02, 03, 07, 11)
// ============================================================================
export const SaywhenReelHero: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = frame / fps;
  const rms = AUDIO_RMS[frame] ?? 0.25;

  const inBeat2 = ease(t, 2.28, 2.52);
  const inBeat3 = ease(t, 5.20, 5.45);
  const inBeat4 = ease(t, 7.80, 8.05);
  const inBeat5 = ease(t, 11.60, 11.85);
  const inFin = ease(t, 13.20, 13.45);

  const captions: ReelCaptionCue[] = [
    { startFrame: 5, endFrame: 70, text: "Voice input.", highlight: "Without a hotkey." },
    { startFrame: 160, endFrame: 210, text: "It can see you", highlight: "looking at the screen." },
    { startFrame: 238, endFrame: 339, text: "You speak like a person.", highlight: "It writes like a developer." },
    { startFrame: 354, endFrame: 400, text: "Nothing leaves", highlight: "this computer." },
  ];

  return (
    <ReelStage badge="LOOK · TALK · SENT" captions={captions} audioFile="reel-hero-audio.wav">
      <div
        style={{
          position: "absolute",
          inset: 0,
          transform: "translate(-420px, 420px) scale(0.95)",
          opacity: 0.85,
          pointerEvents: "none",
        }}
      >
        <HorizonGrid3DCanvas frame={frame} lightPlate={t >= 2.33 && t < 5.27} />
        {t < 2.35 ? <ShatteringKeycaps3DCanvas t={t} shatterTime={1.75} /> : null}
        {t >= 2.33 && t < 5.27 ? (
          <ParametricRibbon3DCanvas t={t - 2.33} mode={t < 3.5 ? 0 : t < 4.4 ? 1 : 2} rms={rms} />
        ) : null}
        {t >= 5.27 && t < 7.87 ? <BiometricFaceMesh3DCanvas t={t - 5.27} rms={rms} /> : null}
        {t >= 7.87 && t < 11.67 ? <AcousticTerrain3DCanvas frame={frame} /> : null}
        {t >= 11.67 && t < 13.25 ? <HexShieldForcefieldCanvas t={t - 11.67} /> : null}
        {t >= 13.25 ? <FibonacciVortexCanvas t={t - 13.25} rms={rms} /> : null}
      </div>

      {t < 2.52 ? (
        <div style={{ opacity: 1 - inBeat2, fontFamily: F.sans, fontSize: 72, fontWeight: 800, color: C.ivory }}>
          Why does voice still need a <span style={{ color: C.amber }}>chord</span>?
        </div>
      ) : null}
      {inBeat2 > 0.01 && inBeat3 < 0.99 ? (
        <div style={{ opacity: inBeat2 * (1 - inBeat3), fontFamily: F.serif, fontStyle: "italic", fontSize: 118, color: C.ivory }}>
          Look. Talk. <span style={{ color: C.amber }}>Sent.</span>
        </div>
      ) : null}
      {inBeat3 > 0.01 && inBeat4 < 0.99 ? (
        <div style={{ opacity: inBeat3 * (1 - inBeat4), fontFamily: F.sans, fontSize: 64, fontWeight: 800, color: C.ivory }}>
          Attention replaces the <span style={{ color: C.amber }}>hotkey</span>.
        </div>
      ) : null}
      {inBeat4 > 0.01 && inBeat5 < 0.99 ? (
        <div style={{ opacity: inBeat4 * (1 - inBeat5), fontFamily: F.mono, fontSize: 34, fontWeight: 700, color: C.amber }}>
          $ git checkout -b feat/voice-gate && cargo test
        </div>
      ) : null}
      {inBeat5 > 0.01 && inFin < 0.99 ? (
        <div style={{ opacity: inBeat5 * (1 - inFin), fontFamily: F.sans, fontSize: 68, fontWeight: 800, color: C.amber }}>
          Runs On Your Computer.
        </div>
      ) : null}

      <FinaleOverlay progress={inFin} localSec={Math.max(0, t - 13.2)} />
    </ReelStage>
  );
};
