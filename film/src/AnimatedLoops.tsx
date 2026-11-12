import React, { type ReactNode } from "react";
import { AbsoluteFill, Img, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { loadFont } from "@remotion/fonts";
import instrumentSerif400 from "@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2";
import instrumentSerif400Italic from "@fontsource/instrument-serif/files/instrument-serif-latin-400-italic.woff2";
import plusJakarta400 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-400-normal.woff2";
import plusJakarta500 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-500-normal.woff2";
import plusJakarta600 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-600-normal.woff2";
import plusJakarta700 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-700-normal.woff2";
import { C, F, eio, seg } from "./SaywhenDemo";

loadFont({ family: "Instrument Serif", url: instrumentSerif400, weight: "400" });
loadFont({ family: "Instrument Serif", url: instrumentSerif400Italic, weight: "400", style: "italic" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta400, weight: "400" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta500, weight: "500" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta600, weight: "600" });
loadFont({ family: "Plus Jakarta Sans", url: plusJakarta700, weight: "700" });
loadFont({ family: "JetBrains Mono", url: staticFile("jetbrains-mono.woff2"), weight: "100 800" });

const OBSIDIAN_BG =
  "radial-gradient(920px 440px at 16% 12%, rgba(92, 20, 78, 0.42), transparent 65%), radial-gradient(760px 420px at 88% 84%, rgba(251, 179, 3, 0.14), transparent 60%), linear-gradient(155deg, #160514 0%, #0d030c 100%)";

function useLoopTime() {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  return frame === durationInFrames - 1 ? 0 : frame / fps;
}

const ease = (t: number, a: number, b: number) => eio(seg(t, a, b));
const windowAt = (t: number, a: number, b: number, edge = 0.4) =>
  ease(t, a, a + edge) * (1 - ease(t, b - edge, b));

function LoopStage({ children }: { children: ReactNode }) {
  return (
    <AbsoluteFill
      style={{
        background: OBSIDIAN_BG,
        color: C.ivory,
        fontFamily: F.sans,
        overflow: "hidden",
      }}
    >
      {children}
    </AbsoluteFill>
  );
}

// Crisp seamless pill stage bar with animated equalizers & glowing progression trace
export const FiveStates: React.FC = () => {
  const t = useLoopTime();
  const states = [
    { name: "Waiting", caption: "Face not locked", badge: "01", accent: C.mutedDark, t0: 0.0, t1: 1.5 },
    { name: "Ready", caption: "Looking at screen", badge: "02", accent: C.cyan, t0: 1.5, t1: 3.0 },
    { name: "Listening", caption: "Capturing with pre-roll", badge: "03", accent: C.amber, t0: 3.0, t1: 4.8 },
    { name: "Writing", caption: "Whisper Metal GPU", badge: "04", accent: "#dfa8ff", t0: 4.8, t1: 6.2 },
    { name: "Check", caption: "Review before Enter", badge: "05", accent: C.emerald, t0: 6.2, t1: 8.0 },
  ];

  // Smooth loop wrap progress 0..4
  const activeIdx = Math.min(4, Math.floor((t / 8) * 5));
  const tracePct = Math.min(100, (t / 7.2) * 100);

  return (
    <LoopStage>
      <div style={{ padding: "48px 72px", height: "100%", boxSizing: "border-box", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 54, lineHeight: 1, color: C.ivory }}>
            The five states of <span style={{ color: C.amber }}>saywhen</span>
          </div>
          <div style={{ fontFamily: F.mono, fontSize: 15, letterSpacing: "0.14em", color: C.mutedDark }}>
            ATTENTION + AUDIO MULTIMODAL MACHINE
          </div>
        </div>

        {/* Continuous optical signal trace behind pills */}
        <div style={{ position: "relative", paddingTop: 18 }}>
          <div
            style={{
              position: "absolute",
              left: 90,
              right: 90,
              top: 52,
              height: 2,
              background: "rgba(255,255,255,0.10)",
              borderRadius: 99,
            }}
          >
            <div
              style={{
                width: `${tracePct}%`,
                height: "100%",
                background: "linear-gradient(90deg, rgba(130,232,255,0.7), rgba(251,179,3,0.95), rgba(61,220,151,0.9))",
                boxShadow: "0 0 18px rgba(251, 179, 3, 0.55)",
              }}
            />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 22, position: "relative" }}>
            {states.map((st, idx) => {
              const lit = idx === activeIdx ? 1 : 0;
              return (
                <div
                  key={st.name}
                  style={{
                    padding: "20px 20px 22px",
                    borderRadius: 20,
                    background: lit
                      ? "linear-gradient(155deg, rgba(60, 18, 52, 0.96), rgba(20, 6, 18, 0.96))"
                      : "rgba(20, 7, 20, 0.86)",
                    border: lit ? `1.5px solid ${st.accent}` : "1px solid rgba(255, 255, 255, 0.12)",
                    boxShadow: lit ? `0 22px 48px rgba(0, 0, 0, 0.6), 0 0 30px ${st.accent}33` : "0 10px 26px rgba(0,0,0,0.35)",
                    transform: `translateY(${lit ? -6 : 0}px)`,
                    transition: "none",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 14 }}>
                    <Img src={staticFile("logo.png")} style={{ width: 28, height: 28, borderRadius: 7, opacity: lit ? 1 : 0.72 }} />
                    <span
                      style={{
                        fontFamily: F.mono,
                        fontSize: 13,
                        fontWeight: 700,
                        padding: "4px 10px",
                        borderRadius: 999,
                        background: lit ? `${st.accent}24` : "rgba(255,255,255,0.06)",
                        color: lit ? st.accent : C.mutedDark,
                      }}
                    >
                      {st.badge}
                    </span>
                  </div>

                  <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                    <span
                      style={{
                        width: 10,
                        height: 10,
                        borderRadius: "50%",
                        background: lit ? st.accent : "rgba(255,255,255,0.28)",
                        boxShadow: lit ? `0 0 12px ${st.accent}` : "none",
                      }}
                    />
                    <span style={{ fontSize: 22, fontWeight: 700, color: lit ? C.ivory : "#c4b6c1" }}>{st.name}</span>
                  </div>

                  <div style={{ fontSize: 15, color: C.mutedDark, marginTop: 8, lineHeight: 1.4 }}>{st.caption}</div>

                  {/* Micro visualizer per state */}
                  <div style={{ height: 22, marginTop: 14, display: "flex", alignItems: "center", gap: 4 }}>
                    {st.name === "Listening" ? (
                      [0, 1, 2, 3, 4, 5, 6].map((b) => (
                        <i
                          key={b}
                          style={{
                            width: 4,
                            height: lit ? 8 + Math.abs(Math.sin(t * 7.5 + b * 0.9)) * 13 : 5,
                            borderRadius: 99,
                            background: lit ? st.accent : "rgba(255,255,255,0.2)",
                          }}
                        />
                      ))
                    ) : st.name === "Writing" ? (
                      <div style={{ width: "100%", height: 4, borderRadius: 99, background: "rgba(255,255,255,0.1)", overflow: "hidden" }}>
                        <div style={{ width: lit ? `${((t * 110) % 100)}%` : "25%", height: "100%", background: st.accent }} />
                      </div>
                    ) : st.name === "Check" ? (
                      <div style={{ width: "100%", height: 4, borderRadius: 99, background: "rgba(255,255,255,0.1)", overflow: "hidden" }}>
                        <div style={{ width: lit ? `${Math.max(12, (1 - (t - 6.2) / 1.8) * 100)}%` : "85%", height: "100%", background: st.accent }} />
                      </div>
                    ) : (
                      <span style={{ fontFamily: F.mono, fontSize: 12, color: lit ? st.accent : "rgba(255,255,255,0.25)" }}>
                        {st.name === "Ready" ? "gaze +2° · lips closed" : "idle · hotkey ready"}
                      </span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <div style={{ fontFamily: F.mono, fontSize: 14, color: C.mutedDark, display: "flex", gap: 28 }}>
          <span>01 · Look to arm</span>
          <span>02 · Talk immediately</span>
          <span>03 · Cancelable Check before Enter</span>
        </div>
      </div>
    </LoopStage>
  );
};

// Hero loop (used for GitHub README) showcasing real Obsidian Dark App plates
export const HeroLoop: React.FC = () => {
  const t = useLoopTime();
  const phase = t < 3 ? 0 : t < 6 ? 1 : 2;
  const imgSrc =
    phase === 0
      ? "screens/ui-dark-ready.png"
      : phase === 1
      ? "screens/ui-dark-listening.png"
      : "screens/ui-dark-preview.png";
  const tag =
    phase === 0
      ? "01 LOCKS TO YOUR GAZE · NO HOTKEY"
      : phase === 1
      ? "02 TALKS + 500 MS PRE-ROLL"
      : "03 CANCELABLE CHECK · SAFE ENTER";

  return (
    <LoopStage>
      <div style={{ padding: "42px 64px", display: "grid", gridTemplateColumns: "0.78fr 1.22fr", gap: 46, alignItems: "center", height: "100%", boxSizing: "border-box" }}>
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: 14, marginBottom: 18 }}>
            <Img src={staticFile("logo.png")} style={{ width: 42, height: 42, borderRadius: 11 }} />
            <span style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.14em", color: C.amber }}>{tag}</span>
          </div>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 78, lineHeight: 0.98, color: C.ivory }}>
            Look. Talk. <span style={{ color: C.amber }}>Sent.</span>
          </div>
          <p style={{ fontSize: 21, lineHeight: 1.5, color: C.silver, marginTop: 22 }}>
            Hands-free voice input that checks you are looking at the screen, cleans spoken commands on-device, and shows a cancelable preview before pressing Enter.
          </p>
        </div>
        <div
          style={{
            borderRadius: 18,
            overflow: "hidden",
            border: "1px solid rgba(255, 255, 255, 0.14)",
            boxShadow: "0 36px 88px rgba(0, 0, 0, 0.72), 0 0 60px rgba(251, 179, 3, 0.16)",
          }}
        >
          <Img src={staticFile(imgSrc)} style={{ width: "100%", height: "auto", display: "block" }} />
        </div>
      </div>
    </LoopStage>
  );
};

// Cleanup loop (docs/README): laser transformation from spoken English to CLI
export const Cleanup: React.FC = () => {
  const t = useLoopTime();
  const sweep = ease(t, 1.0, 3.2);
  const done = ease(t, 2.8, 3.6);
  return (
    <LoopStage>
      <div style={{ padding: "54px 78px", display: "flex", flexDirection: "column", justifyContent: "space-between", height: "100%", boxSizing: "border-box" }}>
        <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 52, color: C.ivory }}>
          Speak like a human. <span style={{ color: C.amber }}>Paste exact syntax.</span>
        </div>
        <div style={{ position: "relative", padding: "36px 40px", borderRadius: 20, background: "rgba(22, 8, 22, 0.92)", border: `1px solid ${done > 0.5 ? C.borderAmber : C.borderDark}` }}>
          <div style={{ fontFamily: F.mono, fontSize: 15, color: C.mutedDark, marginBottom: 14, textDecoration: sweep > 0.5 ? "line-through" : "none" }}>
            HEARD · um run cargo test dash dash release src main dot rs send it
          </div>
          <div style={{ fontFamily: F.mono, fontSize: 38, fontWeight: 700, color: done > 0.5 ? C.amberSoft : C.ivory }}>
            {done > 0.5 ? "cargo test --release src/main.rs" : "um run cargo test dash dash release src main dot rs"}
          </div>
        </div>
        <div style={{ fontFamily: F.mono, fontSize: 15, color: C.mutedDark }}>
          cleanup.rs · Local deterministic dev formatter · Strips "um", "send it", fixes slashes & flags
        </div>
      </div>
    </LoopStage>
  );
};

// Obsidian Glassmorphic 4 Send Styles (Zero harsh white flash!)
export const SendStyles: React.FC = () => {
  const t = useLoopTime();
  const pause = windowAt(t, 0.0, 2.5);
  const nod = windowAt(t, 2.5, 5.0);
  const phrase = windowAt(t, 5.0, 7.5);
  const typeOnly = windowAt(t, 7.5, 10.0);

  const modes = [
    {
      num: "01",
      name: "Just pause",
      detail: "Adaptive silence countdown, then Enter fires safely.",
      foot: pause > 0.1 ? `Sending in ${(Math.max(0.2, 1.8 - (t / 2.5) * 1.6)).toFixed(1)}s` : "Auto countdown · Enter",
      icon: "II",
      lit: pause,
      accent: C.amber,
    },
    {
      num: "02",
      name: "Nod to send",
      detail: "Think as long as you need. Dip chin once to send.",
      foot: nod > 0.55 ? "Nod +11° detected · Sent" : "Waits for head nod",
      icon: "↓",
      lit: nod,
      accent: C.cyan,
    },
    {
      num: "03",
      name: 'Say "send it"',
      detail: 'Strips "send it" cleanly and submits immediately.',
      foot: phrase > 0.55 ? 'Stripped "send it" · Sent' : "End phrase trigger",
      icon: "💬",
      lit: phrase,
      accent: "#dfa8ff",
    },
    {
      num: "04",
      name: "Type only",
      detail: "Pastes clean text into the app. Never presses Enter.",
      foot: typeOnly > 0.5 ? "Pasted · Enter left to you" : "Safest mode · No Enter",
      icon: "⌘",
      lit: typeOnly,
      accent: C.emerald,
    },
  ];

  return (
    <LoopStage>
      <div style={{ padding: "54px 72px", height: "100%", boxSizing: "border-box", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 60, color: C.ivory, lineHeight: 1 }}>
            Four ways to <span style={{ color: C.amber }}>send</span>
          </div>
          <div style={{ fontFamily: F.mono, fontSize: 15, letterSpacing: "0.14em", color: C.mutedDark }}>
            CHOOSE HOW MUCH CONFIRMATION YOU WANT
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 24 }}>
          {modes.map((m) => {
            const on = m.lit > 0.25;
            return (
              <div
                key={m.name}
                style={{
                  height: 410,
                  padding: "30px 28px",
                  borderRadius: 24,
                  background: on
                    ? "linear-gradient(155deg, rgba(64, 18, 56, 0.96), rgba(20, 6, 18, 0.96))"
                    : "rgba(20, 7, 20, 0.84)",
                  border: on ? `1.8px solid ${m.accent}` : "1px solid rgba(255, 255, 255, 0.13)",
                  boxShadow: on ? `0 26px 56px rgba(0, 0, 0, 0.65), 0 0 36px ${m.accent}2e` : "0 14px 32px rgba(0,0,0,0.4)",
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "space-between",
                  transform: `translateY(${on ? -8 : 0}px)`,
                }}
              >
                <div>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 22 }}>
                    <span
                      style={{
                        width: 44,
                        height: 44,
                        borderRadius: 12,
                        display: "grid",
                        placeItems: "center",
                        fontFamily: F.mono,
                        fontSize: 18,
                        fontWeight: 700,
                        background: on ? `${m.accent}26` : "rgba(255,255,255,0.06)",
                        color: on ? m.accent : C.silver,
                        border: on ? `1px solid ${m.accent}66` : "1px solid rgba(255,255,255,0.1)",
                      }}
                    >
                      {m.icon}
                    </span>
                    <span style={{ fontFamily: F.mono, fontSize: 13, letterSpacing: "0.14em", color: on ? m.accent : C.mutedDark }}>
                      MODE {m.num}
                    </span>
                  </div>
                  <div style={{ fontSize: 28, fontWeight: 700, color: C.ivory, lineHeight: 1.15 }}>{m.name}</div>
                  <div style={{ fontSize: 18, lineHeight: 1.5, color: C.mutedDark, marginTop: 14 }}>{m.detail}</div>
                </div>

                <div
                  style={{
                    padding: "12px 16px",
                    borderRadius: 12,
                    background: on ? `${m.accent}1f` : "rgba(255,255,255,0.04)",
                    border: on ? `1px solid ${m.accent}55` : "1px solid rgba(255,255,255,0.08)",
                    fontFamily: F.mono,
                    fontSize: 15,
                    fontWeight: 700,
                    color: on ? m.accent : C.silver,
                  }}
                >
                  {m.foot}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </LoopStage>
  );
};

// Cancel before send (real HUD styling + smooth interruption shield)
export const Cancel: React.FC = () => {
  const t = useLoopTime();
  const canceled = t >= 3.0;
  const barPct = Math.max(0, Math.min(100, (1 - t / 3.2) * 100));

  return (
    <LoopStage>
      <div style={{ padding: "48px 72px", height: "100%", boxSizing: "border-box", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 54, lineHeight: 1 }}>
            Cancel anytime <span style={{ color: C.amber }}>before Enter</span>
          </div>
          <div style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.14em", color: C.mutedDark }}>
            PREVIEW COUNTDOWN · 100% ABORTABLE
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1.25fr 0.75fr", gap: 36, alignItems: "center" }}>
          <div
            style={{
              padding: "28px 34px",
              borderRadius: 22,
              background: canceled ? "rgba(38, 10, 18, 0.94)" : "rgba(22, 8, 22, 0.94)",
              border: canceled ? "1.5px solid rgba(255, 92, 108, 0.7)" : `1.5px solid ${C.borderAmber}`,
              boxShadow: canceled
                ? "0 24px 60px rgba(0, 0, 0, 0.65), 0 0 36px rgba(255, 92, 108, 0.2)"
                : "0 24px 60px rgba(0, 0, 0, 0.65), 0 0 36px rgba(251, 179, 3, 0.16)",
            }}
          >
            {canceled ? (
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", height: 102 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
                  <span style={{ padding: "6px 14px", borderRadius: 999, background: "rgba(255,92,108,0.18)", color: C.danger, fontFamily: F.mono, fontSize: 14, fontWeight: 700 }}>
                    ABORTED
                  </span>
                  <span style={{ fontSize: 28, fontWeight: 700, color: C.ivory }}>Canceled. Nothing sent.</span>
                </div>
                <span style={{ fontFamily: F.mono, fontSize: 15, color: C.mutedDark }}>Clipboard clean</span>
              </div>
            ) : (
              <div>
                <div style={{ display: "flex", justifyContent: "space-between", fontFamily: F.mono, fontSize: 15, color: C.amber, marginBottom: 12 }}>
                  <span>Paste + Enter into Cursor</span>
                  <span>{(1.8 - t * 0.45).toFixed(1)}s</span>
                </div>
                <div style={{ fontFamily: F.mono, fontSize: 29, fontWeight: 700, color: C.ivory }}>cargo test --release src/main.rs</div>
                <div style={{ height: 6, borderRadius: 99, background: "rgba(255,255,255,0.12)", marginTop: 18, overflow: "hidden" }}>
                  <div style={{ width: `${barPct}%`, height: "100%", background: C.amber }} />
                </div>
              </div>
            )}
          </div>

          <div style={{ display: "grid", gap: 12 }}>
            {[
              { k: "Esc", label: "Tap Esc key", on: canceled },
              { k: "← →", label: "Shake your head", on: false },
              { k: "Say", label: 'Say "scratch that" or keep talking', on: false },
            ].map((item, idx) => (
              <div
                key={idx}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 16,
                  padding: "13px 18px",
                  borderRadius: 14,
                  background: item.on ? "rgba(255, 92, 108, 0.16)" : "rgba(255, 255, 255, 0.04)",
                  border: item.on ? "1px solid rgba(255, 92, 108, 0.55)" : "1px solid rgba(255, 255, 255, 0.1)",
                }}
              >
                <span
                  style={{
                    minWidth: 54,
                    padding: "4px 10px",
                    borderRadius: 8,
                    textAlign: "center",
                    fontFamily: F.mono,
                    fontSize: 14,
                    fontWeight: 700,
                    background: "rgba(0,0,0,0.4)",
                    color: item.on ? C.danger : C.amber,
                  }}
                >
                  {item.k}
                </span>
                <span style={{ fontSize: 17, fontWeight: 600, color: item.on ? C.ivory : C.silver }}>{item.label}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </LoopStage>
  );
};

// Risky command guard: dark crimson padlock shields Terminal Enter key
export const Safety: React.FC = () => {
  const t = useLoopTime();
  const cmd = "git reset --hard && rm -rf target/";
  const typedLen = Math.floor(ease(t, 0.6, 2.2) * cmd.length);
  const shown = cmd.slice(0, typedLen);

  return (
    <LoopStage>
      <div style={{ padding: "48px 72px", height: "100%", boxSizing: "border-box", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 54, lineHeight: 1 }}>
            Risky commands <span style={{ color: C.danger }}>never press Enter.</span>
          </div>
          <div style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.14em", color: C.danger }}>
            DESTRUCTIVE GUARD · PASTE-ONLY DOWNGRADE
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 32, alignItems: "center" }}>
          <div
            style={{
              padding: "24px 28px",
              borderRadius: 20,
              background: "rgba(32, 8, 16, 0.94)",
              border: "1.5px solid rgba(255, 92, 108, 0.58)",
              boxShadow: "0 20px 52px rgba(0,0,0,0.55), 0 0 32px rgba(255,92,108,0.16)",
            }}
          >
            <div style={{ fontFamily: F.mono, fontSize: 13, fontWeight: 700, color: C.danger, marginBottom: 10 }}>
              BLOCKED AUTO-ENTER · PASTE ONLY INTO TERMINAL
            </div>
            <div style={{ fontFamily: F.mono, fontSize: 24, fontWeight: 700, color: C.ivory }}>{cmd}</div>
            <div style={{ fontSize: 15, color: C.mutedDark, marginTop: 12 }}>
              Detected: git reset --hard, rm -rf
            </div>
          </div>

          <div
            style={{
              padding: "24px 28px",
              borderRadius: 20,
              background: "rgba(14, 6, 14, 0.96)",
              border: "1px solid rgba(255,255,255,0.14)",
              boxShadow: "0 20px 52px rgba(0,0,0,0.55)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", fontFamily: F.mono, fontSize: 13, color: C.mutedDark, marginBottom: 14 }}>
              <span>ghostty · Terminal</span>
              <span style={{ color: C.amber }}>ENTER WITHHELD</span>
            </div>
            <div style={{ fontFamily: F.mono, fontSize: 23, color: C.silver }}>
              <span style={{ color: C.amber }}>$ </span>
              {shown}
              <span style={{ color: C.danger }}>|</span>
            </div>
            <div style={{ display: "inline-flex", alignItems: "center", gap: 10, marginTop: 18, padding: "6px 14px", borderRadius: 10, background: "rgba(255, 92, 108, 0.14)", border: "1px solid rgba(255, 92, 108, 0.45)", fontFamily: F.mono, fontSize: 14, color: "#ffb8c0" }}>
              Enter key locked · Waiting for your hand
            </div>
          </div>
        </div>
      </div>
    </LoopStage>
  );
};

// Privacy: Sleek SVG biometric face landmark scanner + RAM discard pulse (Zero cartoon grey face!)
export const Privacy: React.FC = () => {
  const t = useLoopTime();
  const scanPct = ((t % 3.2) / 3.2) * 100;
  const yaw = (Math.sin(t * 1.9) * 2.1).toFixed(1);
  const mouthMar = (0.04 + Math.abs(Math.sin(t * 4.5)) * 0.09).toFixed(3);
  const purged = t > 4.2 && t < 6.4;

  return (
    <LoopStage>
      <div style={{ padding: "46px 72px", height: "100%", boxSizing: "border-box", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <div style={{ fontFamily: F.serif, fontStyle: "italic", fontSize: 54, lineHeight: 1 }}>
            The picture is <span style={{ color: C.cyan }}>never saved.</span>
          </div>
          <div style={{ fontFamily: F.mono, fontSize: 14, letterSpacing: "0.14em", color: C.emerald }}>
            RAM ONLY · 0 DISK WRITES · 0 FRAMES UPLOADED
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "0.86fr 1.14fr", gap: 36, alignItems: "center" }}>
          {/* Sleek dark biometric camera frame */}
          <div
            style={{
              position: "relative",
              height: 270,
              borderRadius: 20,
              background: "radial-gradient(circle at 50% 45%, #281026, #0d040d)",
              border: "1px solid rgba(130, 232, 255, 0.35)",
              boxShadow: "0 20px 50px rgba(0, 0, 0, 0.6), 0 0 36px rgba(130, 232, 255, 0.14)",
              overflow: "hidden",
              display: "grid",
              placeItems: "center",
            }}
          >
            {/* Precision SVG landmark contour */}
            <svg width="240" height="220" viewBox="0 0 240 220" style={{ opacity: purged ? 0.25 : 0.95 }}>
              <path d="M 120 26 C 166 26 186 64 184 114 C 182 158 152 194 120 198 C 88 194 58 158 56 114 C 54 64 74 26 120 26 Z" fill="none" stroke="rgba(130,232,255,0.65)" strokeWidth="2" />
              <path d="M 78 90 Q 94 82 110 90 Q 94 96 78 90 Z" fill="none" stroke="#82e8ff" strokeWidth="1.8" />
              <path d="M 130 90 Q 146 82 162 90 Q 146 96 130 90 Z" fill="none" stroke="#82e8ff" strokeWidth="1.8" />
              <circle cx="94" cy="89" r="3.5" fill="#82e8ff" />
              <circle cx="146" cy="89" r="3.5" fill="#82e8ff" />
              <path d="M 120 92 L 120 124 L 112 130 M 120 124 L 128 130" fill="none" stroke="rgba(130,232,255,0.45)" strokeWidth="1.5" />
              <ellipse cx="120" cy="154" rx="22" ry={6 + Math.abs(Math.sin(t * 4.5)) * 6} fill="none" stroke="#fbb303" strokeWidth="2.4" />
              <path d="M 30 34 L 30 16 L 48 16 M 210 34 L 210 16 L 192 16 M 30 186 L 30 204 L 48 204 M 210 186 L 210 204 L 192 204" fill="none" stroke="#fbb303" strokeWidth="2.2" />
            </svg>
            <div style={{ position: "absolute", left: 22, right: 22, top: `${scanPct}%`, height: 2, background: "linear-gradient(90deg, transparent, #82e8ff, transparent)", opacity: 0.7 }} />
            <div style={{ position: "absolute", left: 16, bottom: 12, fontFamily: F.mono, fontSize: 12, color: purged ? C.emerald : C.cyan }}>
              {purged ? "FRAME DISCARDED IN RAM · ZERO BYTES LEFT" : "WEBCAM IN MEMORY · EXTRACTING NUMBERS"}
            </div>
          </div>

          <div style={{ display: "grid", gap: 14 }}>
            {[
              { k: "Head yaw / pitch", v: `${yaw}° · facing screen`, accent: C.cyan },
              { k: "Lip aperture delta", v: `${mouthMar} · moving`, accent: C.amber },
              { k: "Disk / network output", v: "0 frames saved · 0 bytes sent", accent: C.emerald },
            ].map((row) => (
              <div
                key={row.k}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "20px 26px",
                  borderRadius: 16,
                  background: "rgba(22, 8, 22, 0.90)",
                  border: "1px solid rgba(255, 255, 255, 0.13)",
                }}
              >
                <span style={{ fontSize: 18, fontWeight: 600, color: C.silver }}>{row.k}</span>
                <span style={{ fontFamily: F.mono, fontSize: 18, fontWeight: 700, color: row.accent }}>{row.v}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </LoopStage>
  );
};
