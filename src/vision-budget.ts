// Keep these numbers in sync with src-tauri/src/turn_taking.rs.
// They are a frame-budget, not a measured battery percentage.

export const VISION_TICK_ACTIVE_MS = 66;
export const VISION_TICK_IDLE_AWAY_MS = 125;
export const VISION_TICK_IDLE_NO_FACE_MS = 200;
export const VISION_TICK_LOW_FACING_MS = 125;
export const VISION_TICK_LOW_AWAY_MS = 250;
export const VISION_TICK_LOW_NO_FACE_MS = 500;

export type VisionPowerMode = 'auto' | 'full' | 'low';

export function chooseVisionTickMs(
  mode: VisionPowerMode,
  adaptiveFpsEnabled: boolean,
  pipelineActive: boolean,
  facePresent: boolean,
  facingScreen: boolean,
  msSinceLastOnScreen: number,
): number {
  if (mode === 'full') return VISION_TICK_ACTIVE_MS;
  if (mode === 'low') {
    if (pipelineActive) return VISION_TICK_ACTIVE_MS;
    if (facingScreen || msSinceLastOnScreen < 3000) return VISION_TICK_LOW_FACING_MS;
    if (facePresent) return VISION_TICK_LOW_AWAY_MS;
    return VISION_TICK_LOW_NO_FACE_MS;
  }
  if (!adaptiveFpsEnabled || pipelineActive || facingScreen || msSinceLastOnScreen < 3000) {
    return VISION_TICK_ACTIVE_MS;
  }
  return facePresent ? VISION_TICK_IDLE_AWAY_MS : VISION_TICK_IDLE_NO_FACE_MS;
}

export function visionInferencesPerMinute(tickMs: number): number {
  if (tickMs <= 0) return 0;
  return Math.floor(60_000 / tickMs);
}
