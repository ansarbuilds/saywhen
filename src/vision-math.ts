// Landmark math for the camera gate. The live loop in main.ts owns the gesture window.

export interface Point2 {
  x: number;
  y: number;
}

export function distance2D(a: Point2, b: Point2): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return Math.sqrt(dx * dx + dy * dy);
}

const MAR_YAW_CORRECTION_MIN_COS = 0.4;

export function computeMar(landmarks: Point2[], cameraYawDeg = 0): number {
  const vCenter = distance2D(landmarks[13], landmarks[14]);
  const vLeft = distance2D(landmarks[82], landmarks[87]);
  const vRight = distance2D(landmarks[312], landmarks[317]);
  const horizSeen = distance2D(landmarks[78], landmarks[308]);
  const cosYaw = Math.max(MAR_YAW_CORRECTION_MIN_COS, Math.cos((cameraYawDeg * Math.PI) / 180));
  const horiz = horizSeen / cosYaw;
  if (horiz < 1e-6) return 0;
  return (vCenter + vLeft + vRight) / (3.0 * horiz);
}

export function extractHeadPose(matrixData: number[]): { yaw: number; pitch: number } {
  if (matrixData.length < 12) {
    return { yaw: 0, pitch: 0 };
  }
  const r20 = matrixData[2];
  const r21 = matrixData[6];
  const r22 = matrixData[10];
  const yawRad = Math.atan2(-r20, Math.hypot(r21, r22));
  const pitchRad = Math.atan2(r21, r22);
  return {
    yaw: yawRad * (180 / Math.PI),
    pitch: pitchRad * (180 / Math.PI),
  };
}

export function countAngleReversals(values: number[], minSwingDeg: number): number {
  if (values.length < 3) return 0;
  let anchor = values[0];
  let lastDir = 0;
  let reversals = 0;

  for (let i = 1; i < values.length; i++) {
    const v = values[i];
    const diff = v - anchor;
    if (lastDir === 0) {
      if (Math.abs(diff) >= minSwingDeg) {
        lastDir = diff > 0 ? 1 : -1;
        anchor = v;
      }
    } else if (lastDir === 1) {
      if (v > anchor) {
        anchor = v;
      } else if (anchor - v >= minSwingDeg) {
        reversals += 1;
        lastDir = -1;
        anchor = v;
      }
    } else if (v < anchor) {
      anchor = v;
    } else if (v - anchor >= minSwingDeg) {
      reversals += 1;
      lastDir = 1;
      anchor = v;
    }
  }
  return reversals;
}

export function classifyHeadGesture(yaws: number[], pitches: number[]): 'nod' | 'shake' | 'none' {
  if (yaws.length < 6 || pitches.length < 6) return 'none';
  const yawSpan = Math.max(...yaws) - Math.min(...yaws);
  const pitchSpan = Math.max(...pitches) - Math.min(...pitches);
  const yawReversals = countAngleReversals(yaws, 10.0);
  const pitchReversals = countAngleReversals(pitches, 7.5);
  if (yawReversals >= 2 && yawSpan >= 18.0 && yawSpan > pitchSpan * 1.35) return 'shake';
  if (pitchReversals >= 1 && pitchSpan >= 11.0 && pitchSpan > yawSpan * 1.15) return 'nod';
  return 'none';
}

export function medianValue(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}
