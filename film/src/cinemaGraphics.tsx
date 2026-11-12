import React, { useEffect, useRef } from "react";

/**
 * Cinema-grade procedural 3D & optical canvas generators for the saywhen launch film.
 * All geometry is 100% deterministic from frame time `f` and renders cleanly at 1920x1080.
 */

const project3D = (
  x: number,
  y: number,
  z: number,
  fov = 840,
  cx = 960,
  cy = 540
) => {
  const depth = Math.max(40, fov + z);
  const s = fov / depth;
  return { x: cx + x * s, y: cy + y * s, s, depth };
};

const rotX = (y: number, z: number, a: number) => {
  const c = Math.cos(a);
  const s = Math.sin(a);
  return { y: y * c - z * s, z: y * s + z * c };
};

const rotY = (x: number, z: number, a: number) => {
  const c = Math.cos(a);
  const s = Math.sin(a);
  return { x: x * c + z * s, z: -x * s + z * c };
};

const rotZ = (x: number, y: number, a: number) => {
  const c = Math.cos(a);
  const s = Math.sin(a);
  return { x: x * c - y * s, y: x * s + y * c };
};

const pseudo = (seed: number) => {
  const v = Math.sin(seed * 12.9898 + 78.233) * 43758.5453;
  return v - Math.floor(v);
};

// 1. Optical Horizon Grid with subtle specular depth fog
export const HorizonGrid3DCanvas: React.FC<{
  f: number;
  tiltX?: number;
  yaw?: number;
  color?: string;
  alpha?: number;
}> = ({ f, tiltX = 0.44, yaw = 0, color = "#fbb303", alpha = 0.24 }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const cols = 18;
    const rows = 20;
    const spacing = 140;
    const scrollZ = (f * 8) % spacing;

    ctx.lineCap = "round";

    for (let c = -cols / 2; c <= cols / 2; c++) {
      const worldX = c * spacing;
      ctx.beginPath();
      let started = false;
      for (let r = 0; r <= rows; r++) {
        const z0 = r * spacing - scrollZ - 260;
        if (z0 < -720) continue;
        const wave = Math.sin(r * 0.26 - f * 0.04 + c * 0.35) * 14;
        const ry = rotY(worldX, z0, yaw);
        const rx = rotX(280 + wave, ry.z, tiltX);
        const p = project3D(ry.x, rx.y, rx.z, 780, 960, 520);
        if (!started) {
          ctx.moveTo(p.x, p.y);
          started = true;
        } else {
          ctx.lineTo(p.x, p.y);
        }
      }
      const centerDist = Math.abs(c) / (cols / 2);
      ctx.strokeStyle = color;
      ctx.globalAlpha = alpha * (1 - centerDist * 0.72) * 0.65;
      ctx.lineWidth = Math.abs(c) % 4 === 0 ? 1.5 : 0.85;
      ctx.stroke();
    }

    for (let r = 1; r <= rows; r++) {
      const z0 = r * spacing - scrollZ - 260;
      if (z0 < -680) continue;
      const distFade = Math.max(0, 1 - r / (rows + 2));
      ctx.beginPath();
      let started = false;
      for (let c = -cols / 2; c <= cols / 2; c++) {
        const worldX = c * spacing;
        const wave = Math.sin(r * 0.26 - f * 0.04 + c * 0.35) * 14;
        const ry = rotY(worldX, z0, yaw);
        const rx = rotX(280 + wave, ry.z, tiltX);
        const p = project3D(ry.x, rx.y, rx.z, 780, 960, 520);
        if (!started) {
          ctx.moveTo(p.x, p.y);
          started = true;
        } else {
          ctx.lineTo(p.x, p.y);
        }
      }
      ctx.strokeStyle = color;
      ctx.globalAlpha = alpha * distFade * 0.72;
      ctx.lineWidth = 1.05;
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }, [f, tiltX, yaw, color, alpha]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 2. Sculpted Optical Glass Shards & Luminous Dust (cleanly framed, zero text collision)
export const ShatteringKeycaps3DCanvas: React.FC<{
  f: number;
  explodeFrame: number;
}> = ({ f, explodeFrame }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const tExplode = Math.max(0, f - explodeFrame);
    const prog = Math.min(1, tExplode / 42);
    const ease = 1 - Math.pow(1 - prog, 3);

    if (tExplode <= 0) return;

    const pieces = 38;
    const projected: {
      x: number;
      y: number;
      s: number;
      z: number;
      rot: number;
      w: number;
      h: number;
      tint: string;
      alpha: number;
      streakX: number;
      streakY: number;
    }[] = [];

    for (let i = 0; i < pieces; i++) {
      const a = pseudo(i * 11 + 1) * Math.PI * 2;
      const ringRadius = 420 + pseudo(i * 13 + 7) * 580;
      const initX = Math.cos(a) * 180;
      const initY = Math.sin(a) * 110;

      // Push shards into orbital perimeter arcs so center typography stays razor-sharp
      const vx = Math.cos(a) * ringRadius;
      const vy = Math.sin(a) * ringRadius * 0.68;
      const vz = (pseudo(i * 19 + 5) - 0.35) * 900;

      const x = initX + vx * ease;
      const y = initY + vy * ease + Math.sin(ease * 3.1 + i) * 14;
      const z = vz * ease - ease * 180;

      const p = project3D(x, y, z, 860, 960, 540);
      const sz = (16 + pseudo(i * 23) * 28) * p.s;
      const fade = Math.max(0, 1 - Math.pow(prog, 2.1));

      const tints = [
        "rgba(251, 179, 3, 0.85)",
        "rgba(232, 198, 255, 0.72)",
        "rgba(130, 232, 255, 0.65)",
      ];

      projected.push({
        x: p.x,
        y: p.y,
        s: p.s,
        z: p.depth,
        rot: a + tExplode * 0.045 * (pseudo(i * 31) > 0.5 ? 1 : -1),
        w: sz * 1.4,
        h: sz * 0.52,
        tint: tints[i % tints.length],
        alpha: fade * Math.min(1, tExplode / 4),
        streakX: Math.cos(a) * 42 * (1 - prog * 0.6),
        streakY: Math.sin(a) * 28 * (1 - prog * 0.6),
      });
    }

    projected.sort((a, b) => b.z - a.z);

    for (const q of projected) {
      if (q.alpha <= 0.02) continue;
      ctx.save();
      ctx.translate(q.x, q.y);

      // Subtle motion trail behind prism shard
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(-q.streakX, -q.streakY);
      ctx.strokeStyle = q.tint;
      ctx.globalAlpha = q.alpha * 0.35;
      ctx.lineWidth = 1.6;
      ctx.stroke();

      ctx.rotate(q.rot);
      ctx.globalAlpha = q.alpha * 0.82;
      ctx.fillStyle = "rgba(26, 10, 24, 0.75)";
      ctx.strokeStyle = q.tint;
      ctx.lineWidth = 1.4;
      ctx.beginPath();
      ctx.roundRect(-q.w / 2, -q.h / 2, q.w, q.h, 4);
      ctx.fill();
      ctx.stroke();
      ctx.restore();
    }
    ctx.globalAlpha = 1;
  }, [f, explodeFrame]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 3. Sculpted Liquid Silk Ribbon (flows along sculpted arcs, never obscures center serif text)
export const ParametricRibbon3DCanvas: React.FC<{
  f: number;
  progress: number;
  color?: string;
}> = ({ f, progress, color = "#fbb303" }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const steps = 110;
    const maxStep = Math.floor(steps * Math.min(1, Math.max(0.05, progress)));
    const layers = [
      { dy: 210, ampY: 42, phase: 0, stroke: color, alpha: 0.72, lw: 3.0 },
      { dy: 222, ampY: 50, phase: 0.6, stroke: "rgba(180, 92, 232, 0.85)", alpha: 0.45, lw: 1.8 },
      { dy: -225, ampY: 36, phase: 1.7, stroke: "rgba(130, 232, 255, 0.7)", alpha: 0.34, lw: 1.4 },
    ];

    for (const layer of layers) {
      ctx.beginPath();
      for (let i = 0; i <= maxStep; i++) {
        const u = i / steps;
        const wx = -920 + u * 1840;
        const wy =
          layer.dy +
          Math.sin(u * Math.PI * 2.4 - f * 0.045 + layer.phase) * layer.ampY +
          Math.cos(u * Math.PI * 1.2 + layer.phase) * 18;
        const wz = Math.cos(u * Math.PI * 2 + layer.phase) * 140;
        const p = project3D(wx, wy, wz, 880, 960, 540);
        if (i === 0) ctx.moveTo(p.x, p.y);
        else ctx.lineTo(p.x, p.y);
      }
      ctx.strokeStyle = layer.stroke;
      ctx.globalAlpha = layer.alpha;
      ctx.lineWidth = layer.lw;
      ctx.lineCap = "round";
      ctx.shadowColor = layer.stroke;
      ctx.shadowBlur = 18;
      ctx.stroke();
      ctx.shadowBlur = 0;
    }
    ctx.globalAlpha = 1;
  }, [f, progress, color]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 4. Sculpted Anatomical 478-Point MediaPipe Biometric Reticle (sleek glass contours, not an oval egg)
export const BiometricFaceMesh3DCanvas: React.FC<{
  f: number;
  cx?: number;
  cy?: number;
  scale?: number;
  yawAngle?: number;
  pitchAngle?: number;
  mouthOpen?: number;
  isLocked?: boolean;
}> = ({
  f,
  cx = 430,
  cy = 530,
  scale = 1.05,
  yawAngle,
  pitchAngle,
  mouthOpen,
  isLocked = true,
}) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const yAng = yawAngle ?? Math.sin(f * 0.04) * 0.14;
    const pAng = pitchAngle ?? Math.cos(f * 0.03) * 0.07;
    const mouth = mouthOpen ?? (0.28 + Math.max(0, Math.sin(f * 0.36)) * 0.65);

    const proj = (vx: number, vy: number, vz: number) => {
      const r1 = rotY(vx * 185 * scale, vz * 185 * scale, yAng);
      const r2 = rotX(vy * 225 * scale, r1.z, pAng);
      return project3D(r1.x, r2.y, r2.z - 60, 820, cx, cy);
    };

    // Soft biometric radial halo behind reticle
    const halo = ctx.createRadialGradient(cx, cy, 18, cx, cy, 290 * scale);
    halo.addColorStop(0, isLocked ? "rgba(130, 232, 255, 0.16)" : "rgba(251, 179, 3, 0.10)");
    halo.addColorStop(0.55, "rgba(180, 92, 232, 0.08)");
    halo.addColorStop(1, "rgba(18, 4, 18, 0)");
    ctx.fillStyle = halo;
    ctx.beginPath();
    ctx.arc(cx, cy, 290 * scale, 0, Math.PI * 2);
    ctx.fill();

    // Precision optical tracking corners
    const bR = 230 * scale;
    const arm = 28 * scale;
    ctx.strokeStyle = isLocked ? "rgba(251, 179, 3, 0.82)" : "rgba(255, 255, 255, 0.35)";
    ctx.lineWidth = 2.2;
    for (const [dx, dy] of [
      [-1, -1],
      [1, -1],
      [-1, 1],
      [1, 1],
    ]) {
      const bx = cx + dx * bR * 0.88;
      const by = cy + dy * bR * 1.05;
      ctx.beginPath();
      ctx.moveTo(bx - dx * arm, by);
      ctx.lineTo(bx, by);
      ctx.lineTo(bx, by - dy * arm);
      ctx.stroke();
    }

    // Horizontal scan sweep line
    const scanY = cy + Math.sin(f * 0.075) * 195 * scale;
    const scanGrad = ctx.createLinearGradient(cx - 210 * scale, 0, cx + 210 * scale, 0);
    scanGrad.addColorStop(0, "rgba(130, 232, 255, 0)");
    scanGrad.addColorStop(0.5, "rgba(130, 232, 255, 0.55)");
    scanGrad.addColorStop(1, "rgba(130, 232, 255, 0)");
    ctx.strokeStyle = scanGrad;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(cx - 210 * scale, scanY);
    ctx.lineTo(cx + 210 * scale, scanY);
    ctx.stroke();

    // Anatomical outer contour (forehead -> temples -> tapered cheekbones -> sculpted jaw -> chin)
    const jawContour: [number, number, number][] = [
      [0.0, -0.94, 0.14],
      [0.44, -0.88, 0.10],
      [0.74, -0.62, 0.00],
      [0.82, -0.22, -0.06],
      [0.74, 0.24, -0.04],
      [0.54, 0.64, 0.08],
      [0.24, 0.88, 0.22],
      [0.0, 0.94, 0.26],
      [-0.24, 0.88, 0.22],
      [-0.54, 0.64, 0.08],
      [-0.74, 0.24, -0.04],
      [-0.82, -0.22, -0.06],
      [-0.74, -0.62, 0.00],
      [-0.44, -0.88, 0.10],
    ];
    ctx.beginPath();
    jawContour.forEach(([vx, vy, vz], idx) => {
      const p = proj(vx, vy, vz);
      if (idx === 0) ctx.moveTo(p.x, p.y);
      else ctx.lineTo(p.x, p.y);
    });
    ctx.closePath();
    ctx.strokeStyle = "rgba(130, 232, 255, 0.52)";
    ctx.lineWidth = 1.6;
    ctx.stroke();

    // Inner cheekbone/brow/nose spline mesh
    const bridgePts: [number, number, number][] = [
      [-0.58, -0.34, 0.14],
      [-0.28, -0.38, 0.22],
      [0.0, -0.30, 0.28],
      [0.28, -0.38, 0.22],
      [0.58, -0.34, 0.14],
    ];
    ctx.beginPath();
    bridgePts.forEach(([vx, vy, vz], i) => {
      const p = proj(vx, vy, vz);
      if (i === 0) ctx.moveTo(p.x, p.y);
      else ctx.lineTo(p.x, p.y);
    });
    ctx.strokeStyle = "rgba(255, 255, 255, 0.26)";
    ctx.lineWidth = 1.2;
    ctx.stroke();

    // Nose bridge to tip
    const nosePts: [number, number, number][] = [
      [0.0, -0.30, 0.28],
      [0.0, 0.14, 0.42],
      [-0.12, 0.20, 0.30],
      [0.0, 0.22, 0.34],
      [0.12, 0.20, 0.30],
    ];
    ctx.beginPath();
    nosePts.forEach(([vx, vy, vz], i) => {
      const p = proj(vx, vy, vz);
      if (i === 0) ctx.moveTo(p.x, p.y);
      else ctx.lineTo(p.x, p.y);
    });
    ctx.strokeStyle = "rgba(130, 232, 255, 0.36)";
    ctx.lineWidth = 1.3;
    ctx.stroke();

    // Eye contours + glowing iris gaze markers
    for (const side of [-1, 1]) {
      const ex = side * 0.32;
      const ey = -0.22;
      const ez = 0.26;
      const eyeRing: [number, number, number][] = [
        [ex - 0.17, ey, ez - 0.03],
        [ex, ey - 0.08, ez + 0.02],
        [ex + 0.17, ey, ez - 0.03],
        [ex, ey + 0.07, ez + 0.02],
      ];
      ctx.beginPath();
      eyeRing.forEach(([vx, vy, vz], idx) => {
        const p = proj(vx, vy, vz);
        if (idx === 0) ctx.moveTo(p.x, p.y);
        else ctx.lineTo(p.x, p.y);
      });
      ctx.closePath();
      ctx.strokeStyle = "rgba(130, 232, 255, 0.78)";
      ctx.lineWidth = 1.6;
      ctx.stroke();

      const pupil = proj(ex, ey, ez + 0.03);
      ctx.beginPath();
      ctx.arc(pupil.x, pupil.y, 4.8 * scale, 0, Math.PI * 2);
      ctx.fillStyle = "#82e8ff";
      ctx.shadowColor = "#82e8ff";
      ctx.shadowBlur = 12;
      ctx.fill();
      ctx.shadowBlur = 0;
    }

    // Active speaking lip spline + landmark nodes
    const mw = 0.26;
    const mh = 0.04 + mouth * 0.08;
    const lipRing: [number, number, number][] = [
      [-mw, 0.44, 0.26],
      [-mw * 0.45, 0.44 - mh * 0.72, 0.32],
      [0, 0.44 - mh * 0.9, 0.35],
      [mw * 0.45, 0.44 - mh * 0.72, 0.32],
      [mw, 0.44, 0.26],
      [mw * 0.45, 0.44 + mh * 0.82, 0.31],
      [0, 0.44 + mh * 1.0, 0.34],
      [-mw * 0.45, 0.44 + mh * 0.82, 0.31],
    ];
    ctx.beginPath();
    const lipProj = lipRing.map(([vx, vy, vz]) => proj(vx, vy, vz));
    lipProj.forEach((p, idx) => {
      if (idx === 0) ctx.moveTo(p.x, p.y);
      else ctx.lineTo(p.x, p.y);
    });
    ctx.closePath();
    ctx.strokeStyle = "#fbb303";
    ctx.lineWidth = 2.4;
    ctx.shadowColor = "rgba(251, 179, 3, 0.65)";
    ctx.shadowBlur = 14;
    ctx.stroke();
    ctx.shadowBlur = 0;

    ctx.fillStyle = "#ffe596";
    for (const lp of lipProj) {
      ctx.beginPath();
      ctx.arc(lp.x, lp.y, 3.0 * scale, 0, Math.PI * 2);
      ctx.fill();
    }
  }, [f, cx, cy, scale, yawAngle, pitchAngle, mouthOpen, isLocked]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 5. Dual-Trace Harmonic Audio Terrain (Gold vocal envelope + Cyan Silero VAD floor)
export const AcousticTerrain3DCanvas: React.FC<{
  f: number;
  rms: number;
  color?: string;
}> = ({ f, rms, color = "#fbb303" }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const rows = 14;
    const cols = 48;

    for (let r = 0; r < rows; r++) {
      const z = -120 + r * 48;
      const depthFade = 1 - r / (rows + 2);
      ctx.beginPath();
      for (let c = 0; c <= cols; c++) {
        const u = (c / cols) * 2 - 1;
        const wx = u * 1020;
        const n1 = Math.sin(u * 4.8 + f * 0.14 + r * 0.34);
        const n2 = Math.cos(u * 9.2 - f * 0.21 + r * 0.52);
        const centerEnv = Math.exp(-u * u * 1.4);
        const amp = (24 + rms * 190) * centerEnv;
        const wy = 240 - (n1 * 0.62 + n2 * 0.38) * amp;
        const rx = rotX(wy, z, 0.32);
        const p = project3D(wx, rx.y, rx.z, 820, 960, 610);
        if (c === 0) ctx.moveTo(p.x, p.y);
        else ctx.lineTo(p.x, p.y);
      }
      const isLead = r < 2;
      ctx.strokeStyle = isLead ? "#82e8ff" : color;
      ctx.globalAlpha = depthFade * (isLead ? 0.68 : 0.34);
      ctx.lineWidth = isLead ? 2.2 : 1.3;
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }, [f, rms, color]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 6. Optical Perimeter Warp Streaks
export const LightSpeedWarpCanvas: React.FC<{
  f: number;
  speed?: number;
  streakCount?: number;
}> = ({ f, speed = 1.0, streakCount = 72 }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    for (let i = 0; i < streakCount; i++) {
      const a = pseudo(i * 19 + 3) * Math.PI * 2;
      const dist = 360 + pseudo(i * 23 + 7) * 760;
      const x = Math.cos(a) * dist;
      const y = Math.sin(a) * dist * 0.66;
      const z0 = ((pseudo(i * 29) * 1400 - f * 48 * speed) % 1400 + 1400) % 1400 - 500;
      const z1 = z0 + 190 * speed;

      const p0 = project3D(x, y, z0, 740, 960, 540);
      const p1 = project3D(x, y, z1, 740, 960, 540);
      if (z0 < -450) continue;

      const alpha = Math.min(0.65, Math.max(0, (700 - z0) / 1100)) * 0.55;
      ctx.strokeStyle = i % 3 === 0 ? "#fbb303" : i % 3 === 1 ? "#82e8ff" : "#e7baff";
      ctx.globalAlpha = alpha;
      ctx.lineWidth = Math.max(1, 2.2 * p0.s);
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(p0.x, p0.y);
      ctx.lineTo(p1.x, p1.y);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }, [f, speed, streakCount]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 7. Crimson Geodesic Force-Field Ring for Destructive Command Guard
export const HexShieldForcefieldCanvas: React.FC<{
  f: number;
  pulse?: number;
}> = ({ f, pulse = 0 }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const ringCount = 3;
    for (let r = 0; r < ringCount; r++) {
      const radius = 380 + r * 175 + pulse * 14;
      const spin = f * (r % 2 === 0 ? 0.008 : -0.006);
      const segs = 12 + r * 6;
      ctx.strokeStyle = r === 0 ? "rgba(255, 86, 114, 0.38)" : "rgba(255, 86, 114, 0.16)";
      ctx.lineWidth = r === 0 ? 1.8 : 1.0;
      ctx.setLineDash(r === 1 ? [14, 18] : []);
      ctx.beginPath();
      for (let i = 0; i <= segs; i++) {
        const a = (i / segs) * Math.PI * 2 + spin;
        const p = project3D(Math.cos(a) * radius, Math.sin(a) * radius * 0.72, 120 + r * 80, 900, 960, 540);
        if (i === 0) ctx.moveTo(p.x, p.y);
        else ctx.lineTo(p.x, p.y);
      }
      ctx.stroke();
    }
    ctx.setLineDash([]);
  }, [f, pulse]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};

// 8. Golden Fibonacci Convergence Halo for Brand Finale
export const FibonacciVortexCanvas: React.FC<{
  f: number;
  converge?: number;
}> = ({ f, converge = 1 }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, 1920, 1080);

    const n = 220;
    const goldenAngle = Math.PI * (3 - Math.sqrt(5));
    const spin = f * 0.011;

    for (let i = 0; i < n; i++) {
      const a = i * goldenAngle + spin;
      const rNorm = Math.sqrt(i / n);
      const r = (95 + rNorm * 660) * (1.28 - 0.28 * converge);
      const wx = Math.cos(a) * r;
      const wy = Math.sin(a) * r * 0.76;
      const wz = Math.sin(rNorm * Math.PI - f * 0.04) * 160;
      const p = project3D(wx, wy, wz, 920, 960, 500);
      const sz = (1.5 + (1 - rNorm) * 2.8) * p.s;
      const alpha = (0.14 + (1 - rNorm) * 0.56) * converge;

      ctx.beginPath();
      ctx.arc(p.x, p.y, sz, 0, Math.PI * 2);
      ctx.fillStyle = i % 5 === 0 ? "#82e8ff" : i % 2 === 0 ? "#fbb303" : "#e8c4ff";
      ctx.globalAlpha = Math.max(0.05, Math.min(0.85, alpha));
      ctx.fill();
    }
    ctx.globalAlpha = 1;
  }, [f, converge]);

  return (
    <canvas
      ref={ref}
      width={1920}
      height={1080}
      style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
    />
  );
};
