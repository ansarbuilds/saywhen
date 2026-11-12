import React, { type CSSProperties, type ReactNode } from "react";
import { AbsoluteFill, Img, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { loadFont } from "@remotion/fonts";
import plusJakarta700 from "@fontsource/plus-jakarta-sans/files/plus-jakarta-sans-latin-700-normal.woff2";
import { C, F, eio, seg } from "./SaywhenDemo";

loadFont({ family: "Plus Jakarta Sans", url: plusJakarta700, weight: "700" });
loadFont({ family: "JetBrains Mono", url: staticFile("jetbrains-mono.woff2"), weight: "100 800" });

const panel = "#180514";
const line = "rgba(223, 229, 236, 0.23)";
const tint = "#f0eaf0";
const full = "100%";

function useTime() {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  return frame === durationInFrames - 1 ? 0 : frame / fps;
}

const ease = (t: number, a: number, b: number) => eio(seg(t, a, b));
const windowAt = (t: number, a: number, b: number, edge = 0.4) =>
  ease(t, a, a + edge) * (1 - ease(t, b - edge, b));

function Canvas({ children }: { children: ReactNode }) {
  return (
    <AbsoluteFill style={{ background: C.plum, color: C.silver, fontFamily: F.sans }}>
      {children}
    </AbsoluteFill>
  );
}

function Title({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <div style={{ fontFamily: F.serif, fontSize: 86, lineHeight: 0.96, ...style }}>{children}</div>;
}

function Reveal({ value, children, style }: { value: number; children: ReactNode; style?: CSSProperties }) {
  return <div style={{ opacity: value, transform: `translateY(${(1 - value) * 8}px)`, ...style }}>{children}</div>;
}

const states = [
  { name: "Waiting", hint: "Look at the screen." },
  { name: "Ready", hint: "Start talking." },
  { name: "Listening", hint: "Finish your thought." },
  { name: "Writing", hint: "Turning voice into text." },
  { name: "Check", hint: "Review before it sends." },
] as const;

function Meter({ amount = 1 }: { amount?: number }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 4, height: 36 }}>
      {[8, 17, 26, 34, 26, 17, 8].map((height, index) => (
        <span key={index} style={{ width: 4, height: 5 + (height - 5) * amount, borderRadius: 3, background: "currentColor" }} />
      ))}
    </div>
  );
}

function HudPill({ index, focus = 0, meter = 0.8, detail }: {
  index: number; focus?: number; meter?: number; detail?: string;
}) {
  return (
    <div style={{ position: "relative", display: "inline-flex", alignItems: "center", gap: 12, height: 60, padding: "0 19px 0 13px", borderRadius: 999, background: C.plum, border: `2px solid ${line}`, fontSize: 28, whiteSpace: "nowrap" }}>
      <div style={{ position: "absolute", inset: -2, border: `2px solid ${C.amber}`, borderRadius: "inherit", opacity: focus }} />
      <Img src={staticFile("logo.png")} style={{ width: 33, height: 33, borderRadius: 8, filter: "grayscale(1)" }} />
      <span style={{ position: "relative", width: 13, height: 13, boxSizing: "border-box", borderRadius: full, background: index === 3 ? "transparent" : C.silver, border: index === 3 ? `2px solid ${C.silver}` : undefined, borderRightColor: index === 3 ? "transparent" : undefined }} />
      <strong style={{ fontWeight: 700 }}>{states[index].name}</strong>
      {(index === 1 || index === 2) && <Meter amount={index === 1 ? 0 : meter} />}
      {detail && <span style={{ color: C.mutedDark, fontSize: 23 }}>{detail}</span>}
    </div>
  );
}

function Bar({ amount, accent = 1 }: { amount: number; accent?: number }) {
  return (
    <div style={{ height: 6, borderRadius: 6, background: line, overflow: "hidden" }}>
      <div style={{ height: full, width: `${amount * 100}%`, background: `color-mix(in srgb, ${C.amber} ${accent * 100}%, ${C.silver})`, borderRadius: 6 }} />
    </div>
  );
}

function Preview({ command, amount, policy = "Paste + Enter into Terminal", hint = "Esc or shake head to cancel. Nod to send now.", warning = false, accent = warning ? 0 : 1 }: {
  command: string; amount: number; policy?: string; hint?: string; warning?: boolean; accent?: number;
}) {
  return (
    <div style={{ background: C.plumDeep, border: `2px solid ${line}`, borderRadius: 26, padding: "18px 24px 17px", display: "grid", gap: 11 }}>
      <div style={{ color: warning ? C.amber : C.mutedDark, fontWeight: warning ? 700 : 500, fontSize: 24 }}>{policy}</div>
      <div style={{ fontFamily: F.mono, fontSize: 34, color: C.ivory }}>{command}</div>
      <Bar amount={amount} accent={accent} />
      <div style={{ color: C.mutedDark, fontSize: 23 }}>{hint}</div>
    </div>
  );
}

function Terminal({ command, sent = 0, typed = 0 }: { command: string; sent?: number; typed?: number }) {
  return (
    <div style={{ height: full, borderRadius: 28, overflow: "hidden", background: panel, border: `2px solid ${line}` }}>
      <div style={{ display: "flex", alignItems: "center", gap: 20, padding: "15px 25px", background: C.plumDeep, borderBottom: `2px solid ${line}`, fontSize: 24 }}>
        <div style={{ display: "flex", gap: 8 }}>{[0, 1, 2].map(dot => <span key={dot} style={{ width: 11, height: 11, border: `2px solid ${C.mutedDark}`, borderRadius: full }} />)}</div>
        Terminal
      </div>
      <div style={{ padding: "24px 28px", fontFamily: F.mono, fontSize: 34, lineHeight: 1.55 }}>
        <div>
          <span style={{ color: C.mutedDark }}>$ </span>
          <span style={{ position: "relative" }}>
            <span style={{ opacity: typed }}>{command}</span>
            <span style={{ position: "absolute", left: 0, opacity: 1 - typed, color: C.mutedDark }}>▏</span>
          </span>
          <span style={{ opacity: typed * (1 - sent), color: C.mutedDark }}>▏</span>
        </div>
        <div style={{ opacity: sent, color: C.mutedDark }}>$ ▏</div>
      </div>
    </div>
  );
}

const cargo = "cargo test --release";
const rawCargo = "cargo test dash dash release";

export function HeroLoop() {
  const t = useTime();
  const stages = [0.95, 2.05, 4.0, 5.0, 6.85];
  const index = stages.filter(at => t >= at).length % states.length;
  const boundary = Math.min(...[...stages, 8.2].map(at => Math.abs(t - at)));
  const pillText = ease(boundary, 0, 0.23);
  const sent = windowAt(t, 6.85, 8.75, 0.6);
  const typed = windowAt(t, 6.85, 8.75, 0.45);
  const spoken = ["cargo", "test", "dash", "dash", "release"];
  const partial = 1 - ease(t, 4.55, 5.05);
  return (
    <Canvas>
      <div style={{ position: "absolute", left: 88, top: 126 }}>
        <div style={{ fontSize: 28, fontWeight: 700, marginBottom: 22 }}>saywhen</div>
        <Title style={{ fontSize: 142 }}>Look. Talk.<br />Sent.</Title>
        <div style={{ fontSize: 30, lineHeight: 1.5, color: C.mutedDark, marginTop: 30 }}>Hands-free voice input<br />for your terminal.</div>
      </div>
      <div style={{ position: "absolute", left: 650, right: 88, top: 115, height: 560 }}>
        <Terminal command={cargo} typed={typed} sent={sent} />
        <div style={{ position: "absolute", left: 34, right: 34, bottom: 132 }}>
          <Reveal value={windowAt(t, 4.95, 6.9, 0.4)} style={{ position: "absolute", bottom: 0, left: 0, right: 0 }}>
            <Preview command={cargo} amount={1 - seg(t, 5.25, 6.85)} />
          </Reveal>
          <Reveal value={partial} style={{ minHeight: 48, fontSize: 34, fontFamily: F.mono, color: C.mutedDark }}>
            {spoken.map((word, i) => <span key={i} style={{ opacity: ease(t, 2.1 + i * 0.37, 2.37 + i * 0.37), marginRight: 18 }}>{word}</span>)}
          </Reveal>
        </div>
        <div style={{ position: "absolute", left: 34, bottom: 42, opacity: pillText }}>
          <HudPill index={index} detail={index === 0 && t > 6.85 && t < 8.2 ? "Sent to Terminal" : states[index].hint} meter={0.72 + 0.21 * Math.sin(t * 7)} />
        </div>
      </div>
    </Canvas>
  );
}

export function FiveStates() {
  const t = useTime();
  return (
    <Canvas>
      <Title style={{ position: "absolute", left: 88, top: 76 }}>The five states of saywhen</Title>
      <div style={{ position: "absolute", left: 88, right: 88, top: 222, display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 22 }}>
        {states.map((state, index) => {
          const start = 1.18 + (index - 1) * 1.48;
          const focus = index === 0 ? 1 - ease(t, 0.9, 1.18) + ease(t, 7.05, 7.48) : windowAt(t, start, start + 1.42, 0.32);
          return (
            <div key={state.name} style={{ borderTop: `2px solid ${line}`, paddingTop: 31 }}>
              <HudPill index={index} focus={focus} meter={0.6 + focus * 0.25} />
              <div style={{ marginTop: 23, color: C.silver, fontSize: 28, lineHeight: 1.3 }}>{state.hint}</div>
            </div>
          );
        })}
      </div>
    </Canvas>
  );
}

export function Cleanup() {
  const t = useTime();
  const reset = 1 - ease(t, 5.25, 5.8);
  return (
    <Canvas>
      <Title style={{ position: "absolute", left: 88, top: 55 }}>Say the words. Get the command.</Title>
      <div style={{ position: "absolute", left: 88, right: 88, top: 177, display: "grid", gridTemplateColumns: "1.2fr 1fr", gap: "16px 36px" }}>
        <div style={{ fontSize: 25, color: C.mutedDark }}>You say</div>
        <div style={{ fontSize: 25, color: C.mutedDark }}>saywhen writes</div>
        {[[rawCargo, cargo], ["cubectal get pods", "kubectl get pods"]].map(([raw, clean], index) => {
          const strike = ease(t, 0.72 + index * 1.95, 1.35 + index * 1.95) * reset;
          const result = ease(t, 1.18 + index * 1.95, 1.73 + index * 1.95) * reset;
          const accent = index === 0 ? 1 - ease(t, 2.35, 2.65) : ease(t, 2.65, 2.98);
          return (
            <React.Fragment key={clean}>
              <div style={{ borderTop: `2px solid ${line}`, padding: "20px 0", fontSize: 36 }}>
                <span style={{ display: "inline-block", position: "relative", color: C.silver }}>
                  {raw}<span style={{ position: "absolute", top: "55%", left: 0, width: `${strike * 100}%`, height: 3, background: C.mutedDark }} />
                </span>
              </div>
              <div style={{ borderTop: `2px solid ${line}`, paddingTop: 13 }}>
                <Reveal value={result} style={{ background: C.white, color: C.plum, borderRadius: 15, display: "inline-flex", alignItems: "center", gap: 15, padding: "12px 20px", fontFamily: F.mono, fontSize: 32 }}>
                  <span style={{ width: 6, height: 28, background: C.amber, opacity: accent, borderRadius: 4 }} />{clean}
                </Reveal>
              </div>
            </React.Fragment>
          );
        })}
      </div>
    </Canvas>
  );
}

const modes = [
  { name: "Just pause", detail: "Stop talking. A countdown runs, then it sends.", shape: <path d="M8 6v12M16 6v12" />, before: "Counting down", after: "Sent" },
  { name: "Nod to send", detail: "Nod when you’re ready to send.", shape: <><circle cx="12" cy="9" r="4" /><path d="M12 15v5M9 18l3 3 3-3" /></>, before: "Waiting for a nod", after: "Sent" },
  { name: 'Say "send it"', detail: "End with these words to press Enter.", shape: <path d="M4 6h16v9H9l-5 4z" />, before: '“send it”', after: "Sent" },
  { name: "Type only", detail: "Your words are typed without Enter.", shape: <><rect x="5" y="4" width="14" height="17" rx="1" /><path d="M9 9h6M9 13h6M9 17h3" /></>, before: "$", after: "$ git status ▏" },
];

function Glyph({ children, size = 30, style }: { children: ReactNode; size?: number; style?: CSSProperties }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" style={style}>
      {children}
    </svg>
  );
}

export function SendStyles() {
  const t = useTime();
  return (
    <Canvas>
      <Title style={{ position: "absolute", left: 88, top: 92 }}>How do you want to send?</Title>
      <div style={{ position: "absolute", left: 88, right: 88, top: 230, display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 22 }}>
        {modes.map((mode, index) => {
          const start = 0.25 + index * 2.35;
          const local = t - start;
          const active = windowAt(local, 0, 2.2, 0.35);
          const before = 1 - ease(local, 1.14, 1.38);
          const result = ease(local, 1.38, 1.64);
          const nod = windowAt(local, 0.67, 1.42, 0.35);
          return (
            <div key={mode.name} style={{ position: "relative", height: 360, boxSizing: "border-box", padding: 26, borderRadius: 26, background: C.white, border: `2px solid ${C.borderLight}`, color: C.plum }}>
              <div style={{ position: "absolute", inset: -2, borderRadius: "inherit", border: `3px solid ${C.amber}`, opacity: active }} />
              <div style={{ position: "absolute", right: 24, top: 24, width: 29, height: 29, borderRadius: full, border: `2px solid ${C.borderLight}`, display: "grid", placeItems: "center" }}>
                <div style={{ position: "absolute", inset: -1, borderRadius: "inherit", background: C.plum, opacity: active }} />
                <Glyph size={19} style={{ opacity: active, color: C.white, position: "relative" }}><path d="m6 12 4 4 8-8" /></Glyph>
              </div>
              <div style={{ display: "grid", placeItems: "center", width: 61, height: 61, background: tint, borderRadius: 17 }}>
                <Glyph style={{ transform: `translateY(${index === 1 ? nod * 8 : 0}px)` }}>{mode.shape}</Glyph>
              </div>
              <div style={{ marginTop: 17, fontSize: 34, fontWeight: 700, letterSpacing: "-0.04em" }}>{mode.name}</div>
              <div style={{ marginTop: 10, fontSize: 27, lineHeight: 1.33, color: C.mutedLight }}>{mode.detail}</div>
              <div style={{ position: "absolute", left: 26, right: 26, bottom: 25, height: 62, borderTop: `2px solid ${C.borderLight}`, paddingTop: 10, boxSizing: "border-box" }}>
                <div style={{ position: "relative", height: 33, fontSize: 25, fontFamily: index === 3 ? F.mono : F.sans }}>
                  <Reveal value={active * before} style={{ position: "absolute" }}>{mode.before}</Reveal>
                  <Reveal value={active * result} style={{ position: "absolute", fontWeight: 700 }}>{mode.after}</Reveal>
                </div>
                {index === 0 && <div style={{ opacity: active * before }}><Bar amount={1 - seg(local, 0.45, 1.45)} accent={0} /></div>}
              </div>
            </div>
          );
        })}
      </div>
    </Canvas>
  );
}

export function Cancel() {
  const t = useTime();
  const reset = ease(t, 5, 5.72);
  const preview = 1 - windowAt(t, 2.15, 5.72, 0.36);
  const canceled = windowAt(t, 2.51, 5.36, 0.34);
  const pressed = windowAt(t, 1.66, 2.7, 0.42);
  const count = 1 - 0.76 * seg(t, 0.55, 2.15) * (1 - reset);
  const ways = [
    { label: "Press Esc", icon: <span style={{ fontFamily: F.mono, fontSize: 24 }}>Esc</span> },
    { label: "Shake your head", icon: <Glyph><path d="M4 12h16M7 9l-3 3 3 3m10-6 3 3-3 3" /></Glyph> },
    { label: "Keep talking", icon: <Meter amount={0.65} /> },
  ];
  return (
    <Canvas>
      <Title style={{ position: "absolute", left: 88, top: 55 }}>Cancel before it sends.</Title>
      <div style={{ position: "absolute", left: 88, top: 190, width: 762 }}>
        <Reveal value={preview}>
          <Preview command={cargo} amount={count} accent={1 - ease(t, 1.3, 1.66) * (1 - reset)} />
        </Reveal>
        <Reveal value={canceled} style={{ position: "absolute", left: 0, right: 0, top: 0, height: 161, display: "grid", alignContent: "center", padding: "0 26px", background: C.plumDeep, border: `2px solid ${line}`, borderRadius: 26, fontSize: 37, fontWeight: 700 }}>
          Canceled. Nothing sent.
        </Reveal>
      </div>
      <div style={{ position: "absolute", left: 952, top: 177, display: "grid", gap: 16 }}>
        {ways.map(({ label, icon }, index) => (
          <div key={label} style={{ display: "flex", alignItems: "center", gap: 20, fontSize: 30 }}>
            <div style={{ position: "relative", width: 86, height: 58, borderRadius: 13, border: `2px solid ${line}`, display: "grid", placeItems: "center", transform: `translateY(${index === 0 ? pressed * 3 : 0}px)` }}>
              {index === 0 && <span style={{ position: "absolute", inset: -2, borderRadius: "inherit", border: `2px solid ${C.amber}`, opacity: pressed }} />}
              {icon}
            </div>
            {label}
          </div>
        ))}
      </div>
    </Canvas>
  );
}

export function Safety() {
  const t = useTime();
  const command = "rm -rf dist/";
  const amount = 1 - seg(t, 0.55, 2.32);
  const typed = windowAt(t, 2.32, 5.75, 0.45);
  const enter = ease(t, 2.55, 2.95) * (1 - ease(t, 5.15, 5.75));
  return (
    <Canvas>
      <Title style={{ position: "absolute", left: 88, top: 58, fontSize: 83 }}>Risky commands wait for your hand.</Title>
      <div style={{ position: "absolute", left: 88, top: 201, width: 701 }}>
        <Preview command={command} amount={amount + ease(t, 5.3, 5.8)} warning policy="Destructive command: paste only into Terminal" hint="saywhen leaves Enter to you." />
      </div>
      <div style={{ position: "absolute", left: 829, right: 88, top: 182, height: 233 }}>
        <Terminal command={command} typed={typed} />
        <Reveal value={enter} style={{ position: "absolute", left: 28, bottom: 22, display: "flex", alignItems: "center", gap: 13, color: C.silver, fontSize: 24 }}>
          <span style={{ fontFamily: F.mono, border: `2px solid ${line}`, padding: "4px 11px", borderRadius: 9 }}>Enter</span>
          Waiting for you
        </Reveal>
      </div>
    </Canvas>
  );
}

function CameraFrame({ amount }: { amount: number }) {
  return (
    <div style={{ width: 395, height: 252, boxSizing: "border-box", padding: 16, borderRadius: 24, background: C.white, color: C.plum }}>
      <div style={{ fontSize: 25, fontWeight: 700, margin: "0 0 10px 3px" }}>Camera</div>
      <div style={{ position: "relative", height: 174, overflow: "hidden", borderRadius: 15, background: C.plumDeep, display: "grid", placeItems: "center" }}>
        <div style={{ position: "absolute", width: 105, height: 136, border: `2px solid ${line}`, borderRadius: full }} />
        <div style={{ position: "absolute", inset: 0, opacity: ease(amount, 0.4, 1) }}>
          <div style={{ position: "absolute", left: 145, top: 31, width: 71, height: 87, borderRadius: "48% 48% 43% 43%", background: "#cabac8" }} />
          <div style={{ position: "absolute", left: 119, top: 114, width: 123, height: 77, borderRadius: "50% 50% 0 0", background: "#8b7487" }} />
          <div style={{ position: "absolute", left: 174, top: 86, width: 13, height: 6, borderRadius: full, background: C.plumDeep }} />
        </div>
        <div style={{ position: "relative", opacity: 1 - ease(amount, 0, 0.4), color: C.silver, fontSize: 24 }}>Frame discarded</div>
      </div>
    </div>
  );
}

export function Privacy() {
  const t = useTime();
  const clear = 1 - ease(t, 6.25, 7.6);
  const picture = 1 - ease(t, 3.63, 4.73) * clear;
  const readings = [
    { title: "Head angle", value: "Facing screen" },
    { title: "Lips moving", value: "Yes" },
    { title: "Face present", value: "Yes" },
  ];
  return (
    <Canvas>
      <Title style={{ position: "absolute", left: 88, top: 48, fontSize: 76 }}>The picture is never saved.</Title>
      <div style={{ position: "absolute", left: 91, top: 133, fontSize: 32, color: C.mutedDark }}>Only the numbers are used.</div>
      <div style={{ position: "absolute", left: 88, top: 198 }}><CameraFrame amount={picture} /></div>
      <div style={{ position: "absolute", left: 518, top: 297, color: C.mutedDark }}><Glyph size={54}><path d="M4 12h15m-6-6 6 6-6 6" /></Glyph></div>
      <div style={{ position: "absolute", left: 610, right: 88, top: 232, display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 18 }}>
        {readings.map(({ title, value }, i) => {
          const shown = ease(t, 0.9 + i * 0.63, 1.46 + i * 0.63) * clear;
          return (
            <div key={title} style={{ border: `2px solid ${line}`, borderRadius: 21, minHeight: 151, padding: "24px 17px", boxSizing: "border-box" }}>
              <div style={{ fontSize: 26, color: C.mutedDark }}>{title}</div>
              <Reveal value={shown} style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 16, fontSize: 29, fontWeight: 700, whiteSpace: "nowrap" }}>
                <span style={{ width: 9, height: 9, borderRadius: full, background: i === 0 ? C.amber : C.silver }} />
                {value}
              </Reveal>
            </div>
          );
        })}
      </div>
    </Canvas>
  );
}
