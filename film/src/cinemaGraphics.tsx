import React, { useEffect, useRef } from "react";
import { AUDIO_PEAK, AUDIO_RMS } from "./audioData";

const VW = 1920;
const VH = 1080;
const CX = VW / 2;
const CY = VH / 2;

const clamp = (x: number, a: number, b: number) => Math.max(a, Math.min(b, x));
const lerp = (a: number, b: number, u: number) => a + (b - a) * u;
const seg = (t: number, a: number, b: number) =>
  b <= a ? (t >= b ? 1 : 0) : clamp((t - a) / (b - a), 0, 1);
const eo = (u: number) => 1 - Math.pow(1 - u, 3);
const eio = (u: number) =>
  u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2;

// 3D rotation & perspective projection helper
function project3D(
  x: number,
  y: number,
  z: number,
  rx: number,
  ry: number,
  rz: number,
  cx = CX,
  cy = CY,
  fov = 1150,
) {
  // Rotate around X
  const cosX = Math.cos(rx);
  const sinX = Math.sin(rx);
  const y1 = y * cosX - z * sinX;
  const z1 = y * sinX + z * cosX;

  // Rotate around Y
  const cosY = Math.cos(ry);
  const sinY = Math.sin(ry);
  const x2 = x * cosY + z1 * sinY;
  const z2 = -x * sinY + z1 * cosY;

  // Rotate around Z
  const cosZ = Math.cos(rz);
  const sinZ = Math.sin(rz);
  const x3 = x2 * cosZ - y1 * sinZ;
  const y3 = x2 * sinZ + y1 * cosZ;

  const scale = fov / Math.max(120, fov - z2);
  return {
    sx: cx + x3 * scale,
    sy: cy + y3 * scale,
    z: z2,
    scale,
  };
}

// ============================================================================
// 1. 3D ATMOSPHERIC HORIZON GRID & VOLUMETRIC GOD-RAYS
// ============================================================================
export const HorizonGrid3DCanvas: React.FC<{
  frame: number;
  lightPlate?: boolean;
  intensity?: number;
}> = ({ frame, lightPlate = false, intensity = 1 }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const t = frame / 30;
    const rms = AUDIO_RMS[frame] ?? 0.2;
    const pk = AUDIO_PEAK[frame] ?? 0.3;

    // 1. Volumetric horizon bloom
    const hy = lightPlate ? 580 : 620;
    const rad = ctx.createRadialGradient(
      CX + Math.sin(t * 0.5) * 90,
      hy,
      20,
      CX,
      hy,
      920,
    );
    if (lightPlate) {
      rad.addColorStop(0, `rgba(251, 179, 3, ${(0.22 + rms * 0.14) * intensity})`);
      rad.addColorStop(0.45, `rgba(216, 169, 214, ${0.14 * intensity})`);
      rad.addColorStop(1, "rgba(246, 238, 244, 0)");
    } else {
      rad.addColorStop(0, `rgba(251, 179, 3, ${(0.26 + rms * 0.18) * intensity})`);
      rad.addColorStop(0.4, `rgba(88, 18, 74, ${(0.38 + pk * 0.12) * intensity})`);
      rad.addColorStop(1, "rgba(34, 6, 28, 0)");
    }
    ctx.fillStyle = rad;
    ctx.fillRect(0, 0, VW, VH);

    // 2. 3D Perspective Floor & Ceiling Grid
    const gridCol = lightPlate
      ? `rgba(42, 8, 35, ${0.075 * intensity})`
      : `rgba(251, 179, 3, ${(0.11 + rms * 0.06) * intensity})`;
    ctx.strokeStyle = gridCol;
    ctx.lineWidth = 1.2;

    const zSpeed = (t * 260) % 120;
    for (let iz = 1; iz <= 14; iz++) {
      const z = iz * 120 - zSpeed - 300;
      const pL = project3D(-1800, 360, z, 0.18, 0, 0, CX, CY, 900);
      const pR = project3D(1800, 360, z, 0.18, 0, 0, CX, CY, 900);
      const alpha = clamp(1 - iz / 14, 0, 1) * 0.55 * intensity;
      ctx.globalAlpha = alpha;
      ctx.beginPath();
      ctx.moveTo(pL.sx, pL.sy);
      ctx.lineTo(pR.sx, pR.sy);
      ctx.stroke();
    }

    for (let ix = -12; ix <= 12; ix++) {
      const x = ix * 160;
      const pNear = project3D(x, 360, 400, 0.18, 0, 0, CX, CY, 900);
      const pFar = project3D(x, 360, -1400, 0.18, 0, 0, CX, CY, 900);
      ctx.globalAlpha = (1 - Math.abs(ix) / 14) * 0.32 * intensity;
      ctx.beginPath();
      ctx.moveTo(pNear.sx, pNear.sy);
      ctx.lineTo(pFar.sx, pFar.sy);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }, [frame, lightPlate, intensity]);

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
// 2. BEAT 1: 3D EXTRUDED MECHANICAL KEYCAPS SHATTERING IN 3D SPACE
// ============================================================================
export const ShatteringKeycaps3DCanvas: React.FC<{
  t: number;
  shatterTime?: number;
}> = ({ t, shatterTime = 1.55 }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const boom = eo(seg(t, shatterTime, shatterTime + 0.85));
    const fade = 1 - seg(t, shatterTime + 0.45, shatterTime + 0.92);
    if (fade <= 0) return;

    // Shockwave ring on shatter
    if (boom > 0 && boom < 1) {
      const r = lerp(40, 1180, boom);
      ctx.save();
      ctx.globalAlpha = (1 - boom) * 0.85;
      ctx.strokeStyle = "#fbb303";
      ctx.lineWidth = lerp(18, 1.5, boom);
      ctx.beginPath();
      ctx.arc(CX, CY + 40, r, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }

    // 72 3D geometric prism shards bursting out of the 3 hotkey caps
    let seed = 9001;
    const rnd = () => {
      seed = (seed * 1664525 + 1013904223) >>> 0;
      return seed / 4294967296;
    };

    if (boom > 0.01) {
      for (let i = 0; i < 84; i++) {
        const originX = (i % 3 === 0 ? -520 : i % 3 === 1 ? 0 : 520) + (rnd() - 0.5) * 280;
        const originY = (rnd() - 0.5) * 160;
        const vx = originX * (1.6 + rnd() * 1.4) + (rnd() - 0.5) * 500;
        const vy = originY * 2.2 - 180 + rnd() * 520;
        const vz = 240 + rnd() * 680;

        const x = lerp(originX, originX + vx, boom);
        const y = lerp(originY, originY + vy + 280 * boom * boom, boom);
        const z = lerp(0, vz, boom);

        const rx = boom * (3 + rnd() * 8);
        const ry = boom * (3 + rnd() * 8);
        const rz = boom * (2 + rnd() * 6);
        const sz = 14 + rnd() * 26;

        const c0 = project3D(x - sz, y - sz, z, rx, ry, rz);
        const c1 = project3D(x + sz, y - sz, z, rx, ry, rz);
        const c2 = project3D(x + sz, y + sz, z, rx, ry, rz);
        const c3 = project3D(x - sz, y + sz, z, rx, ry, rz);

        ctx.globalAlpha = fade * (0.45 + 0.55 * rnd());
        ctx.fillStyle =
          i % 4 === 0 ? "#fbb303" : i % 4 === 1 ? "#ff6b5e" : "#dfe5ec";
        ctx.beginPath();
        ctx.moveTo(c0.sx, c0.sy);
        ctx.lineTo(c1.sx, c1.sy);
        ctx.lineTo(c2.sx, c2.sy);
        ctx.lineTo(c3.sx, c3.sy);
        ctx.closePath();
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    }
  }, [t, shatterTime]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 30,
      }}
    />
  );
};

// ============================================================================
// 3. BEAT 2: 3D PARAMETRIC S-RIBBON SCULPTURE (Morphs: Iris -> Helix -> Arrow)
// ============================================================================
export const ParametricRibbon3DCanvas: React.FC<{
  t: number;
  mode: 0 | 1 | 2;
  rms: number;
}> = ({ t, mode, rms }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const N = 220;
    const m01 = eio(seg(t, 1.08, 1.48));
    const m12 = eio(seg(t, 2.02, 2.42));

    // Compute 3D centerline points of the ribbon across the 3 geometric states
    const pts: {
      x: number;
      y: number;
      z: number;
      nx: number;
      ny: number;
      nz: number;
      u: number;
    }[] = [];

    for (let i = 0; i <= N; i++) {
      const u = i / N;
      const ang = u * Math.PI * 4;

      // State 0 ("Look."): 3D Torus Iris Ring
      const r0 = 260 + 48 * Math.cos(u * Math.PI * 6 + t * 3);
      const x0 = r0 * Math.cos(u * Math.PI * 2);
      const y0 = r0 * Math.sin(u * Math.PI * 2);
      const z0 = 95 * Math.sin(u * Math.PI * 4 + t * 2);

      // State 1 ("Talk."): 3D Acoustic Double-Helix Wave
      const x1 = (u - 0.5) * 1280;
      const env = Math.sin(u * Math.PI);
      const amp = (150 + rms * 130) * env;
      const y1 = amp * Math.sin(ang * 2.2 - t * 9);
      const z1 = amp * Math.cos(ang * 2.2 - t * 9);

      // State 2 ("Sent."): 3D Folded S-Ribbon + Forward Launch Vector
      const sCurve = Math.sin((u - 0.5) * Math.PI * 2);
      const x2 = sCurve * 340;
      const y2 = (0.5 - u) * 520;
      const z2 = Math.cos(u * Math.PI * 2) * 160;

      const xa = lerp(x0, x1, m01);
      const ya = lerp(y0, y1, m01);
      const za = lerp(z0, z1, m01);

      const x = lerp(xa, x2, m12);
      const y = lerp(ya, y2, m12);
      const z = lerp(za, z2, m12);

      // Ribbon width normal twisted along u (2*PI keeps closed torus seam continuous)
      const twist = u * Math.PI * 2 + t * 1.4;
      const w = 34 * (0.45 + 0.55 * Math.sin(u * Math.PI));
      pts.push({
        x,
        y,
        z,
        nx: Math.cos(twist) * w,
        ny: Math.sin(twist) * w,
        nz: Math.cos(twist * 0.7) * w * 0.5,
        u,
      });
    }

    const quads: {
      p0: { sx: number; sy: number };
      p1: { sx: number; sy: number };
      p2: { sx: number; sy: number };
      p3: { sx: number; sy: number };
      z: number;
      u: number;
    }[] = [];

    const rx = 0.22 * Math.sin(t * 0.9);
    const ry = 0.35 * Math.cos(t * 0.7);
    const rz = 0.08 * Math.sin(t * 0.5);

    for (let i = 0; i < N; i++) {
      const a = pts[i];
      const b = pts[i + 1];
      const aL = project3D(a.x - a.nx, a.y - a.ny, a.z - a.nz, rx, ry, rz, CX, CY - 30);
      const aR = project3D(a.x + a.nx, a.y + a.ny, a.z + a.nz, rx, ry, rz, CX, CY - 30);
      const bR = project3D(b.x + b.nx, b.y + b.ny, b.z + b.nz, rx, ry, rz, CX, CY - 30);
      const bL = project3D(b.x - b.nx, b.y - b.ny, b.z - b.nz, rx, ry, rz, CX, CY - 30);

      quads.push({
        p0: aL,
        p1: aR,
        p2: bR,
        p3: bL,
        z: (aL.z + bR.z) * 0.5,
        u: a.u,
      });
    }

    quads.sort((a, b) => a.z - b.z);

    for (const q of quads) {
      const torusArc = 0.5 - 0.5 * Math.cos(q.u * Math.PI * 2);
      const helixRamp = eio(seg(q.u, 0.52, 0.86));
      const morphMix = clamp(m01 + m12, 0, 1);
      const tipGrad = lerp(torusArc * 0.72, helixRamp, morphMix);
      const amberMix = clamp(tipGrad * (0.65 + 0.35 * m12) + m12 * 0.78, 0, 1);
      const shade = clamp((q.z + 260) / 520, 0.25, 1);
      ctx.beginPath();
      ctx.moveTo(q.p0.sx, q.p0.sy);
      ctx.lineTo(q.p1.sx, q.p1.sy);
      ctx.lineTo(q.p2.sx, q.p2.sy);
      ctx.lineTo(q.p3.sx, q.p3.sy);
      ctx.closePath();

      const silV = lerp(75, 228, shade);
      const ambR = lerp(180, 251, shade);
      const ambG = lerp(95, 179, shade);
      const ambB = lerp(8, 3, shade);

      const r = Math.round(lerp(silV, ambR, amberMix));
      const g = Math.round(lerp(silV * 0.94, ambG, amberMix));
      const b = Math.round(lerp(silV, ambB, amberMix));
      const a = lerp(0.82, 0.90, amberMix);
      const fillCol = `rgba(${r}, ${g}, ${b}, ${a.toFixed(3)})`;
      ctx.fillStyle = fillCol;
      ctx.fill();
      ctx.strokeStyle = fillCol;
      ctx.lineWidth = 0.75;
      ctx.stroke();
    }
  }, [t, mode, rms]);

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
// 4. BEAT 3: PROCEDURAL 3D 478-POINT BIOMETRIC FACE TOPOLOGY & GAZE FRUSTUM
// ============================================================================
export const BiometricFaceMesh3DCanvas: React.FC<{
  t: number;
  rms: number;
}> = ({ t, rms }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const lockU = eio(seg(t, 0.15, 1.05));
    // Head rotates from turned-away (-0.52 rad yaw) into dead-center screen lock (-0.025 rad)
    const yaw = lerp(-0.54, -0.024, lockU) + 0.02 * Math.sin(t * 2.1);
    const pitch = lerp(0.22, 0.015, lockU) + 0.015 * Math.cos(t * 1.7);
    const roll = lerp(-0.12, 0.005, lockU);

    const faceCx = 460;
    const faceCy = 520;

    // 1. Volumetric Gaze Cone Frustum projecting from both eyes when locked
    if (lockU > 0.35) {
      const coneAlpha = seg(lockU, 0.35, 0.95) * (0.28 + rms * 0.18);
      const leftEye = project3D(-68, -42, 165, pitch, yaw, roll, faceCx, faceCy, 950);
      const rightEye = project3D(68, -42, 165, pitch, yaw, roll, faceCx, faceCy, 950);

      for (const eye of [leftEye, rightEye]) {
        const grad = ctx.createLinearGradient(eye.sx, eye.sy, 1180, faceCy);
        grad.addColorStop(0, `rgba(251, 179, 3, ${coneAlpha * 1.6})`);
        grad.addColorStop(0.5, `rgba(184, 204, 228, ${coneAlpha * 0.7})`);
        grad.addColorStop(1, "rgba(251, 179, 3, 0)");
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.moveTo(eye.sx, eye.sy);
        ctx.lineTo(1160, faceCy - 250);
        ctx.lineTo(1160, faceCy + 250);
        ctx.closePath();
        ctx.fill();
      }
    }

    // 2. Generate 3D parameterized facial surface topology (18 latitude x 22 longitude)
    const rows = 16;
    const cols = 20;
    const grid: { sx: number; sy: number; z: number; region: string }[][] = [];

    for (let r = 0; r <= rows; r++) {
      const v = r / rows; // 0 = forehead, 1 = chin
      const phi = (v - 0.5) * Math.PI * 0.88;
      const rowPts: { sx: number; sy: number; z: number; region: string }[] = [];

      for (let c = 0; c <= cols; c++) {
        const u = c / cols; // 0 = left ear, 1 = right ear
        const theta = (u - 0.5) * Math.PI * 0.92;

        // Sculpt anatomically recognizable 3D face geometry
        const jawTaper = 1 - 0.28 * Math.pow(Math.max(0, v - 0.45), 1.4);
        let x = 185 * Math.sin(theta) * Math.cos(phi) * jawTaper;
        let y = 245 * Math.sin(phi);
        let z = 175 * Math.cos(theta) * Math.cos(phi);

        // Nose bridge protrusion
        const noseDistX = Math.abs(u - 0.5);
        const noseInY = v > 0.36 && v < 0.64;
        if (noseInY && noseDistX < 0.12) {
          z += (1 - noseDistX / 0.12) * 38 * Math.sin(((v - 0.36) / 0.28) * Math.PI);
        }

        // Eye socket indentation
        const eyeDistL = Math.hypot(u - 0.33, v - 0.4);
        const eyeDistR = Math.hypot(u - 0.67, v - 0.4);
        if (eyeDistL < 0.1 || eyeDistR < 0.1) {
          z -= 18;
        }

        // Animated lip aperture movement
        const mouthDist = Math.hypot((u - 0.5) * 1.4, (v - 0.74) * 2.2);
        let region = "skin";
        if (eyeDistL < 0.095 || eyeDistR < 0.095) region = "eye";
        if (mouthDist < 0.18) {
          region = "lips";
          const lipOpen = (0.25 + rms * 0.95) * 18;
          y += (v > 0.74 ? 1 : -1) * lipOpen * (1 - mouthDist / 0.18);
          z += 14;
        }

        const p = project3D(x, y, z, pitch, yaw, roll, faceCx, faceCy, 950);
        rowPts.push({ ...p, region });
      }
      grid.push(rowPts);
    }

    // Draw triangulated wireframe mesh
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        const p00 = grid[r][c];
        const p10 = grid[r + 1][c];
        const p01 = grid[r][c + 1];
        const p11 = grid[r + 1][c + 1];

        const isFeature =
          p00.region === "eye" ||
          p00.region === "lips" ||
          p11.region === "eye" ||
          p11.region === "lips";

        ctx.strokeStyle = isFeature
          ? `rgba(251, 179, 3, ${0.55 + 0.35 * lockU})`
          : `rgba(184, 204, 228, ${0.18 + 0.16 * lockU})`;
        ctx.lineWidth = isFeature ? 1.5 : 0.85;

        ctx.beginPath();
        ctx.moveTo(p00.sx, p00.sy);
        ctx.lineTo(p01.sx, p01.sy);
        ctx.lineTo(p11.sx, p11.sy);
        ctx.lineTo(p00.sx, p00.sy);
        ctx.lineTo(p10.sx, p10.sy);
        ctx.lineTo(p11.sx, p11.sy);
        ctx.stroke();
      }
    }

    // Draw glowing landmark vertices
    for (let r = 0; r <= rows; r++) {
      for (let c = 0; c <= cols; c++) {
        const p = grid[r][c];
        const isHot = p.region === "eye" || p.region === "lips";
        ctx.fillStyle = isHot ? "#fbb303" : "#dfe5ec";
        ctx.globalAlpha = isHot ? 0.95 : 0.48;
        const rad = isHot ? 3.2 : 1.7;
        ctx.beginPath();
        ctx.arc(p.sx, p.sy, rad, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    ctx.globalAlpha = 1;

    // 3. Biometric Target Reticle Ring around Face
    ctx.save();
    ctx.translate(faceCx, faceCy);
    ctx.rotate(t * 0.25);
    ctx.strokeStyle = lockU > 0.6 ? "#fbb303" : "#b8cce4";
    ctx.lineWidth = 2;
    ctx.setLineDash([18, 12]);
    ctx.beginPath();
    ctx.arc(0, 0, 315, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }, [t, rms]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 15,
      }}
    />
  );
};

// ============================================================================
// 5. BEAT 4: 3D PERSPECTIVE ACOUSTIC FREQUENCY WATERFALL TERRAIN
// ============================================================================
export const AcousticTerrain3DCanvas: React.FC<{
  frame: number;
}> = ({ frame }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const rows = 26;
    const cols = 64;

    for (let r = rows - 1; r >= 0; r--) {
      const histFrame = clamp(frame - r * 2, 0, 1799);
      const env = AUDIO_RMS[histFrame] ?? 0.25;
      const pk = AUDIO_PEAK[histFrame] ?? 0.35;
      const z = 280 - r * 58;

      ctx.beginPath();
      for (let c = 0; c <= cols; c++) {
        const u = c / cols;
        const x = (u - 0.5) * 2100;
        const bell = Math.exp(-Math.pow((u - 0.5) * 3.1, 2));
        const wave =
          Math.sin(u * 24 + histFrame * 0.28) * 0.55 +
          Math.cos(u * 41 - histFrame * 0.19) * 0.45;
        const amp = (28 + env * 155 + pk * 75) * bell;
        const y = 370 - Math.abs(wave) * amp;

        const p = project3D(x, y, z, 0.16, 0, 0, CX, CY, 1050);
        if (c === 0) ctx.moveTo(p.sx, p.sy);
        else ctx.lineTo(p.sx, p.sy);
      }

      const alpha = clamp(1 - r / rows, 0.08, 0.92);
      ctx.strokeStyle =
        r < 5
          ? `rgba(251, 179, 3, ${alpha})`
          : `rgba(42, 8, 35, ${alpha * 0.38})`;
      ctx.lineWidth = r === 0 ? 3.2 : 1.5;
      ctx.stroke();
    }
  }, [frame]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 2,
      }}
    />
  );
};

// ============================================================================
// 6. BEAT 6: 3D HYPERSPACE DATA WARP CONDUIT & SHOCKWAVE
// ============================================================================
export const LightSpeedWarpCanvas: React.FC<{
  t: number;
}> = ({ t }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const warpU = seg(t, 0.05, 1.05);
    if (warpU <= 0 || warpU >= 1) return;
    const fade = 1 - seg(t, 0.65, 1.02);

    let sd = 4242;
    const rn = () => {
      sd = (sd * 1664525 + 1013904223) >>> 0;
      return sd / 4294967296;
    };

    ctx.save();
    for (let i = 0; i < 180; i++) {
      const ang = rn() * Math.PI * 2;
      const rad = 40 + rn() * 680;
      const z0 = -800 + rn() * 1200;
      const z1 = z0 + warpU * 1450;
      const tailZ = z1 - 240 * (1 - warpU * 0.4);

      const x = Math.cos(ang) * rad;
      const y = Math.sin(ang) * rad * 0.62;

      const pHead = project3D(x, y, z1, 0, 0, 0, CX, CY, 900);
      const pTail = project3D(x, y, tailZ, 0, 0, 0, CX, CY, 900);

      ctx.globalAlpha = fade * (0.35 + 0.65 * rn());
      ctx.strokeStyle = i % 3 === 0 ? "#fbb303" : "#dfe5ec";
      ctx.lineWidth = i % 4 === 0 ? 3.2 : 1.6;
      ctx.beginPath();
      ctx.moveTo(pTail.sx, pTail.sy);
      ctx.lineTo(pHead.sx, pHead.sy);
      ctx.stroke();
    }
    ctx.restore();
  }, [t]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 45,
      }}
    />
  );
};

// ============================================================================
// 7. BEAT 10: 3D HEXAGONAL LASER CONTAINMENT FORCE-FIELD
// ============================================================================
export const HexShieldForcefieldCanvas: React.FC<{
  t: number;
}> = ({ t }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const activeU = eo(seg(t, 1.35, 1.95));
    if (activeU <= 0) return;

    const pulse = 0.75 + 0.25 * Math.sin(t * 9);
    const shieldCx = 1425;
    const shieldCy = 372;

    // Glowing red/amber containment rings around the blocked Enter key
    for (let ring = 0; ring < 3; ring++) {
      const r = (74 + ring * 26) * activeU;
      ctx.save();
      ctx.translate(shieldCx, shieldCy);
      ctx.rotate((ring % 2 === 0 ? 1 : -1) * t * (0.8 + ring * 0.3));
      ctx.strokeStyle = ring === 0 ? "#ff6b5e" : "#fbb303";
      ctx.globalAlpha = (0.65 - ring * 0.18) * pulse * activeU;
      ctx.lineWidth = ring === 0 ? 3.5 : 2;

      // Draw hexagon
      ctx.beginPath();
      for (let k = 0; k < 6; k++) {
        const a = (k * Math.PI) / 3;
        const hx = Math.cos(a) * r;
        const hy = Math.sin(a) * r;
        if (k === 0) ctx.moveTo(hx, hy);
        else ctx.lineTo(hx, hy);
      }
      ctx.closePath();
      ctx.stroke();
      ctx.restore();
    }
  }, [t]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 35,
      }}
    />
  );
};

// ============================================================================
// 8. BEAT 12: FIBONACCI 3D GOLDEN VORTEX CONVERGENCE
// ============================================================================
export const FibonacciVortexCanvas: React.FC<{
  t: number;
  rms: number;
}> = ({ t, rms }) => {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, VW, VH);

    const N = 260;
    const goldenAngle = Math.PI * (3 - Math.sqrt(5));
    const settle = eo(seg(t, 0, 1.35));

    for (let i = 0; i < N; i++) {
      const frac = i / N;
      const baseR = lerp(820, 290 + 340 * Math.sqrt(frac), settle);
      const theta = i * goldenAngle + t * (0.45 - frac * 0.22);
      const z = Math.sin(frac * Math.PI * 4 + t * 2) * (140 * (1 - settle * 0.5));

      const x = Math.cos(theta) * baseR;
      const y = Math.sin(theta) * baseR * 0.68;

      const p = project3D(x, y, z, 0.25 * (1 - settle), 0, 0, CX, CY - 40, 1100);
      const rad = (1.6 + (1 - frac) * 3.2 + rms * 2.2) * p.scale;

      ctx.globalAlpha = clamp(0.2 + (1 - frac) * 0.65, 0.15, 0.85);
      ctx.fillStyle = i % 3 === 0 ? "#fbb303" : "#dfe5ec";
      ctx.beginPath();
      ctx.arc(p.sx, p.sy, rad, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
  }, [t, rms]);

  return (
    <canvas
      ref={ref}
      width={VW}
      height={VH}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        zIndex: 2,
      }}
    />
  );
};
