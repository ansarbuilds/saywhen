import React, { useEffect, useMemo, useRef } from "react";
import {
  AbsoluteFill,
  Audio,
  Img,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { AUDIO_RMS } from "./audioData";
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

// ============================================================================
// BRAND TOKENS (Strictly from saywhen src/style.css & src/hud.css)
// ============================================================================
const C = {
  plum: "#22061c",
  plumDeep: "#330a2b",
  plumInk: "#2a0823",
  amber: "#fbb303",
  amberSoft: "#ffc94d",
  silver: "#dfe5ec",
  ivory: "#f6eef4",
  mutedDark: "#c7b8c4",
  mutedLight: "#6d5669",
  armed: "#b8cce4",
  transcribing: "#d8a9d6",
  danger: "#ff6b5e",
  white: "#ffffff",
  borderLight: "rgba(34, 6, 28, 0.11)",
  borderDark: "rgba(223, 229, 236, 0.16)",
} as const;

const F = {
  serif: "'Instrument Serif', Georgia, serif",
  sans: "'Plus Jakarta Sans', -apple-system, sans-serif",
  mono: "'JetBrains Mono', monospace",
} as const;

const VW = 1920;
const VH = 1080;
const CX = VW / 2;
const CY = VH / 2;
const FPS = 30;
const DT = 1 / FPS;

// ============================================================================
// CORE ENGINE MATH (Ported from display-dev/product-film kinetic.html & beats.html)
// ============================================================================
const clamp = (x: number, a: number, b: number) => Math.max(a, Math.min(b, x));
const lerp = (a: number, b: number, u: number) => a + (b - a) * u;
const seg = (t: number, a: number, b: number) =>
  b <= a ? (t >= b ? 1 : 0) : clamp((t - a) / (b - a), 0, 1);

const eo = (u: number) => 1 - Math.pow(1 - u, 3);
const ei = (u: number) => u * u * u;
const eio = (u: number) =>
  u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2;
const bo = (u: number) => {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(u - 1, 3) + c1 * Math.pow(u - 1, 2);
};

// 180-degree shutter motion blur: screen px/frame -> Gaussian stdDeviation
const mb = (pxPerFrame: number) =>
  pxPerFrame > 5 ? Math.min(38, pxPerFrame * 0.22) : 0;

// Button press curve (down, flat bottom, release)
const press = (t: number, p: number) =>
  seg(t, p, p + 0.08) * (1 - seg(t, p + 0.11, p + 0.27));

// 2D Camera keyframe interpolation (log-space zoom m)
interface Cam2DKey {
  t: number;
  cx: number;
  cy: number;
  m: number;
}
function evalCam2D(t: number, keys: Cam2DKey[]) {
  let a = keys[keys.length - 1];
  let b = a;
  if (t < a.t) {
    a = b = keys[0];
    for (let i = 0; i < keys.length - 1; i++) {
      if (t >= keys[i].t && t <= keys[i + 1].t) {
        a = keys[i];
        b = keys[i + 1];
        break;
      }
    }
  }
  const u = b.t > a.t ? eio(seg(t, a.t, b.t)) : 1;
  return {
    cx: lerp(a.cx, b.cx, u),
    cy: lerp(a.cy, b.cy, u),
    m: Math.exp(lerp(Math.log(a.m), Math.log(b.m), u)),
  };
}

function cam2DBlur(
  fn: (tau: number) => { cx: number; cy: number; m: number },
  t: number,
) {
  const c0 = fn(t);
  const c1 = fn(t + DT);
  return {
    bx: mb(Math.abs(c1.m * (c0.cx - c1.cx))),
    by: mb(Math.abs(c1.m * (c0.cy - c1.cy))),
  };
}

function w2s(c: { cx: number; cy: number; m: number }, x: number, y: number) {
  return { x: CX + c.m * (x - c.cx), y: CY + c.m * (y - c.cy) };
}

// 3D Camera keyframe interpolation (camera-rig.md)
interface Cam3DKey {
  t: number;
  x?: number;
  y?: number;
  z?: number;
  rx?: number;
  ry?: number;
}
function evalCam3D(t: number, keys: Cam3DKey[], rot = true) {
  let a = keys[0];
  let b = keys[keys.length - 1];
  for (let i = 0; i < keys.length - 1; i++) {
    if (t >= keys[i].t && t <= keys[i + 1].t) {
      a = keys[i];
      b = keys[i + 1];
      break;
    }
  }
  if (t > b.t) a = b;
  const u = b.t > a.t ? eio(seg(t, a.t, b.t)) : 1;
  let x = lerp(a.x ?? 0, b.x ?? 0, u) + 6 * Math.sin(t * 0.9 + 1.2);
  let y = lerp(a.y ?? 0, b.y ?? 0, u) + 4 * Math.sin(t * 1.25 + 0.4);
  const z = lerp(a.z ?? 0, b.z ?? 0, u);
  let rx = lerp(a.rx ?? 0, b.rx ?? 0, u);
  let ry = lerp(a.ry ?? 0, b.ry ?? 0, u);
  if (rot) {
    ry += 0.4 * Math.sin(t * 0.7);
    rx += 0.3 * Math.sin(t * 0.55 + 2);
  }
  return {
    transform: `translate3d(${(-x).toFixed(2)}px, ${(-y).toFixed(2)}px, ${z.toFixed(2)}px) rotateX(${(-rx).toFixed(2)}deg) rotateY(${(-ry).toFixed(2)}deg)`,
  };
}

// 3D Object placement with Depth-of-Field blur (camera-rig.md: 1px per 110px beyond 90px dead zone)
function p3Style(
  t: number,
  t0: number,
  dur: number,
  o: {
    x?: number;
    y?: number;
    z?: number;
    focus?: number;
    s?: number;
    op?: number;
    tilt?: boolean;
    rx?: number;
    ry?: number;
  } = {},
): React.CSSProperties {
  const u = eo(seg(t, t0, t0 + dur));
  const z = o.z ?? 0;
  const focus = o.focus ?? z;
  const d = Math.abs(z - focus);
  const blur = clamp((d - 90) / 110, 0, 3.5);
  const rx =
    o.rx !== undefined
      ? o.rx
      : o.tilt === false
        ? 0
        : lerp(7, 1.5, u) + 0.6 * Math.sin(0.9 * t);
  const ry =
    o.ry !== undefined
      ? o.ry
      : o.tilt === false
        ? 0
        : lerp(-9, -2, u) + 1.1 * Math.sin(0.7 * t + 1);
  const y = (o.y ?? 0) + lerp(44, 0, u);
  const s = lerp(0.94, 1, u) * (o.s ?? 1);
  const op = u * (o.op ?? 1) * (1 - 0.1 * clamp(blur / 3.5, 0, 1));
  return {
    transformStyle: "preserve-3d",
    transform: `translate3d(${(o.x ?? 0).toFixed(2)}px, ${y.toFixed(2)}px, ${z.toFixed(2)}px) rotateX(${rx.toFixed(2)}deg) rotateY(${ry.toFixed(2)}deg) scale(${s.toFixed(3)})`,
    filter: blur > 0.05 ? `blur(${blur.toFixed(2)}px)` : undefined,
    opacity: op,
  };
}

// Hand-drawn rough SVG rectangle path (from reel.html roughRect)
function roughRect(w: number, h: number, m: number, seed: number) {
  let sd = seed;
  const r = () => {
    sd = (sd * 1664525 + 1013904223) >>> 0;
    return sd / 4294967296 - 0.5;
  };
  const j = 6;
  const x0 = m + r() * j;
  const y0 = m + r() * j;
  const x1 = w - m + r() * j;
  const y1 = h - m + r() * j;
  const f2 = (n: number) => n.toFixed(1);
  return (
    `M${f2(x0 + 12)} ${f2(y0 + r() * 3)} ` +
    `C${f2(w * 0.35)} ${f2(y0 - 4 + r() * j)} ${f2(w * 0.7)} ${f2(y0 + 3 + r() * j)} ${f2(x1)} ${f2(y0 + r() * j)} ` +
    `C${f2(x1 + 3 + r() * j)} ${f2(h * 0.4)} ${f2(x1 - 2 + r() * j)} ${f2(h * 0.7)} ${f2(x1 + r() * j)} ${f2(y1)} ` +
    `C${f2(w * 0.66)} ${f2(y1 + 4 + r() * j)} ${f2(w * 0.3)} ${f2(y1 - 3 + r() * j)} ${f2(x0)} ${f2(y1 + r() * j)} ` +
    `C${f2(x0 - 3 + r() * j)} ${f2(h * 0.62)} ${f2(x0 + 3 + r() * j)} ${f2(h * 0.3)} ${f2(x0 - 4)} ${f2(y0 + 14)}`
  );
}

// ============================================================================
// GROUP TRANSITIONS & 180-DEGREE SHUTTER DIRECTIONAL SVG MOTION BLUR
// ============================================================================
const PD = 0.38; // whip pan duration (s)
const FD = 0.42; // focus pull duration (s)
const W_SPAN = VW + 120;

function panStep(tau: number) {
  const u = seg(tau, 0, PD);
  const v =
    u > 0 && u < 1
      ? (W_SPAN * (u < 0.5 ? 12 * u * u : 12 * (1 - u) * (1 - u))) / PD / 30
      : 0;
  return { x: W_SPAN * eio(u), v };
}

type TransKind = "none" | "pan" | "up" | "focus" | "iris" | "wipe";

function calcTin(kind: TransKind, tau: number) {
  if (kind === "pan") {
    const p = panStep(tau);
    return { dx: W_SPAN - p.x, dy: 0, bx: mb(p.v), by: 0, op: 1, s: 1 };
  }
  if (kind === "up") {
    const p = panStep(tau);
    const k = VH / W_SPAN;
    return {
      dx: 0,
      dy: (W_SPAN - p.x) * k,
      bx: 0,
      by: mb(p.v * k),
      op: 1,
      s: 1,
    };
  }
  if (kind === "focus") {
    const e = eo(seg(tau, 0, FD));
    return {
      dx: 0,
      dy: 0,
      bx: 16 * (1 - e),
      by: 16 * (1 - e),
      op: e,
      s: lerp(1.035, 1, e),
    };
  }
  if (kind === "wipe") {
    const u = eo(seg(tau, 0, 0.22));
    return {
      dx: 0,
      dy: (1 - u) * VH,
      bx: 0,
      by: mb((1 - u) * 90),
      op: 1,
      s: 1,
    };
  }
  return { dx: 0, dy: 0, bx: 0, by: 0, op: 1, s: 1 };
}

function calcTout(kind: TransKind, tau: number) {
  if (tau <= 0) return { dx: 0, dy: 0, bx: 0, by: 0, op: 1, s: 1 };
  if (kind === "pan") {
    const p = panStep(tau);
    return { dx: -p.x, dy: 0, bx: mb(p.v), by: 0, op: 1, s: 1 };
  }
  if (kind === "up") {
    const p = panStep(tau);
    const k = VH / W_SPAN;
    return { dx: 0, dy: -p.x * k, bx: 0, by: mb(p.v * k), op: 1, s: 1 };
  }
  if (kind === "focus") {
    const e = eio(seg(tau, 0, FD));
    return {
      dx: 0,
      dy: 0,
      bx: 16 * e,
      by: 16 * e,
      op: 1 - e,
      s: lerp(1, 0.975, e),
    };
  }
  return { dx: 0, dy: 0, bx: 0, by: 0, op: 1, s: 1 };
}

const MotionGroup: React.FC<{
  id: string;
  frame: number;
  startFrame: number;
  endFrame: number;
  inKind?: TransKind;
  outKind?: TransKind;
  extraBlur?: { bx: number; by: number };
  plate: string;
  zIndex?: number;
  children: React.ReactNode;
}> = ({
  id,
  frame,
  startFrame,
  endFrame,
  inKind = "none",
  outKind = "none",
  extraBlur = { bx: 0, by: 0 },
  plate,
  zIndex = 1,
  children,
}) => {
  const tailFrames =
    outKind === "pan" || outKind === "up"
      ? Math.ceil(PD * FPS)
      : outKind === "focus"
        ? Math.ceil(FD * FPS)
        : 0;
  if (frame < startFrame || frame >= endFrame + tailFrames) return null;

  const t = frame / FPS;
  const t0 = startFrame / FPS;
  const t1 = endFrame / FPS;

  const fi = calcTin(inKind, t - t0);
  const fo = calcTout(outKind, t - t1);

  const dx = fi.dx + fo.dx;
  const dy = fi.dy + fo.dy;
  const bx = Math.max(fi.bx, fo.bx, extraBlur.bx);
  const by = Math.max(fi.by, fo.by, extraBlur.by);
  const op = fi.op * fo.op;
  const s = fi.s * fo.s;

  // Halftone iris clip when inKind === "iris"
  const irisDur = 0.68;
  const rMax = 1480;
  const irisR =
    inKind === "iris" ? lerp(0, rMax, eio(seg(t - t0, 0, irisDur))) : rMax;

  const filterId = `mb-${id}`;
  const hasBlur = bx + by > 0.08;

  return (
    <>
      <svg width="0" height="0" style={{ position: "absolute" }}>
        <defs>
          <filter
            id={filterId}
            x="-15%"
            y="-15%"
            width="130%"
            height="130%"
            colorInterpolationFilters="sRGB"
          >
            <feGaussianBlur
              stdDeviation={`${bx.toFixed(2)} ${by.toFixed(2)}`}
            />
          </filter>
        </defs>
      </svg>
      <div
        style={{
          position: "absolute",
          inset: 0,
          width: VW,
          height: VH,
          backgroundColor: plate,
          overflow: "hidden",
          zIndex,
          transformOrigin: "960px 540px",
          transform: `translate(${dx.toFixed(2)}px, ${dy.toFixed(2)}px) scale(${s.toFixed(4)})`,
          opacity: op,
          filter: hasBlur ? `url(#${filterId})` : undefined,
          clipPath:
            inKind === "iris" && irisR < rMax
              ? `circle(${irisR.toFixed(1)}px at ${CX}px ${CY}px)`
              : undefined,
        }}
      >
        {children}
      </div>
      {inKind === "iris" && irisR < rMax && (
        <HalftoneIrisCanvas
          radius={irisR}
          band={130}
          opacity={1 - seg(irisR, rMax * 0.82, rMax)}
          zIndex={zIndex + 2}
        />
      )}
    </>
  );
};

// ============================================================================
// 2D CANVAS EFFECTS (Ported from kinetic.html & beats.html)
// ============================================================================

// 1. Halftone Dot-Ring Iris (kinetic.html iris())
const HalftoneIrisCanvas: React.FC<{
  radius: number;
  band?: number;
  opacity?: number;
  zIndex?: number;
}> = ({ radius, band = 130, opacity = 1, zIndex = 50 }) => {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);
    if (opacity <= 0) return;

    const p = 14;
    const a = 0.35;
    const ca = Math.cos(a);
    const sa = Math.sin(a);
    const Rm = radius + band;
    const n = Math.ceil(Rm / p) + 1;

    ctx.globalAlpha = opacity;
    ctx.fillStyle = C.amber;
    ctx.beginPath();
    for (let i = -n; i <= n; i++) {
      for (let j = -n; j <= n; j++) {
        const gx = i * p;
        const gy = j * p;
        const X = CX + gx * ca - gy * sa;
        const Y = CY + gx * sa + gy * ca;
        if (X < -12 || X > VW + 12 || Y < -12 || Y > VH + 12) continue;
        const d = Math.hypot(X - CX, Y - CY);
        if (d < radius || d > Rm) continue;
        const r = p * 0.46 * Math.pow(1 - (d - radius) / band, 0.8);
        if (r < 0.35) continue;
        ctx.moveTo(X + r, Y);
        ctx.arc(X, Y, r, 0, Math.PI * 2);
      }
    }
    ctx.fill();
    ctx.globalAlpha = 1;
  }, [radius, band, opacity]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex,
      }}
    />
  );
};

// 2. 165-Square Pixel Burst (kinetic.html burst())
const PixelBurstCanvas: React.FC<{
  t: number;
  t0: number;
  srcRect: { x: number; y: number; w: number; h: number };
  dstRect: { x: number; y: number; w: number; h: number };
}> = ({ t, t0, srcRect, dstRect }) => {
  const ref = useRef<HTMLCanvasElement>(null);
  const squares = useMemo(() => {
    let sd = 7919;
    const rn = () => {
      sd = (sd * 1664525 + 1013904223) >>> 0;
      return sd / 4294967296;
    };
    const palette: [string, number][] = [
      [C.amber, 1],
      [C.amber, 1],
      [C.amberSoft, 0.85],
      [C.silver, 0.9],
      [C.ivory, 0.65],
      [C.plumDeep, 0.9],
    ];
    return Array.from({ length: 165 }, () => ({
      u: rn(),
      v: rn(),
      a: rn() * Math.PI * 2,
      d: 160 + rn() * 420,
      s: 10 + rn() * 18,
      c: palette[Math.floor(rn() * palette.length)],
      dl: rn() * 0.1,
      tu: rn(),
      tv: rn(),
    }));
  }, []);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const g = seg(t, t0, t0 + 0.92);
    if (g <= 0 || g >= 1) return;
    const fade = 1 - seg(t, t0 + 0.64, t0 + 0.9);

    for (const q of squares) {
      const p1 = eo(seg(t, t0 + q.dl, t0 + q.dl + 0.36));
      const p2 = eio(seg(t, t0 + 0.34 + q.dl * 0.5, t0 + 0.76));
      if (p1 <= 0) continue;
      const sx = srcRect.x + q.u * srcRect.w;
      const sy = srcRect.y + q.v * srcRect.h;
      const ex = sx + Math.cos(q.a) * q.d;
      const ey = sy + Math.sin(q.a) * q.d * 0.75;
      const tx = dstRect.x + q.tu * dstRect.w;
      const ty = dstRect.y + q.tv * dstRect.h;
      const x = lerp(lerp(sx, ex, p1), tx, p2);
      const y = lerp(lerp(sy, ey, p1), ty, p2);
      const s = lerp(q.s, 11, p2);
      const al = eo(seg(t, t0 + q.dl, t0 + q.dl + 0.08)) * fade;
      ctx.globalAlpha = al * q.c[1];
      ctx.fillStyle = q.c[0];
      ctx.fillRect(x - s / 2, y - s / 2, s, s);
    }
    ctx.globalAlpha = 1;
  }, [t, t0, srcRect, dstRect, squares]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 40,
      }}
    />
  );
};

// 3. 170 Converging 3D Bokeh Particles (beats.html particles())
const ConvergingParticlesCanvas: React.FC<{
  t: number;
  convergeStart?: number;
}> = ({ t, convergeStart }) => {
  const ref = useRef<HTMLCanvasElement>(null);
  const pts = useMemo(() => {
    let s = 1337;
    const rnd = () => {
      s = (s * 1664525 + 1013904223) >>> 0;
      return s / 4294967296;
    };
    return Array.from({ length: 170 }, () => ({
      bx: rnd() * VW,
      by: rnd() * VH,
      r: 1.3 + rnd() * 3.8,
      d: 0.35 + rnd() * 0.65,
      ph: rnd() * Math.PI * 2,
      gold: rnd() < 0.28,
    }));
  }, []);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const hasCo = convergeStart !== undefined;
    const u = hasCo ? ei(seg(t, convergeStart, convergeStart + 1.0)) : 0;
    const fade = hasCo
      ? 1 - seg(t, convergeStart + 0.75, convergeStart + 1.35)
      : 1;
    if (fade <= 0) return;

    for (const p of pts) {
      let x = p.bx + 22 * p.d * Math.sin(0.35 * t + p.ph);
      let y = (((p.by - 16 * p.d * t) % VH) + VH) % VH;
      if (hasCo) {
        x = lerp(x, CX + (p.bx - CX) * 0.06, u);
        y = lerp(y, CY + (p.by - CY) * 0.06, u);
      }
      const a = (0.3 + 0.42 * p.d) * fade;
      const g = ctx.createRadialGradient(x, y, 0, x, y, p.r * 2.4);
      const col = p.gold ? "251,179,3" : "223,229,236";
      g.addColorStop(0, `rgba(${col},${a.toFixed(3)})`);
      g.addColorStop(1, `rgba(${col},0)`);
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(x, y, p.r * 2.4, 0, Math.PI * 2);
      ctx.fill();
    }
  }, [t, convergeStart, pts]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{ position: "absolute", inset: 0, pointerEvents: "none" }}
    />
  );
};

// ============================================================================
// KINETIC TYPOGRAPHY & WET-INK OKLAB COMPONENTS
// ============================================================================

const BuildLetters: React.FC<{
  text: string;
  t: number;
  t0?: number;
  step?: number;
  d?: number;
  color?: string;
}> = ({ text, t, t0 = 0.12, step = 0.048, d = 0.34, color }) => {
  return (
    <>
      {[...text].map((ch, i) => {
        const u = eo(seg(t, t0 + i * step, t0 + i * step + d));
        const blur = u < 0.999 ? 10 * (1 - u) : 0;
        return (
          <span
            key={i}
            style={{
              display: "inline-block",
              opacity: u,
              transform: `translateY(${lerp(32, 0, u).toFixed(2)}px)`,
              filter: blur > 0.05 ? `blur(${blur.toFixed(2)}px)` : undefined,
              color,
            }}
          >
            {ch === " " ? "\u00A0" : ch}
          </span>
        );
      })}
    </>
  );
};

const KineticWords: React.FC<{
  tokens: { text: string; accent?: boolean }[];
  t: number;
  t0?: number;
  step?: number;
  d?: number;
  inkColor: string;
  accentColor?: string;
}> = ({
  tokens,
  t,
  t0 = 0.12,
  step = 0.085,
  d = 0.36,
  inkColor,
  accentColor = C.amber,
}) => {
  return (
    <>
      {tokens.map((tok, i) => {
        const u = eo(seg(t, t0 + i * step, t0 + i * step + d));
        const blur = u < 0.999 ? 10 * (1 - u) : 0;
        return (
          <span
            key={i}
            style={{
              display: "inline-block",
              marginRight: "0.26em",
              opacity: u,
              transform: `translateY(${lerp(34, 0, u).toFixed(2)}px)`,
              filter: blur > 0.05 ? `blur(${blur.toFixed(2)}px)` : undefined,
              color: tok.accent ? accentColor : inkColor,
            }}
          >
            {tok.text}
          </span>
        );
      })}
    </>
  );
};

// Wet-ink oklab word stream (kinetic.html ink()): words arrive in #fbb303 amber and settle to ink
const WetInkText: React.FC<{
  text: string;
  t: number;
  t0: number;
  cps?: number;
  settle?: number;
  inkColor: string;
  accentColor?: string;
}> = ({
  text,
  t,
  t0,
  cps = 18,
  settle = 0.48,
  inkColor,
  accentColor = C.amber,
}) => {
  const words = useMemo(() => {
    const matches = text.match(/\S+\s*/g) || [];
    let offset = 0;
    return matches.map((w) => {
      const start = offset;
      offset += w.length;
      return { w, start };
    });
  }, [text]);

  const n = Math.floor(clamp((t - t0) * cps + 1e-6, 0, text.length));

  return (
    <>
      {words.map((item, idx) => {
        const c = clamp(n - item.start, 0, item.w.length);
        if (c <= 0) return null;
        const doneTime = t0 + (item.start + item.w.trimEnd().length) / cps;
        const p = seg(t, doneTime + 0.08, doneTime + 0.08 + settle);
        const accPct = Math.round((1 - p) * 100);
        return (
          <span
            key={idx}
            style={{
              whiteSpace: "pre",
              color: `color-mix(in oklab, ${accentColor} ${accPct}%, ${inkColor})`,
            }}
          >
            {item.w.slice(0, c)}
          </span>
        );
      })}
    </>
  );
};

// Simulated screen-space pointer (kinetic.html cursor())
interface CursorKey {
  t: number;
  x: number;
  y: number;
  drag?: boolean;
}
const SimulatedCursor: React.FC<{
  t: number;
  keys: CursorKey[];
  presses?: number[];
  show: [number, number];
  cam?: { cx: number; cy: number; m: number };
}> = ({ t, keys, presses = [], show, cam = { cx: CX, cy: CY, m: 1 } }) => {
  if (t < show[0] || t > show[1] + 0.25) return null;
  let a = keys[0];
  let b = keys[0];
  let idx = 0;
  if (t >= keys[keys.length - 1].t) {
    a = b = keys[keys.length - 1];
  } else if (t > keys[0].t) {
    for (let i = 0; i < keys.length - 1; i++) {
      if (t >= keys[i].t && t <= keys[i + 1].t) {
        a = keys[i];
        b = keys[i + 1];
        idx = i;
        break;
      }
    }
  }
  const u = b.t > a.t ? eio(seg(t, a.t, b.t)) : 1;
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const L = Math.hypot(dx, dy) || 1;
  const arc = b.drag
    ? 0
    : Math.sin(Math.PI * u) * Math.min(40, L * 0.1) * (idx % 2 ? 1 : -1);
  const wx = lerp(a.x, b.x, u) - (dy / L) * arc;
  const wy = lerp(a.y, b.y, u) + (dx / L) * arc;
  const s = w2s(cam, wx, wy);

  let pr = 0;
  for (const q of presses) pr = Math.max(pr, press(t, q));
  const op =
    seg(t, show[0], show[0] + 0.16) * (1 - seg(t, show[1], show[1] + 0.16));

  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        top: 0,
        width: 58,
        height: 58,
        pointerEvents: "none",
        zIndex: 80,
        transformOrigin: "6px 4px",
        transform: `translate(${(s.x - 6).toFixed(2)}px, ${(s.y - 4).toFixed(2)}px) scale(${(1 - 0.16 * pr).toFixed(3)})`,
        opacity: op,
      }}
    >
      <svg
        viewBox="0 0 24 24"
        style={{
          width: 58,
          height: 58,
          filter: "drop-shadow(0 6px 12px rgba(0,0,0,0.28))",
        }}
      >
        <path
          d="M4 2.5v17.2l4.6-4.3 2.9 6.6 3-1.3-2.9-6.5h6.3z"
          fill={C.plum}
          stroke={C.white}
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
};

// ============================================================================
// REAL SHIPPED SAYWHEN HUD PILL & PREVIEW CARD (from src/hud.css)
// ============================================================================
const ShippedHudPill: React.FC<{
  state: "IDLE" | "ARMED" | "LISTENING" | "TRANSCRIBING" | "PREVIEW";
  label: string;
  sublabel?: string;
  scale?: number;
  rms?: number;
}> = ({ state, label, sublabel, scale = 1, rms = 0.5 }) => {
  const accent =
    state === "ARMED"
      ? C.armed
      : state === "LISTENING"
        ? C.amber
        : state === "TRANSCRIBING"
          ? C.transcribing
          : state === "PREVIEW"
            ? C.amberSoft
            : C.silver;

  return (
    <div
      style={{
        position: "relative",
        display: "inline-flex",
        alignItems: "center",
        gap: 14,
        height: 56,
        padding: "0 22px 0 16px",
        borderRadius: 999,
        background: `linear-gradient(180deg, rgba(51, 10, 43, 0.96), rgba(34, 6, 28, 0.96))`,
        border: `1.5px solid color-mix(in srgb, ${accent} 48%, rgba(223,229,236,0.16))`,
        boxShadow: `0 22px 54px rgba(14, 2, 12, 0.52), 0 0 32px color-mix(in srgb, ${accent} 28%, transparent), inset 0 1px 0 rgba(255,255,255,0.14)`,
        transform: `scale(${scale})`,
        fontFamily: F.sans,
        color: C.ivory,
      }}
    >
      <Img
        src={staticFile("logo.png")}
        style={{
          width: 28,
          height: 28,
          borderRadius: 8,
          boxShadow: "0 0 0 1px rgba(255,255,255,0.14)",
        }}
      />
      <span
        style={{
          width: 10,
          height: 10,
          borderRadius: "50%",
          background: accent,
          boxShadow: `0 0 12px ${accent}`,
        }}
      />
      <span
        style={{
          fontSize: 19,
          fontWeight: 600,
          letterSpacing: "-0.01em",
          whiteSpace: "nowrap",
        }}
      >
        {label}
      </span>
      {state === "LISTENING" && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 3,
            height: 22,
            marginLeft: 4,
          }}
        >
          {[0.45, 0.85, 1.0, 0.7, 0.92, 0.55].map((m, i) => (
            <span
              key={i}
              style={{
                width: 3.5,
                height: Math.max(6, Math.round(22 * m * rms)),
                borderRadius: 99,
                background: C.amber,
              }}
            />
          ))}
        </div>
      )}
      {sublabel && (
        <span
          style={{
            fontFamily: F.mono,
            fontSize: 14,
            fontWeight: 500,
            color: C.mutedDark,
            paddingLeft: 8,
            borderLeft: "1px solid rgba(223,229,236,0.14)",
            whiteSpace: "nowrap",
          }}
        >
          {sublabel}
        </span>
      )}
    </div>
  );
};

// ============================================================================
// MAIN 60-SECOND (1800-FRAME) FILM COMPOSITION
// ============================================================================
export const SaywhenDemo: React.FC = () => {
  const frame = useCurrentFrame();
  useVideoConfig();
  const t = frame / FPS;
  const rms = AUDIO_RMS[frame] ?? 0.25;

  // --------------------------------------------------------------------------
  // BEAT 1 (f0..118, 0.0s..3.93s): 3D Keycap Shatter -> "No hotkey."
  // Voiceover 01: "Voice input. Without a hotkey."
  // --------------------------------------------------------------------------
  const T1 = t;
  const b1Switch = 1.55; // at 1.55s, problem cards shatter in 3D & whip left
  const b1Whip = eo(seg(T1, b1Switch, b1Switch + 0.36));
  const b1WhipBlur = mb(
    b1Whip > 0 && b1Whip < 1 ? (VW * (1 - b1Whip) * 3.2) / 30 : 0,
  );

  // --------------------------------------------------------------------------
  // BEAT 2 (f118..224, 3.93s..7.47s): Halftone Iris -> 3D Parametric Ribbon + "Look. Talk. Sent."
  // Voiceover 02: "Look. Talk. Sent."
  // --------------------------------------------------------------------------
  const T2 = (frame - 118) / FPS;
  const b2Step: 0 | 1 | 2 = T2 < 1.0 ? 0 : T2 < 1.95 ? 1 : 2;

  // --------------------------------------------------------------------------
  // BEAT 3 (f224..314, 7.47s..10.47s): 3D Biometric Face Mesh + 3D Camera Rig
  // Voiceover 03: "It can see you looking at the screen."
  // --------------------------------------------------------------------------
  const T3 = (frame - 224) / FPS;
  const cam3D_B3 = evalCam3D(T3, [
    { t: 0, x: -40, y: 12, z: -40, rx: 3, ry: -4 },
    { t: 0.9, x: 10, y: -8, z: 75, rx: 1, ry: 1.5 },
    { t: 3.0, x: 20, y: -10, z: 95, rx: 0.5, ry: 2 },
  ]);

  // --------------------------------------------------------------------------
  // BEAT 4 (f314..492, 10.47s..16.40s): 3D Acoustic Terrain + Caret-Tracking Giant Ticker
  // Voiceover 04: "So you just talk. No button to hold."
  // --------------------------------------------------------------------------
  const T4 = (frame - 314) / FPS;
  const PROMPT_4 =
    "run cargo test workspace release and fix the failing assertion";
  const TS4 = 0.72;
  const CPS4 = 15.5;
  const TE4 = TS4 + PROMPT_4.length / CPS4;
  const MT4 = 1.42;
  const cam4Fn = (tau: number) => {
    const typedChars = clamp((tau - TS4) * CPS4, 0, PROMPT_4.length);
    const pxWidth = typedChars * 23.5;
    const baseCx = 760;
    const trackCx = clamp(
      baseCx + Math.max(0, pxWidth - 360) * 0.42,
      760,
      1140,
    );
    if (tau < TS4) {
      return evalCam2D(tau, [
        { t: 0, cx: 960, cy: 540, m: 1.0 },
        { t: 0.42, cx: 960, cy: 540, m: 1.0 },
        { t: TS4, cx: baseCx, cy: 540, m: MT4 },
      ]);
    }
    if (tau < TE4) {
      return { cx: trackCx, cy: 540, m: MT4 };
    }
    return evalCam2D(tau, [
      { t: TE4, cx: trackCx, cy: 540, m: MT4 },
      { t: TE4 + 0.55, cx: 960, cy: 540, m: 1.06 },
      { t: 5.9, cx: 960, cy: 540, m: 1.08 },
    ]);
  };
  const cam4 = cam4Fn(T4);
  const cam4MotionBlur = cam2DBlur(cam4Fn, T4);

  // --------------------------------------------------------------------------
  // BEAT 5 (f492..600, 16.40s..20.00s): 3D Rack Focus + Real Preview HUD Card
  // Voiceover 05: "It writes the real command, and lets you check."
  // --------------------------------------------------------------------------
  const T5 = (frame - 492) / FPS;
  const cam3D_B5 = evalCam3D(T5, [
    { t: 0, x: 0, y: 10, z: -20, rx: 3, ry: -3 },
    { t: 1.1, x: 20, y: -14, z: 185, rx: 1, ry: 1.5 },
    { t: 3.6, x: 28, y: -18, z: 220, rx: 0.5, ry: 2 },
  ]);
  const previewProgress = clamp(1 - seg(T5, 0.45, 3.35), 0, 1);
  const previewSecLeft = (previewProgress * 0.9).toFixed(1);

  // --------------------------------------------------------------------------
  // BEAT 6 (f600..714, 20.00s..23.80s): Hyperspace Warp + 165-Square Burst -> Terminal
  // Voiceover 06: "Then it types it in, and presses enter."
  // --------------------------------------------------------------------------
  const T6 = (frame - 600) / FPS;
  const PT6 = 0.16;

  // --------------------------------------------------------------------------
  // BEAT 7 (f714..982, 23.80s..32.73s): Developer Cleanup Compiler + Laser Scanner
  // Voiceover 07: "You speak like a person. It writes like a developer."
  // --------------------------------------------------------------------------
  const T7 = (frame - 714) / FPS;

  // --------------------------------------------------------------------------
  // BEAT 8 (f982..1124, 32.73s..37.47s): 3D Camera Rig — Pause. Nod. Or say send it.
  // Voiceover 08: "Pause. Nod. Or say send it."
  // --------------------------------------------------------------------------
  const T8 = (frame - 982) / FPS;
  const cam3D_B8 = evalCam3D(T8, [
    { t: 0, x: -70, y: 0, z: 45, rx: 1.5, ry: -2.5 },
    { t: 1.4, x: 0, y: -6, z: 65, rx: 1, ry: 0 },
    { t: 2.8, x: 70, y: 0, z: 45, rx: 1.5, ry: 2.5 },
    { t: 4.5, x: 0, y: 0, z: 25, rx: 1, ry: 0 },
  ]);

  // --------------------------------------------------------------------------
  // BEAT 9 (f1124..1280, 37.47s..42.67s): Time-Freeze Shatter & Focus-Pull Cancel
  // Voiceover 09: "And you can always stop it."
  // --------------------------------------------------------------------------
  const T9 = (frame - 1124) / FPS;
  const cancelTrigger = 1.15;
  const cancelProgress = eo(seg(T9, cancelTrigger, cancelTrigger + 0.38));

  // --------------------------------------------------------------------------
  // BEAT 10 (f1280..1528, 42.67s..50.93s): Halftone Iris -> 3D Hex Force-Field Guard
  // Voiceover 10: "Risky commands never press enter for you."
  // --------------------------------------------------------------------------
  const T10 = (frame - 1280) / FPS;
  const cam10Fn = (tau: number) =>
    evalCam2D(tau, [
      { t: 0, cx: 960, cy: 540, m: 0.95 },
      { t: 0.9, cx: 960, cy: 540, m: 1.0 },
      { t: 1.75, cx: 960, cy: 548, m: 1.12 },
      { t: 5.5, cx: 960, cy: 548, m: 1.15 },
      { t: 6.4, cx: 960, cy: 540, m: 1.0 },
    ]);
  const cam10 = cam10Fn(T10);
  const cam10Blur = cam2DBlur(cam10Fn, T10);

  // --------------------------------------------------------------------------
  // BEAT 11 (f1528..1676, 50.93s..55.87s): High-Speed Vertical Slot Reel (kinetic.html G4)
  // Voiceover 11: "Nothing leaves this computer."
  // --------------------------------------------------------------------------
  const T11 = (frame - 1528) / FPS;
  const REEL_ITEMS = [
    { label: "MediaPipe 15 FPS.", acc: false },
    { label: "Silero VAD v5.", acc: false },
    { label: "Whisper Metal GPU.", acc: false },
    { label: "Zero Cloud Calls.", acc: false },
    { label: "Zero Telemetry.", acc: false },
    { label: "Cursor & Claude Code.", acc: false },
    { label: "Ghostty & Zed.", acc: false },
    { label: "100% On-Device.", acc: true },
  ];
  const RL = REEL_ITEMS.length - 1;
  const REH = 166;
  const RV = 5.4;
  const RT0 = 0.34;
  const RT1 = RT0 + 0.22;
  const RP1 = 0.5 * RV * (RT1 - RT0);
  const RT2 = RT1 + (RL - 1 - RP1) / RV;
  const ROV = 0.26;
  const RD = (2 * (1 + ROV)) / RV;
  const RT3 = RT2 + RD;
  const RT4 = RT3 + 0.28;
  const reelPos = (tau: number) => {
    if (tau < RT0) return 0;
    if (tau < RT1) return 0.5 * (RV / (RT1 - RT0)) * Math.pow(tau - RT0, 2);
    if (tau < RT2) return RP1 + RV * (tau - RT1);
    if (tau < RT3) {
      const x = tau - RT2;
      return RL - 1 + RV * x - 0.5 * (RV / RD) * x * x;
    }
    return lerp(RL + ROV, RL, eio(seg(tau, RT3, RT4)));
  };
  const rP = reelPos(T11);
  const rVel = Math.abs(reelPos(T11 + DT) - rP) * REH;
  const rLand =
    seg(T11, RT4, RT4 + 0.12) * (1 - seg(T11, RT4 + 0.12, RT4 + 0.42));

  // --------------------------------------------------------------------------
  // BEAT 12 (f1676..1800, 55.87s..60.00s): Fibonacci Golden Vortex + End Card Lockup
  // Voiceover 12: "Look. Talk. Sent."
  // --------------------------------------------------------------------------
  const T12 = (frame - 1676) / FPS;

  return (
    <AbsoluteFill style={{ backgroundColor: C.plum, overflow: "hidden" }}>
      {/* Master Studio Soundtrack (Music + 12 Ava VO clips + CC0 SFX) */}
      <Audio src={staticFile("demo-soundtrack.wav")} />

      {/* ====================================================================
          BEAT 1 (0..118): 3D Keycap Shatter -> "No hotkey." Whip Pan
         ==================================================================== */}
      <MotionGroup
        id="B1"
        frame={frame}
        startFrame={0}
        endFrame={118}
        inKind="none"
        outKind="focus"
        extraBlur={{ bx: b1WhipBlur, by: 0 }}
        plate={C.plum}
        zIndex={1}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <ShatteringKeycaps3DCanvas t={T1} shatterTime={b1Switch} />

        {/* Phase 1A: Three 3D Problem Cards crossed out & shattered */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            transform: `translateX(${(-b1Whip * VW).toFixed(1)}px)`,
          }}
        >
          <div
            style={{
              position: "absolute",
              left: 110,
              top: 88,
              fontFamily: F.mono,
              fontSize: 20,
              fontWeight: 600,
              color: C.amber,
              letterSpacing: "0.08em",
              textTransform: "uppercase",
            }}
          >
            THE OLD WAY · PUSH-TO-TALK FRICTION
          </div>
          <div
            style={{
              position: "absolute",
              left: 106,
              top: 132,
              fontFamily: F.sans,
              fontSize: 66,
              fontWeight: 800,
              letterSpacing: "-0.045em",
              color: C.ivory,
            }}
          >
            Why does voice still need a{" "}
            <span style={{ color: C.amber }}>chord</span>?
          </div>

          {[
            {
              title: "Hold Fn + Space",
              sub: "Finger cramps mid-sentence",
              note: "dropped early!",
              x: 110,
              y: 340,
              rot: -2.2,
              t0: 0.0,
              seed: 101,
            },
            {
              title: "Press ⌥ + R to toggle",
              sub: "Forgot to toggle off · recorded 40s of breathing",
              note: "wrong window focused",
              x: 710,
              y: 360,
              rot: 1.4,
              t0: 0.22,
              seed: 202,
            },
            {
              title: "Paste, then press Enter",
              sub: "Reach back for the keyboard anyway",
              note: "still using hands",
              x: 1310,
              y: 336,
              rot: -1.6,
              t0: 0.44,
              seed: 303,
            },
          ].map((card, i) => {
            const u = i === 0 ? 1 : eo(seg(T1, card.t0, card.t0 + 0.28));
            const penU = eio(seg(T1, 0.5 + i * 0.2, 0.82 + i * 0.2));
            return (
              <div
                key={i}
                style={{
                  position: "absolute",
                  left: card.x,
                  top: card.y,
                  width: 500,
                  height: 310,
                  borderRadius: 20,
                  background: C.white,
                  color: C.plumInk,
                  padding: "34px 36px",
                  boxShadow: "0 34px 80px rgba(0,0,0,0.55)",
                  transform: `translateY(${lerp(28, 0, u)}px) rotate(${card.rot}deg) scale(${lerp(0.92, 1, u)})`,
                  opacity: u,
                }}
              >
                <div
                  style={{
                    fontFamily: F.mono,
                    fontSize: 15,
                    fontWeight: 600,
                    color: C.mutedLight,
                    textTransform: "uppercase",
                    letterSpacing: "0.08em",
                  }}
                >
                  ATTEMPT 0{i + 1}
                </div>
                <div
                  style={{
                    marginTop: 18,
                    fontFamily: F.mono,
                    fontSize: 34,
                    fontWeight: 700,
                    letterSpacing: "-0.03em",
                    padding: "14px 20px",
                    borderRadius: 12,
                    background: C.ivory,
                    border: `1px solid ${C.borderLight}`,
                  }}
                >
                  {card.title}
                </div>
                <div
                  style={{
                    marginTop: 18,
                    fontFamily: F.sans,
                    fontSize: 20,
                    fontWeight: 500,
                    color: C.mutedLight,
                    lineHeight: 1.4,
                  }}
                >
                  {card.sub}
                </div>
                <svg
                  viewBox="0 0 532 342"
                  style={{
                    position: "absolute",
                    left: -16,
                    top: -16,
                    width: 532,
                    height: 342,
                    pointerEvents: "none",
                    overflow: "visible",
                  }}
                >
                  <path
                    d={roughRect(532, 342, 22, card.seed)}
                    fill="none"
                    stroke={C.danger}
                    strokeWidth={5}
                    strokeLinecap="round"
                    pathLength={1}
                    strokeDasharray="1 1"
                    strokeDashoffset={1 - penU}
                    style={{ opacity: penU > 0.01 ? 1 : 0 }}
                  />
                </svg>
                <div
                  style={{
                    position: "absolute",
                    right: 24,
                    bottom: -24,
                    padding: "6px 16px",
                    borderRadius: 99,
                    background: C.danger,
                    color: C.white,
                    fontFamily: F.mono,
                    fontSize: 16,
                    fontWeight: 700,
                    transform: `rotate(-4deg) scale(${lerp(0.6, 1, bo(penU))})`,
                    opacity: penU,
                  }}
                >
                  ✕ {card.note}
                </div>
              </div>
            );
          })}
        </div>

        {/* Phase 1B: Whip-pan in of "No hotkey." at 1.55s */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            transform: `translateX(${((1 - b1Whip) * VW).toFixed(1)}px)`,
          }}
        >
          <div
            style={{
              fontFamily: F.mono,
              fontSize: 20,
              fontWeight: 600,
              letterSpacing: "0.14em",
              color: C.amber,
              textTransform: "uppercase",
              marginBottom: 20,
            }}
          >
            INTRODUCING SAYWHEN
          </div>
          <div
            style={{
              fontFamily: F.serif,
              fontSize: 196,
              lineHeight: 0.96,
              letterSpacing: "-0.035em",
              color: C.ivory,
              textShadow: `0 0 ${Math.round(40 + rms * 50)}px rgba(251, 179, 3, 0.35)`,
            }}
          >
            <BuildLetters
              text="No hotkey."
              t={T1}
              t0={b1Switch + 0.1}
              step={0.04}
            />
          </div>
          <div
            style={{
              marginTop: 28,
              fontFamily: F.sans,
              fontSize: 44,
              fontWeight: 600,
              letterSpacing: "-0.025em",
              color: C.mutedDark,
            }}
          >
            <KineticWords
              t={T1}
              t0={b1Switch + 0.48}
              step={0.07}
              inkColor={C.mutedDark}
              accentColor={C.amber}
              tokens={[
                { text: "Voice" },
                { text: "input" },
                { text: "that" },
                { text: "watches", accent: true },
                { text: "when" },
                { text: "you're" },
                { text: "ready." },
              ]}
            />
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 2 (118..224): Halftone Iris -> 3D Parametric Ribbon + "Look. Talk. Sent."
         ==================================================================== */}
      <MotionGroup
        id="B2"
        frame={frame}
        startFrame={118}
        endFrame={224}
        inKind="iris"
        outKind="up"
        plate={C.ivory}
        zIndex={2}
      >
        <HorizonGrid3DCanvas frame={frame} lightPlate />
        {/* 3D Parametric S-Ribbon Sculpture morphing in 3D space behind the typography */}
        <ParametricRibbon3DCanvas t={T2} mode={b2Step} rms={rms} />

        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "baseline",
              gap: 54,
              fontFamily: F.serif,
              fontSize: 182,
              lineHeight: 1,
              letterSpacing: "-0.035em",
              textShadow: "0 18px 48px rgba(246, 238, 244, 0.92)",
            }}
          >
            {[
              { word: "Look.", t0: 0.12 },
              { word: "Talk.", t0: 1.0 },
              { word: "Sent.", t0: 1.92 },
            ].map((item, i) => {
              const u = eo(seg(T2, item.t0, item.t0 + 0.34));
              const settle = seg(T2, item.t0 + 0.25, item.t0 + 0.85);
              const popScale = lerp(0.84, 1, bo(u));
              return (
                <span
                  key={i}
                  style={{
                    display: "inline-block",
                    opacity: u,
                    transform: `translateY(${lerp(42, 0, u).toFixed(1)}px) scale(${popScale.toFixed(3)})`,
                    filter:
                      u < 0.99
                        ? `blur(${(10 * (1 - u)).toFixed(1)}px)`
                        : undefined,
                    color: `color-mix(in oklab, ${C.amber} ${Math.round((1 - settle) * 100)}%, ${C.plum})`,
                  }}
                >
                  {item.word}
                </span>
              );
            })}
          </div>

          <div style={{ marginTop: 64 }}>
            {b2Step === 0 && (
              <ShippedHudPill
                state="ARMED"
                label="Ready · facing screen"
                sublabel="Yaw -1.2° · Pitch +0.8°"
                scale={1.38}
              />
            )}
            {b2Step === 1 && (
              <ShippedHudPill
                state="LISTENING"
                label="Listening..."
                sublabel="Silero VAD v5 · 16 kHz"
                scale={1.38}
                rms={0.65 + 0.35 * Math.sin(T2 * 18)}
              />
            )}
            {b2Step === 2 && (
              <ShippedHudPill
                state="PREVIEW"
                label="Injected & Sent ↵"
                sublabel="0.9s preview verified"
                scale={1.38}
              />
            )}
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 3 (224..314): Procedural 3D 478-Point Biometric Face Mesh + 3D Rig
         ==================================================================== */}
      <MotionGroup
        id="B3"
        frame={frame}
        startFrame={224}
        endFrame={314}
        inKind="up"
        outKind="pan"
        plate={C.plum}
        zIndex={3}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <ConvergingParticlesCanvas t={T3} convergeStart={0.2} />
        {/* Procedural 3D Face Mesh & Volumetric Gaze Frustum on Left Stage */}
        <BiometricFaceMesh3DCanvas t={T3} rms={rms} />

        <div
          style={{
            position: "absolute",
            inset: 0,
            perspective: 1500,
            perspectiveOrigin: "50% 46%",
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 0,
              transformOrigin: "50% 50%",
              transformStyle: "preserve-3d",
              ...cam3D_B3,
            }}
          >
            {/* Background plane (z=0): Real saywhen Stage 3 Multimodal window */}
            <div
              style={{
                position: "absolute",
                left: 820,
                top: 90,
                width: 980,
                height: 680,
                borderRadius: 24,
                overflow: "hidden",
                background: C.plumDeep,
                border: `1px solid ${C.borderDark}`,
                boxShadow: "0 44px 110px rgba(0,0,0,0.65)",
                ...p3Style(T3, 0.05, 0.55, {
                  z: 0,
                  focus: T3 > 0.75 ? 160 : 0,
                  s: 0.94,
                }),
              }}
            >
              <Img
                src={staticFile("screens/stage3-multimodal-listening.png")}
                style={{ width: "100%", height: "100%", objectFit: "cover" }}
              />
            </div>

            {/* Foreground plane (z=160): Sharp 3D Vision Gate Telemetry Card */}
            <div
              style={{
                position: "absolute",
                left: 920,
                top: 380,
                width: 780,
                borderRadius: 26,
                background: C.white,
                color: C.plumInk,
                padding: "36px 40px",
                boxShadow:
                  "0 50px 120px rgba(10, 2, 8, 0.65), 0 0 0 1px rgba(255,255,255,0.9)",
                ...p3Style(T3, 0.32, 0.58, {
                  z: 160,
                  focus: 160,
                  s: 1.0,
                }),
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <span
                  style={{
                    fontFamily: F.mono,
                    fontSize: 16,
                    fontWeight: 700,
                    color: C.mutedLight,
                    letterSpacing: "0.08em",
                  }}
                >
                  MEDIAPIPE 3D FACE TOPOLOGY · 15 FPS
                </span>
                <span
                  style={{
                    padding: "6px 14px",
                    borderRadius: 99,
                    background: C.amber,
                    color: C.plumInk,
                    fontFamily: F.mono,
                    fontSize: 15,
                    fontWeight: 700,
                  }}
                >
                  ● GAZE LOCKED · ARMED
                </span>
              </div>

              <div
                style={{
                  marginTop: 18,
                  fontFamily: F.sans,
                  fontSize: 40,
                  fontWeight: 800,
                  letterSpacing: "-0.035em",
                  lineHeight: 1.1,
                }}
              >
                Attention replaces the hotkey.
              </div>

              <div
                style={{
                  display: "flex",
                  gap: 16,
                  marginTop: 26,
                }}
              >
                {[
                  { k: "Head Yaw", v: "-1.4°", sub: "Facing display" },
                  { k: "Lip Aperture", v: "0.042", sub: "Motion active" },
                  { k: "Pre-Roll Ring", v: "500 ms", sub: "Zero clipped words" },
                ].map((stat, idx) => (
                  <div
                    key={idx}
                    style={{
                      flex: 1,
                      padding: "18px 20px",
                      borderRadius: 16,
                      background: C.ivory,
                      border: `1px solid ${C.borderLight}`,
                    }}
                  >
                    <div
                      style={{
                        fontFamily: F.sans,
                        fontSize: 15,
                        fontWeight: 600,
                        color: C.mutedLight,
                      }}
                    >
                      {stat.k}
                    </div>
                    <div
                      style={{
                        marginTop: 6,
                        fontFamily: F.mono,
                        fontSize: 30,
                        fontWeight: 700,
                        color: C.plumInk,
                      }}
                    >
                      {stat.v}
                    </div>
                    <div
                      style={{
                        marginTop: 4,
                        fontFamily: F.sans,
                        fontSize: 14,
                        fontWeight: 500,
                        color: C.mutedLight,
                      }}
                    >
                      {stat.sub}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 4 (314..492): 3D Acoustic Frequency Terrain + Caret-Tracking Giant Ticker
         ==================================================================== */}
      <MotionGroup
        id="B4"
        frame={frame}
        startFrame={314}
        endFrame={492}
        inKind="pan"
        outKind="focus"
        extraBlur={cam4MotionBlur}
        plate={C.ivory}
        zIndex={4}
      >
        <HorizonGrid3DCanvas frame={frame} lightPlate />
        {/* 26-Ridge 3D Frequency Waterfall Landscape across lower stage */}
        <AcousticTerrain3DCanvas frame={frame} />

        <div
          style={{
            position: "absolute",
            inset: 0,
            WebkitMaskImage:
              "linear-gradient(to right, transparent 0%, #000 6%, #000 94%, transparent 100%)",
          }}
        >
          <div
            style={{
              position: "absolute",
              left: 0,
              top: 0,
              width: VW,
              height: VH,
              transformOrigin: "0 0",
              transform: `translate(${CX}px, ${CY}px) scale(${cam4.m.toFixed(4)}) translate(${(-cam4.cx).toFixed(2)}px, ${(-cam4.cy).toFixed(2)}px)`,
            }}
          >
            <div
              style={{
                position: "absolute",
                left: 360,
                top: 320,
                fontFamily: F.mono,
                fontSize: 20,
                fontWeight: 700,
                color: C.mutedLight,
                letterSpacing: "0.08em",
                opacity: eo(seg(T4, 0.2, 0.5)),
              }}
            >
              JUST SPEAK · LIVE VOICE STREAM (16 KHZ + SILERO VAD V5)
            </div>

            {(() => {
              const morphU = eio(seg(T4, 0.12, 0.54));
              const w = lerp(240, 1520, morphU);
              const h = lerp(94, 196, morphU);
              const insideOp = eo(seg(T4, 0.42, 0.62));
              return (
                <div
                  style={{
                    position: "absolute",
                    left: 960 - w / 2,
                    top: 500 - h / 2,
                    width: w,
                    height: h,
                    borderRadius: 34,
                    background: C.white,
                    border: `2px solid ${C.borderLight}`,
                    boxShadow: "0 34px 80px rgba(34, 6, 28, 0.14)",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      position: "absolute",
                      left: 40,
                      top: 26,
                      display: "flex",
                      alignItems: "center",
                      gap: 12,
                      opacity: insideOp,
                    }}
                  >
                    <span
                      style={{
                        width: 11,
                        height: 11,
                        borderRadius: "50%",
                        background: C.amber,
                        boxShadow: `0 0 12px ${C.amber}`,
                      }}
                    />
                    <span
                      style={{
                        fontFamily: F.sans,
                        fontSize: 20,
                        fontWeight: 700,
                        color: C.plumInk,
                      }}
                    >
                      saywhen · Listening
                    </span>
                    <span
                      style={{
                        fontFamily: F.mono,
                        fontSize: 16,
                        color: C.mutedLight,
                        marginLeft: 10,
                      }}
                    >
                      No hotkey held
                    </span>
                  </div>

                  <div
                    style={{
                      position: "absolute",
                      left: 40,
                      top: 88,
                      height: 72,
                      display: "flex",
                      alignItems: "center",
                      fontFamily: F.sans,
                      fontSize: 44,
                      fontWeight: 700,
                      letterSpacing: "-0.025em",
                      whiteSpace: "nowrap",
                      opacity: insideOp,
                    }}
                  >
                    <WetInkText
                      text={PROMPT_4}
                      t={T4}
                      t0={TS4}
                      cps={CPS4}
                      settle={0.5}
                      inkColor={C.plumInk}
                      accentColor={C.amber}
                    />
                    <span
                      style={{
                        display: "inline-block",
                        width: 4,
                        height: "1.02em",
                        marginLeft: 3,
                        background: C.amber,
                        borderRadius: 2,
                      }}
                    />
                  </div>
                </div>
              );
            })()}
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 5 (492..600): 3D Rack Focus + Shipped Cancelable Preview HUD Card
         ==================================================================== */}
      <MotionGroup
        id="B5"
        frame={frame}
        startFrame={492}
        endFrame={600}
        inKind="focus"
        outKind="none"
        plate={C.plum}
        zIndex={5}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <div
          style={{
            position: "absolute",
            inset: 0,
            perspective: 1500,
            perspectiveOrigin: "50% 46%",
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 0,
              transformOrigin: "50% 50%",
              transformStyle: "preserve-3d",
              ...cam3D_B5,
            }}
          >
            {/* Background plane (z=-30): Real Stage 4 Cleanup & Preview screenshot */}
            <div
              style={{
                position: "absolute",
                left: 210,
                top: 90,
                width: 1500,
                height: 860,
                borderRadius: 22,
                overflow: "hidden",
                border: `1px solid ${C.borderDark}`,
                boxShadow: "0 40px 100px rgba(0,0,0,0.6)",
                ...p3Style(T5, 0, 0.5, {
                  z: -30,
                  focus: 220,
                  s: 0.94,
                  op: 0.72,
                }),
              }}
            >
              <Img
                src={staticFile("screens/stage4-cleanup-preview.png")}
                style={{ width: "100%", height: "100%", objectFit: "cover" }}
              />
            </div>

            {/* Foreground plane (z=220): Exact shipped Preview HUD Card (src/hud.css) */}
            <div
              style={{
                position: "absolute",
                left: 440,
                top: 290,
                width: 1040,
                borderRadius: 26,
                background:
                  "linear-gradient(180deg, rgba(51, 10, 43, 0.98), rgba(34, 6, 28, 0.98))",
                border: `2px solid color-mix(in srgb, ${C.amber} 55%, rgba(223,229,236,0.2))`,
                boxShadow: `0 50px 120px rgba(0, 0, 0, 0.78), 0 0 ${Math.round(50 + rms * 55)}px rgba(251, 179, 3, 0.32)`,
                padding: "34px 40px 42px",
                overflow: "hidden",
                ...p3Style(T5, 0.15, 0.55, {
                  z: 220,
                  focus: 220,
                  s: 1.08,
                }),
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  fontFamily: F.sans,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <span
                    style={{
                      padding: "6px 14px",
                      borderRadius: 99,
                      background: "rgba(251, 179, 3, 0.18)",
                      border: `1px solid ${C.amber}`,
                      color: C.amber,
                      fontFamily: F.mono,
                      fontSize: 15,
                      fontWeight: 700,
                    }}
                  >
                    PREVIEW · AUTO-ENTER IN {previewSecLeft}s
                  </span>
                  <span
                    style={{
                      fontFamily: F.mono,
                      fontSize: 15,
                      color: C.mutedDark,
                    }}
                  >
                    Target: Ghostty (dev.mitchellh.ghostty)
                  </span>
                </div>
                <span
                  style={{
                    padding: "6px 14px",
                    borderRadius: 10,
                    background: "rgba(223, 229, 236, 0.1)",
                    fontFamily: F.mono,
                    fontSize: 15,
                    color: C.silver,
                  }}
                >
                  Esc or shake head to cancel
                </span>
              </div>

              <div
                style={{
                  marginTop: 26,
                  padding: "24px 28px",
                  borderRadius: 18,
                  background: "rgba(18, 2, 14, 0.72)",
                  border: `1px solid ${C.borderDark}`,
                  fontFamily: F.mono,
                  fontSize: 34,
                  fontWeight: 600,
                  color: C.ivory,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <span>
                  <span style={{ color: C.amber }}>$ </span>
                  cargo test --workspace --release
                </span>
                <span
                  style={{
                    padding: "8px 18px",
                    borderRadius: 12,
                    background: C.amber,
                    color: C.plumInk,
                    fontSize: 20,
                    fontWeight: 800,
                  }}
                >
                  Enter ↵
                </span>
              </div>

              <div
                style={{
                  position: "absolute",
                  left: 0,
                  right: 0,
                  bottom: 0,
                  height: 7,
                  background: "rgba(223, 229, 236, 0.1)",
                }}
              >
                <div
                  style={{
                    width: `${(previewProgress * 100).toFixed(1)}%`,
                    height: "100%",
                    background: C.amber,
                    boxShadow: `0 0 16px ${C.amber}`,
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 6 (600..714): 3D Hyperspace Warp + 165-Square Burst -> Terminal
         ==================================================================== */}
      <MotionGroup
        id="B6"
        frame={frame}
        startFrame={600}
        endFrame={714}
        inKind="none"
        outKind="wipe"
        plate={C.plum}
        zIndex={6}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <LightSpeedWarpCanvas t={T6} />

        {T6 < PT6 + 0.35 && (
          <div
            style={{
              position: "absolute",
              left: CX - 260,
              top: 220,
              width: 520,
              height: 76,
              borderRadius: 999,
              background: C.amber,
              color: C.plumInk,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: 14,
              fontFamily: F.mono,
              fontSize: 24,
              fontWeight: 800,
              transform: `scale(${(1 - 0.18 * press(T6, PT6)).toFixed(3)})`,
              opacity: 1 - seg(T6, PT6 + 0.08, PT6 + 0.32),
            }}
          >
            <span>Injecting & Pressing Enter ↵</span>
          </div>
        )}

        <PixelBurstCanvas
          t={T6}
          t0={PT6}
          srcRect={{ x: CX - 260, y: 220, w: 520, h: 76 }}
          dstRect={{ x: 200, y: 150, w: 1520, h: 780 }}
        />

        {(() => {
          const termU = eo(seg(T6, PT6 + 0.36, PT6 + 0.72));
          return (
            <div
              style={{
                position: "absolute",
                left: 200,
                top: 150,
                width: 1520,
                height: 780,
                borderRadius: 24,
                background: "#130310",
                border: `1.5px solid ${C.borderDark}`,
                boxShadow: "0 48px 120px rgba(0,0,0,0.72)",
                overflow: "hidden",
                opacity: termU,
                transform: `scale(${lerp(0.94, 1, termU).toFixed(3)})`,
                filter:
                  termU < 0.99
                    ? `blur(${(8 * (1 - termU)).toFixed(2)}px)`
                    : undefined,
              }}
            >
              <div
                style={{
                  height: 56,
                  padding: "0 26px",
                  background: "rgba(51, 10, 43, 0.65)",
                  borderBottom: `1px solid ${C.borderDark}`,
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                }}
              >
                <span
                  style={{
                    width: 13,
                    height: 13,
                    borderRadius: "50%",
                    background: "#ff5f56",
                  }}
                />
                <span
                  style={{
                    width: 13,
                    height: 13,
                    borderRadius: "50%",
                    background: "#ffbd2e",
                  }}
                />
                <span
                  style={{
                    width: 13,
                    height: 13,
                    borderRadius: "50%",
                    background: "#27c93f",
                  }}
                />
                <span
                  style={{
                    marginLeft: 18,
                    fontFamily: F.mono,
                    fontSize: 16,
                    color: C.mutedDark,
                  }}
                >
                  ghostty · ~/Projects/saywhen · zsh
                </span>
                <span
                  style={{
                    marginLeft: "auto",
                    padding: "5px 14px",
                    borderRadius: 99,
                    background: "rgba(251, 179, 3, 0.16)",
                    color: C.amber,
                    fontFamily: F.mono,
                    fontSize: 14,
                    fontWeight: 700,
                  }}
                >
                  ✓ Verify-Before-Paste Passed · Clipboard Restored
                </span>
              </div>

              <div
                style={{
                  padding: "38px 46px",
                  fontFamily: F.mono,
                  fontSize: 28,
                  lineHeight: 1.65,
                  color: C.ivory,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
                  <span style={{ color: C.amber }}>❯</span>
                  <span>cargo test --workspace --release</span>
                  {T6 >= 0.85 && (
                    <span
                      style={{
                        padding: "4px 14px",
                        borderRadius: 8,
                        background: C.amber,
                        color: C.plumInk,
                        fontSize: 18,
                        fontWeight: 800,
                        transform: `scale(${lerp(1.4, 1, bo(seg(T6, 0.85, 1.15)))})`,
                      }}
                    >
                      ↵ AUTO-SENT
                    </span>
                  )}
                </div>

                {T6 >= 1.1 && (
                  <div
                    style={{ marginTop: 22, color: C.mutedDark, fontSize: 24 }}
                  >
                    <div>
                      {"   "}Compiling saywhen v0.1.0
                      (/Users/dev/saywhen/src-tauri)
                    </div>
                    {T6 >= 1.45 && (
                      <div>
                        {"    "}Finished `release` profile [optimized] target(s)
                        in 0.38s
                      </div>
                    )}
                    {T6 >= 1.75 && (
                      <div style={{ marginTop: 14, color: C.ivory }}>
                        {"     "}Running unittests src/lib.rs
                      </div>
                    )}
                    {T6 >= 2.05 && (
                      <div>
                        test turn_taking::tests::adaptive_silence_timeout ...{" "}
                        <span style={{ color: C.amber }}>ok</span>
                      </div>
                    )}
                    {T6 >= 2.25 && (
                      <div>
                        test cleanup::tests::developer_cli_flags ...{" "}
                        <span style={{ color: C.amber }}>ok</span>
                      </div>
                    )}
                    {T6 >= 2.45 && (
                      <div>
                        test inject::policy::verify_frontmost_bundle ...{" "}
                        <span style={{ color: C.amber }}>ok</span>
                      </div>
                    )}
                    {T6 >= 2.75 && (
                      <div
                        style={{
                          marginTop: 20,
                          padding: "14px 22px",
                          borderRadius: 12,
                          background: "rgba(251, 179, 3, 0.14)",
                          border: `1px solid ${C.amber}`,
                          color: C.amber,
                          fontWeight: 700,
                          display: "inline-block",
                        }}
                      >
                        test result: ok. 48 passed; 0 failed; finished in 0.41s
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          );
        })()}
      </MotionGroup>

      {/* ====================================================================
          BEAT 7 (714..982): Light Plate — "You speak like a person. It writes like a developer."
         ==================================================================== */}
      <MotionGroup
        id="B7"
        frame={frame}
        startFrame={714}
        endFrame={982}
        inKind="wipe"
        outKind="pan"
        plate={C.ivory}
        zIndex={7}
      >
        <HorizonGrid3DCanvas frame={frame} lightPlate />

        <div style={{ position: "absolute", left: 106, top: 76 }}>
          <div
            style={{
              fontFamily: F.mono,
              fontSize: 19,
              fontWeight: 700,
              color: C.mutedLight,
              letterSpacing: "0.08em",
            }}
          >
            LOCAL DETERMINISTIC CLEANUP · SRC-TAURI/SRC/CLEANUP.RS
          </div>
          <div
            style={{
              marginTop: 10,
              fontFamily: F.sans,
              fontSize: 68,
              fontWeight: 800,
              letterSpacing: "-0.045em",
              color: C.plumInk,
            }}
          >
            Speak like a person.{" "}
            <span
              style={{
                padding: "0 14px",
                borderRadius: 12,
                background: "color-mix(in oklab, #fbb303 34%, transparent)",
              }}
            >
              Write like a developer.
            </span>
          </div>
        </div>

        {[
          {
            spoken:
              '"git check out dash b feat slash voice gate and push origin"',
            written: "git checkout -b feat/voice-gate && git push -u origin HEAD",
            tag: "CLI Flag & Operator Compiler",
            t0: 0.35,
            morphT: 1.45,
          },
          {
            spoken:
              '"in turn taking dot rs update evaluate arming hysteresis"',
            written:
              "In `src-tauri/src/turn_taking.rs`, update `evaluate_arming_hysteresis`",
            tag: "snake_case & File Path Symbol Linker",
            t0: 2.4,
            morphT: 3.5,
          },
          {
            spoken: '"claude dash p fix the race condition in audio dot rs"',
            written:
              'claude -p "fix the race condition in src-tauri/src/audio.rs"',
            tag: "Custom Vocab + Agent Prompt Formatter",
            t0: 4.45,
            morphT: 5.55,
          },
        ].map((row, i) => {
          const enterU = eo(seg(T7, row.t0, row.t0 + 0.42));
          const isCompiled = T7 >= row.morphT;
          const popU = bo(seg(T7, row.morphT, row.morphT + 0.36));
          const hlW = seg(T7, row.morphT - 0.55, row.morphT - 0.05);

          return (
            <div
              key={i}
              style={{
                position: "absolute",
                left: 106,
                top: 256 + i * 244,
                width: 1708,
                height: 214,
                borderRadius: 24,
                background: C.white,
                border: `2px solid ${isCompiled ? C.amber : C.borderLight}`,
                boxShadow: isCompiled
                  ? "0 28px 70px rgba(34, 6, 28, 0.14)"
                  : "0 14px 36px rgba(34, 6, 28, 0.06)",
                padding: "28px 38px",
                display: "flex",
                flexDirection: "column",
                justifyContent: "space-between",
                opacity: enterU,
                transform: `translateY(${lerp(36, 0, enterU).toFixed(1)}px) scale(${isCompiled ? lerp(1.03, 1, popU).toFixed(3) : 1})`,
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <span
                  style={{
                    fontFamily: F.mono,
                    fontSize: 16,
                    fontWeight: 700,
                    color: isCompiled ? C.plumInk : C.mutedLight,
                    padding: "5px 14px",
                    borderRadius: 99,
                    background: isCompiled ? C.amber : C.ivory,
                  }}
                >
                  {isCompiled
                    ? `✓ COMPILED · ${row.tag}`
                    : `SPOKEN RAW · 0${i + 1}`}
                </span>
                <span
                  style={{
                    fontFamily: F.mono,
                    fontSize: 15,
                    color: C.mutedLight,
                  }}
                >
                  0 ms network latency · Deterministic Rust pass
                </span>
              </div>

              {!isCompiled ? (
                <div
                  style={{
                    position: "relative",
                    display: "inline-block",
                    alignSelf: "flex-start",
                    fontFamily: F.serif,
                    fontStyle: "italic",
                    fontSize: 46,
                    color: C.mutedLight,
                  }}
                >
                  <span
                    style={{
                      position: "absolute",
                      left: -6,
                      top: 4,
                      bottom: 4,
                      width: `${(hlW * 100).toFixed(1)}%`,
                      background:
                        "color-mix(in oklab, #fbb303 32%, transparent)",
                      borderRadius: 8,
                    }}
                  />
                  <span style={{ position: "relative" }}>{row.spoken}</span>
                </div>
              ) : (
                <div
                  style={{
                    fontFamily: F.mono,
                    fontSize: 36,
                    fontWeight: 700,
                    color: C.plumInk,
                    letterSpacing: "-0.02em",
                  }}
                >
                  <WetInkText
                    text={row.written}
                    t={T7}
                    t0={row.morphT}
                    cps={65}
                    settle={0.55}
                    inkColor={C.plumInk}
                    accentColor={C.amber}
                  />
                </div>
              )}
            </div>
          );
        })}
      </MotionGroup>

      {/* ====================================================================
          BEAT 8 (982..1124): 3D Camera Rig — "Pause. Nod. Or say send it."
         ==================================================================== */}
      <MotionGroup
        id="B8"
        frame={frame}
        startFrame={982}
        endFrame={1124}
        inKind="pan"
        outKind="focus"
        plate={C.plum}
        zIndex={8}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <div
          style={{
            position: "absolute",
            left: 0,
            width: VW,
            top: 78,
            textAlign: "center",
            zIndex: 10,
          }}
        >
          <div
            style={{
              fontFamily: F.mono,
              fontSize: 18,
              fontWeight: 700,
              color: C.amber,
              letterSpacing: "0.12em",
            }}
          >
            THREE HANDS-FREE SEND MODES
          </div>
          <div
            style={{
              marginTop: 8,
              fontFamily: F.serif,
              fontSize: 96,
              color: C.ivory,
              letterSpacing: "-0.03em",
            }}
          >
            Pause. Nod. Or say <span style={{ color: C.amber }}>send it.</span>
          </div>
        </div>

        <div
          style={{
            position: "absolute",
            inset: 0,
            perspective: 1500,
            perspectiveOrigin: "50% 54%",
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 0,
              transformOrigin: "50% 50%",
              transformStyle: "preserve-3d",
              ...cam3D_B8,
            }}
          >
            {[
              {
                title: "Pause.",
                badge: "MODE 01 · ADAPTIVE SILENCE",
                desc: "Waits 550 ms on a crisp command, or 1,100 ms mid-thought so it never cuts you off.",
                metric: "780 ms",
                x: 160,
                activeWindow: [0, 1.55],
              },
              {
                title: "Nod.",
                badge: "MODE 02 · HEAD GESTURE",
                desc: "Think silently as long as you want. Dip your chin once (+11° pitch) to fire Enter.",
                metric: "+11.4° Pitch",
                x: 720,
                activeWindow: [1.35, 2.95],
              },
              {
                title: '"Send it."',
                badge: "MODE 03 · VOICE TRIGGER",
                desc: 'Strips "send it" or "submit it" from the transcript end and triggers immediate submit.',
                metric: "Stripped & Sent ↵",
                x: 1280,
                activeWindow: [2.75, 4.8],
              },
            ].map((card, i) => {
              const active =
                T8 >= card.activeWindow[0] && T8 <= card.activeWindow[1];
              const z = active ? 95 : 0;
              return (
                <div
                  key={i}
                  style={{
                    position: "absolute",
                    left: card.x,
                    top: 320,
                    width: 480,
                    height: 540,
                    borderRadius: 28,
                    background: active ? C.white : "rgba(51, 10, 43, 0.85)",
                    color: active ? C.plumInk : C.ivory,
                    border: `2px solid ${active ? C.amber : C.borderDark}`,
                    boxShadow: active
                      ? "0 48px 110px rgba(0,0,0,0.68), 0 0 48px rgba(251,179,3,0.28)"
                      : "0 24px 60px rgba(0,0,0,0.4)",
                    padding: "42px 38px",
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "space-between",
                    ...p3Style(T8, 0.1 + i * 0.14, 0.5, {
                      z,
                      focus: 95,
                      tilt: false,
                    }),
                  }}
                >
                  <div>
                    <div
                      style={{
                        fontFamily: F.mono,
                        fontSize: 15,
                        fontWeight: 700,
                        color: active ? C.mutedLight : C.amber,
                        letterSpacing: "0.08em",
                      }}
                    >
                      {card.badge}
                    </div>
                    <div
                      style={{
                        marginTop: 22,
                        fontFamily: F.serif,
                        fontSize: 92,
                        lineHeight: 0.95,
                      }}
                    >
                      {card.title}
                    </div>
                    <div
                      style={{
                        marginTop: 24,
                        fontFamily: F.sans,
                        fontSize: 23,
                        lineHeight: 1.45,
                        fontWeight: 500,
                        color: active ? C.mutedLight : C.mutedDark,
                      }}
                    >
                      {card.desc}
                    </div>
                  </div>

                  <div
                    style={{
                      padding: "18px 22px",
                      borderRadius: 16,
                      background: active ? C.amber : "rgba(223,229,236,0.08)",
                      color: active ? C.plumInk : C.ivory,
                      fontFamily: F.mono,
                      fontSize: 22,
                      fontWeight: 800,
                      display: "flex",
                      justifyContent: "space-between",
                    }}
                  >
                    <span>{card.metric}</span>
                    <span>↵</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 9 (1124..1280): 3D Shatter & Focus-Pull Cancel Override
         ==================================================================== */}
      <MotionGroup
        id="B9"
        frame={frame}
        startFrame={1124}
        endFrame={1280}
        inKind="focus"
        outKind="none"
        plate={C.plum}
        zIndex={9}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <ShatteringKeycaps3DCanvas t={T9} shatterTime={cancelTrigger} />

        <div
          style={{
            position: "absolute",
            left: CX - 540,
            top: 250,
            width: 1080,
            padding: "40px 48px",
            borderRadius: 28,
            background: C.plumDeep,
            border: `2px solid ${C.borderDark}`,
            transform: `scale(${lerp(1, 0.93, cancelProgress).toFixed(3)})`,
            filter:
              cancelProgress > 0.02
                ? `blur(${(11 * cancelProgress).toFixed(2)}px)`
                : undefined,
            opacity: lerp(1, 0.38, cancelProgress),
          }}
        >
          <div
            style={{
              fontFamily: F.mono,
              fontSize: 18,
              color: C.amber,
              marginBottom: 18,
            }}
          >
            PREVIEW · SENDING IN 0.4s...
          </div>
          <div
            style={{
              fontFamily: F.mono,
              fontSize: 38,
              color: C.ivory,
              textDecoration: cancelProgress > 0.5 ? "line-through" : "none",
            }}
          >
            $ git push --force origin main
          </div>
        </div>

        {T9 >= cancelTrigger && (
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <div
              style={{
                padding: "28px 54px",
                borderRadius: 999,
                background: C.white,
                color: C.plumInk,
                boxShadow: "0 44px 110px rgba(0,0,0,0.75)",
                display: "flex",
                alignItems: "center",
                gap: 24,
                transform: `scale(${lerp(0.65, 1, bo(seg(T9, cancelTrigger, cancelTrigger + 0.36))).toFixed(3)})`,
                opacity: eo(seg(T9, cancelTrigger, cancelTrigger + 0.14)),
              }}
            >
              <span
                style={{
                  padding: "10px 22px",
                  borderRadius: 99,
                  background: C.danger,
                  color: C.white,
                  fontFamily: F.mono,
                  fontSize: 24,
                  fontWeight: 800,
                }}
              >
                ESC / SHAKE / &quot;SCRATCH THAT&quot;
              </span>
              <span
                style={{
                  fontFamily: F.sans,
                  fontSize: 44,
                  fontWeight: 800,
                  letterSpacing: "-0.03em",
                }}
              >
                Canceled. Nothing injected.
              </span>
            </div>

            <div
              style={{
                marginTop: 34,
                display: "flex",
                gap: 20,
                opacity: eo(seg(T9, cancelTrigger + 0.28, cancelTrigger + 0.6)),
              }}
            >
              {[
                "1. Shake head left-right",
                '2. Say "scratch that"',
                "3. Start speaking again",
                "4. Tap Esc",
              ].map((m, idx) => (
                <div
                  key={idx}
                  style={{
                    padding: "14px 24px",
                    borderRadius: 14,
                    background: "rgba(223,229,236,0.1)",
                    border: `1px solid ${C.borderDark}`,
                    fontFamily: F.mono,
                    fontSize: 19,
                    fontWeight: 600,
                    color: C.ivory,
                  }}
                >
                  {m}
                </div>
              ))}
            </div>
          </div>
        )}
      </MotionGroup>

      {/* ====================================================================
          BEAT 10 (1280..1528): Halftone Iris -> 3D Hex Force-Field Destructive Guard
         ==================================================================== */}
      <MotionGroup
        id="B10"
        frame={frame}
        startFrame={1280}
        endFrame={1528}
        inKind="iris"
        outKind="up"
        extraBlur={cam10Blur}
        plate={C.ivory}
        zIndex={10}
      >
        <HorizonGrid3DCanvas frame={frame} lightPlate />

        <div
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            width: VW,
            height: VH,
            transformOrigin: "0 0",
            transform: `translate(${CX}px, ${CY}px) scale(${cam10.m.toFixed(4)}) translate(${(-cam10.cx).toFixed(2)}px, ${(-cam10.cy).toFixed(2)}px)`,
          }}
        >
          <HexShieldForcefieldCanvas t={T10} />
          <div
            style={{
              position: "absolute",
              left: 300,
              top: 110,
              fontFamily: F.sans,
              fontSize: 52,
              fontWeight: 800,
              letterSpacing: "-0.04em",
              color: C.plumInk,
            }}
          >
            Risky commands{" "}
            <span style={{ color: C.danger }}>never press Enter</span> for you.
          </div>

          <div
            style={{
              position: "absolute",
              left: 300,
              top: 210,
              width: 1320,
              borderRadius: 26,
              background: C.white,
              border: `2px solid ${C.borderLight}`,
              boxShadow: "0 36px 90px rgba(34, 6, 28, 0.14)",
              overflow: "hidden",
              padding: "44px 50px",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <span
                style={{
                  fontFamily: F.mono,
                  fontSize: 18,
                  fontWeight: 700,
                  color: C.danger,
                  padding: "8px 18px",
                  borderRadius: 99,
                  background: "rgba(255, 107, 94, 0.14)",
                }}
              >
                ⚠ DESTRUCTIVE COMMAND GUARD · CHECK_DESTRUCTIVE_COMMAND()
              </span>
              <span
                style={{
                  fontFamily: F.mono,
                  fontSize: 17,
                  fontWeight: 600,
                  color: C.mutedLight,
                }}
              >
                Downgraded to Paste-Only
              </span>
            </div>

            <div
              style={{
                marginTop: 42,
                padding: "32px 36px",
                borderRadius: 20,
                background: C.plum,
                color: C.ivory,
                fontFamily: F.mono,
                fontSize: 30,
                fontWeight: 700,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 20,
                whiteSpace: "nowrap",
              }}
            >
              <div style={{ whiteSpace: "nowrap" }}>
                <span style={{ color: C.amber }}>$ </span>
                <WetInkText
                  text="rm -rf dist/ && git reset --hard HEAD~1"
                  t={T10}
                  t0={0.55}
                  cps={36}
                  settle={0.5}
                  inkColor={C.ivory}
                  accentColor={C.danger}
                />
                <span
                  style={{
                    display: "inline-block",
                    width: 4,
                    height: "1em",
                    marginLeft: 6,
                    verticalAlign: "-0.12em",
                    background: C.amber,
                    opacity: Math.floor(T10 * 2.5) % 2 === 0 ? 1 : 0.2,
                  }}
                />
              </div>

              <div
                style={{
                  padding: "12px 22px",
                  borderRadius: 14,
                  background: C.danger,
                  color: C.white,
                  fontSize: 19,
                  fontWeight: 800,
                  whiteSpace: "nowrap",
                  flexShrink: 0,
                  transform: `scale(${lerp(1.35, 1, bo(seg(T10, 1.65, 2.05)))})`,
                  opacity: eo(seg(T10, 1.55, 1.75)),
                }}
              >
                ✕ AUTO-ENTER BLOCKED
              </div>
            </div>

            <div
              style={{
                display: "flex",
                gap: 20,
                marginTop: 42,
              }}
            >
              {[
                {
                  pattern: "rm -rf / sudo",
                  status: "Pasted without Enter",
                },
                {
                  pattern: "git reset --hard / push --force",
                  status: "Pasted without Enter",
                },
                {
                  pattern: "DROP TABLE / kubectl delete",
                  status: "Pasted without Enter",
                },
              ].map((item, idx) => (
                <div
                  key={idx}
                  style={{
                    flex: 1,
                    padding: "24px 26px",
                    borderRadius: 18,
                    background: C.ivory,
                    border: `1px solid ${C.borderLight}`,
                  }}
                >
                  <div
                    style={{
                      fontFamily: F.mono,
                      fontSize: 20,
                      fontWeight: 700,
                      color: C.plumInk,
                    }}
                  >
                    {item.pattern}
                  </div>
                  <div
                    style={{
                      marginTop: 8,
                      fontFamily: F.sans,
                      fontSize: 18,
                      fontWeight: 600,
                      color: C.danger,
                    }}
                  >
                    ✓ {item.status}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 11 (1528..1676): High-Speed Vertical Slot Reel + 3D Horizon Grid
         ==================================================================== */}
      <MotionGroup
        id="B11"
        frame={frame}
        startFrame={1528}
        endFrame={1676}
        inKind="up"
        outKind="focus"
        extraBlur={{ bx: 0, by: mb(rVel) }}
        plate={C.plum}
        zIndex={11}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <div
          style={{
            position: "absolute",
            inset: 0,
            overflow: "hidden",
            WebkitMaskImage:
              "linear-gradient(to bottom, transparent 16%, #000 36%, #000 64%, transparent 84%)",
          }}
        >
          {REEL_ITEMS.map((item, i) => {
            const d = i - rP;
            if (Math.abs(d) > 2.6) return null;
            const ad = Math.abs(d);
            const scaleY = 1 - 0.16 * Math.pow(Math.min(ad, 1.6), 2) * 0.5;
            const op = 1 - 0.62 * Math.min(ad, 1.4);
            const extraPop = i === RL ? 1 + 0.06 * rLand : 1;

            return (
              <div
                key={i}
                style={{
                  position: "absolute",
                  left: 0,
                  right: 0,
                  top: 0,
                  textAlign: "center",
                  fontFamily: F.sans,
                  fontSize: 138,
                  fontWeight: 800,
                  letterSpacing: "-0.05em",
                  lineHeight: 1.18,
                  whiteSpace: "nowrap",
                  color: item.acc ? C.amber : C.ivory,
                  transformOrigin: "50% 50%",
                  transform: `translateY(${(CY - REH / 2 + d * REH).toFixed(1)}px) scaleY(${scaleY.toFixed(3)}) scale(${extraPop.toFixed(3)})`,
                  opacity: op,
                }}
              >
                {item.label}
              </div>
            );
          })}

          <div
            style={{
              position: "absolute",
              left: "24%",
              right: "24%",
              top: CY - REH / 2 - 14,
              height: 1.5,
              background: C.borderDark,
            }}
          />
          <div
            style={{
              position: "absolute",
              left: "24%",
              right: "24%",
              top: CY + REH / 2 + 8,
              height: 1.5,
              background: C.borderDark,
            }}
          />
        </div>
      </MotionGroup>

      {/* ====================================================================
          BEAT 12 (1676..1800): Fibonacci Golden Vortex + End Card Lockup
         ==================================================================== */}
      <MotionGroup
        id="B12"
        frame={frame}
        startFrame={1676}
        endFrame={1800}
        inKind="focus"
        outKind="none"
        plate={C.plum}
        zIndex={12}
      >
        <HorizonGrid3DCanvas frame={frame} />
        <FibonacciVortexCanvas t={T12} rms={rms} />
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 10,
          }}
        >
          {(() => {
            const logoU = eo(seg(T12, 0.08, 0.55));
            return (
              <Img
                src={staticFile("logo.png")}
                style={{
                  width: 176,
                  height: 176,
                  opacity: logoU,
                  transform: `scale(${lerp(0.92, 1, logoU).toFixed(3)})`,
                  filter: `drop-shadow(0 0 ${Math.round(32 + rms * 45)}px rgba(251, 179, 3, 0.45))`,
                }}
              />
            );
          })()}

          <div
            style={{
              marginTop: 18,
              fontFamily: F.sans,
              fontSize: 136,
              fontWeight: 700,
              letterSpacing: "-0.05em",
              lineHeight: 1,
              color: C.silver,
            }}
          >
            <BuildLetters text="saywhen" t={T12} t0={0.18} step={0.045} />
          </div>

          <div
            style={{
              marginTop: 22,
              fontFamily: F.serif,
              fontSize: 54,
              color: C.amber,
              letterSpacing: "-0.02em",
              opacity: eo(seg(T12, 0.65, 1.1)),
              transform: `translateY(${lerp(16, 0, eo(seg(T12, 0.65, 1.1))).toFixed(1)}px)`,
            }}
          >
            Look. Talk. Sent.
          </div>

          {(() => {
            const pillU = seg(T12, 1.05, 1.42);
            return (
              <div
                style={{
                  marginTop: 36,
                  height: 74,
                  padding: "0 38px",
                  borderRadius: 999,
                  background: C.amber,
                  color: C.plumInk,
                  fontFamily: F.sans,
                  fontSize: 28,
                  fontWeight: 700,
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 16,
                  boxShadow: "0 24px 60px rgba(251, 179, 3, 0.34)",
                  opacity: eo(seg(T12, 1.05, 1.2)),
                  transform: `scale(${(pillU ? lerp(0.65, 1, bo(pillU)) : 0.65).toFixed(3)})`,
                }}
              >
                <span>Hands-free voice for developers</span>
                <span>→</span>
              </div>
            );
          })()}
        </div>
      </MotionGroup>

      {/* Simulated Screen-Space Pointer for Beat 6 Send Press */}
      {frame >= 585 && frame <= 628 && (
        <SimulatedCursor
          t={(frame - 585) / FPS}
          keys={[
            { t: 0, x: CX + 280, y: 420 },
            { t: 0.55, x: CX + 110, y: 258 },
          ]}
          presses={[0.62]}
          show={[0.05, 1.15]}
        />
      )}

      {/* Subtle Voiceover Captions at Bottom */}
      <FilmSubtitles frame={frame} />
    </AbsoluteFill>
  );
};
