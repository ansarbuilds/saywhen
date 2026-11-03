import { FaceLandmarker, FilesetResolver, NormalizedLandmark } from '@mediapipe/tasks-vision';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface DictationConfig {
  handsFreeEnabled: boolean;
  vadThreshold: number;
  silenceTimeoutMs: number;
  autoStopOnSilence: boolean;
  cleanupEnabled: boolean;
  previewEnabled: boolean;
  previewCountdownMs: number;
  cancelOnLookAway: boolean;
  perAppRulesEnabled: boolean;
  autoSendAllowlist: string[];
  autoInject: boolean;
  autoPressEnter: boolean;
  useClipboardPaste: boolean;
  preferredDevice: string | null;
  hudEnabled: boolean;
}

interface VisionTelemetryPayload {
  facePresent: boolean;
  facingScreen: boolean;
  lipsMoving: boolean;
  mouthClosed: boolean;
  mar: number;
  marVariance: number;
  yaw: number;
  pitch: number;
  jawOpen: number;
  gesture: string;
}

interface ModelInfo {
  id: string;
  filename: string;
  label: string;
  sizeMb: number;
  installed: boolean;
  path: string;
}

interface SystemStatusSnapshot {
  state: string;
  devices: string[];
  config: DictationConfig;
  accessibilityTrusted: boolean;
  models: ModelInfo[];
  selectedModelId: string;
  hotkeyLabel: string;
  frontmostApp: string;
}

interface AudioTelemetryEvent {
  state: string;
  rms: number;
  vadProb: number;
  isSpeech: boolean;
  preRollMs: number;
  listeningDurationMs: number;
  previewRemainingMs: number;
  deviceName: string;
  speechDetectedInUtterance: boolean;
  visionActive: boolean;
  visionArmed: boolean;
  visionLipsMoving: boolean;
  cameraEnabled: boolean;
  hudEnabled: boolean;
}

interface TranscriptionResult {
  text: string;
  audioDurationMs: number;
  inferenceMs: number;
  rtf: number;
  modelUsed: string;
  peakVadProb: number;
  endPhraseTriggered: boolean;
}

interface CleanupResult {
  rawText: string;
  cleanedText: string;
  modified: boolean;
  modeUsed: string;
  isDestructiveCommand: boolean;
}

interface AppTargetPolicy {
  appName: string;
  policy: string;
  willPressEnter: boolean;
  reason: string;
}

interface PreviewStartedEvent {
  transcription: TranscriptionResult;
  cleanup: CleanupResult;
  targetPolicy: AppTargetPolicy;
  countdownMs: number;
  triggerSource: string;
}

interface InjectionOutcome {
  success: boolean;
  method: string;
  pressedEnter: boolean;
  targetApp: string;
  policyApplied: string;
  accessibilityTrusted: boolean;
  message: string;
}

interface UtteranceCompleteEvent {
  transcription: TranscriptionResult;
  cleanup: CleanupResult;
  targetPolicy: AppTargetPolicy;
  injection: InjectionOutcome | null;
  injectionError: string | null;
  triggerSource: string;
  timestampMs: number;
}

// App State
let currentConfig: DictationConfig = {
  handsFreeEnabled: true,
  vadThreshold: 0.45,
  silenceTimeoutMs: 650,
  autoStopOnSilence: true,
  cleanupEnabled: true,
  previewEnabled: true,
  previewCountdownMs: 1800,
  cancelOnLookAway: false,
  perAppRulesEnabled: true,
  autoSendAllowlist: [
    'saywhen',
    'Terminal',
    'iTerm2',
    'Warp',
    'Ghostty',
    'Cursor',
    'Code',
    'Zed',
    'Antigravity',
    'Claude',
  ],
  autoInject: true,
  autoPressEnter: true,
  useClipboardPaste: true,
  preferredDevice: null,
  hudEnabled: true,
};

let modelsCatalog: ModelInfo[] = [];
let selectedModelId = 'tiny.en';
let activePreviewTotalMs = 1800;
let simulatedPreviewTimer: number | null = null;

// Stage 3 & 4 Telemetry
let totalTriggersCount = 0;
let falseTriggersCount = 0;
let canceledSendsCount = 0;

// Vision Gate State
let faceLandmarker: FaceLandmarker | null = null;
let cameraRunning = false;
let mediaStream: MediaStream | null = null;
let lastVisionFrameTime = 0;
let lastLipsMovingTimestamp = 0;
let lastGestureTime = 0;
let visionTicker: Worker | null = null;
let releaseVisionKeepAlive: (() => void) | null = null;

// The vision loop is driven by a Worker timer instead of requestAnimationFrame.
// rAF stops entirely when the main window is hidden or occluded, and main-thread
// timers get throttled, but messages posted from a dedicated worker keep arriving.
// This is what lets the camera gate keep working while saywhen runs in the background.
const VISION_TICK_MS = 66; // ~15 FPS
const VISION_TICKER_SRC = `
  let id = null;
  onmessage = (e) => {
    if (id !== null) { clearInterval(id); id = null; }
    if (e.data && e.data.intervalMs) {
      id = setInterval(() => postMessage(0), e.data.intervalMs);
    }
  };
`;

function startVisionTicker() {
  stopVisionTicker();
  const url = URL.createObjectURL(new Blob([VISION_TICKER_SRC], { type: 'text/javascript' }));
  visionTicker = new Worker(url);
  URL.revokeObjectURL(url);
  visionTicker.onmessage = () => processVisionFrame(performance.now());
  visionTicker.postMessage({ intervalMs: VISION_TICK_MS });

  // A held Web Lock exempts the page from tab freezing in Chromium based webviews
  // (WebView2 on Windows), which the backgroundThrottling config cannot control there.
  if (navigator.locks && !releaseVisionKeepAlive) {
    void navigator.locks
      .request(
        'saywhen-vision-keepalive',
        () =>
          new Promise<void>((resolve) => {
            releaseVisionKeepAlive = resolve;
          })
      )
      .catch(() => {
        releaseVisionKeepAlive = null;
      });
  }
}

function stopVisionTicker() {
  if (visionTicker) {
    visionTicker.postMessage({ intervalMs: 0 });
    visionTicker.terminate();
    visionTicker = null;
  }
  if (releaseVisionKeepAlive) {
    releaseVisionKeepAlive();
    releaseVisionKeepAlive = null;
  }
}

const marWindow: { timestamp: number; mar: number }[] = [];
const poseWindow: { timestamp: number; yaw: number; pitch: number }[] = [];
const chartHistory: { vadProb: number; rms: number }[] = [];
const MAX_CHART_POINTS = 120;

// DOM Elements
const prerollVal = document.getElementById('preroll-val') as HTMLElement;
const accessibilityPill = document.getElementById('accessibility-pill') as HTMLButtonElement;
const axDot = document.getElementById('ax-dot') as HTMLElement;
const axText = document.getElementById('ax-text') as HTMLElement;
const pipelineBadge = document.getElementById('pipeline-badge') as HTMLElement;
const pipelineStateLabel = document.getElementById('pipeline-state-label') as HTMLElement;

const stepIdle = document.getElementById('step-idle') as HTMLElement;
const stepArmed = document.getElementById('step-armed') as HTMLElement;
const stepListening = document.getElementById('step-listening') as HTMLElement;
const stepTranscribing = document.getElementById('step-transcribing') as HTMLElement;
const stepPreview = document.getElementById('step-preview') as HTMLElement;

const previewHud = document.getElementById('preview-hud') as HTMLElement;
const previewPolicyBadge = document.getElementById('preview-policy-badge') as HTMLElement;
const previewCountdownText = document.getElementById('preview-countdown-text') as HTMLElement;
const previewSendBtn = document.getElementById('preview-send-btn') as HTMLButtonElement;
const previewCancelBtn = document.getElementById('preview-cancel-btn') as HTMLButtonElement;
const previewTextInput = document.getElementById('preview-text-input') as HTMLInputElement;
const previewRawDiff = document.getElementById('preview-raw-diff') as HTMLElement;
const previewRawText = document.getElementById('preview-raw-text') as HTMLElement;
const previewProgressFill = document.getElementById('preview-progress-fill') as HTMLElement;

const commandDeck = document.getElementById('command-deck') as HTMLElement;
const deckStatus = document.getElementById('deck-status') as HTMLElement;
const deckSubtitle = document.getElementById('deck-subtitle') as HTMLElement;
const utteranceTimer = document.getElementById('utterance-timer') as HTMLElement;
const canceledSendsStat = document.getElementById('canceled-sends-stat') as HTMLElement;
const falseTriggerStat = document.getElementById('false-trigger-stat') as HTMLElement;
const activeModelPill = document.getElementById('active-model-pill') as HTMLElement;

const toggleCameraBtn = document.getElementById('toggle-camera-btn') as HTMLButtonElement;
const toggleCameraLabel = document.getElementById('toggle-camera-label') as HTMLElement;
const startListenBtn = document.getElementById('start-listen-btn') as HTMLButtonElement;
const stopListenBtn = document.getElementById('stop-listen-btn') as HTMLButtonElement;
const cancelListenBtn = document.getElementById('cancel-listen-btn') as HTMLButtonElement;

const webcamVideo = document.getElementById('webcam-video') as HTMLVideoElement;
const webcamOverlay = document.getElementById('webcam-overlay') as HTMLCanvasElement;
const overlayCtx = webcamOverlay.getContext('2d')!;
const webcamPlaceholder = document.getElementById('webcam-placeholder') as HTMLElement;
const gazePill = document.getElementById('gaze-pill') as HTMLElement;
const lipPill = document.getElementById('lip-pill') as HTMLElement;
const valPose = document.getElementById('val-pose') as HTMLElement;
const valMar = document.getElementById('val-mar') as HTMLElement;
const valGesture = document.getElementById('val-gesture') as HTMLElement;

const errorBanner = document.getElementById('error-banner') as HTMLElement;
const errorBannerText = document.getElementById('error-banner-text') as HTMLElement;
const errorDismissBtn = document.getElementById('error-dismiss-btn') as HTMLButtonElement;

const vadCanvas = document.getElementById('vad-canvas') as HTMLCanvasElement;
const vadCtx = vadCanvas.getContext('2d')!;

const valVadProb = document.getElementById('val-vad-prob') as HTMLElement;
const vadStateTag = document.getElementById('vad-state-tag') as HTMLElement;
const valRms = document.getElementById('val-rms') as HTMLElement;
const micDeviceTag = document.getElementById('mic-device-tag') as HTMLElement;
const valLatency = document.getElementById('val-latency') as HTMLElement;
const rtfTag = document.getElementById('rtf-tag') as HTMLElement;

const terminalHistory = document.getElementById('terminal-history') as HTMLElement;
const terminalInput = document.getElementById('terminal-input') as HTMLInputElement;
const demoCleanupBtn = document.getElementById('demo-cleanup-btn') as HTMLButtonElement;
const clearTerminalBtn = document.getElementById('clear-terminal-btn') as HTMLButtonElement;

const deviceSelect = document.getElementById('device-select') as HTMLSelectElement;
const modelSelect = document.getElementById('model-select') as HTMLSelectElement;
const downloadModelBtn = document.getElementById('download-model-btn') as HTMLButtonElement;

const vadThreshSlider = document.getElementById('vad-thresh-slider') as HTMLInputElement;
const vadThreshDisplay = document.getElementById('vad-thresh-display') as HTMLElement;
const previewCountdownSlider = document.getElementById('preview-countdown-slider') as HTMLInputElement;
const previewCountdownDisplay = document.getElementById('preview-countdown-display') as HTMLElement;
const allowlistInput = document.getElementById('allowlist-input') as HTMLInputElement;

const toggleCleanup = document.getElementById('toggle-cleanup') as HTMLInputElement;
const togglePreview = document.getElementById('toggle-preview') as HTMLInputElement;
const togglePerApp = document.getElementById('toggle-per-app') as HTMLInputElement;
const toggleHandsFree = document.getElementById('toggle-hands-free') as HTMLInputElement;
const toggleAutoEnter = document.getElementById('toggle-auto-enter') as HTMLInputElement;
const toggleHud = document.getElementById('toggle-hud') as HTMLInputElement;

const transcriptList = document.getElementById('transcript-list') as HTMLElement;
const emptyFeed = document.getElementById('empty-feed') as HTMLElement;
const clearFeedBtn = document.getElementById('clear-feed-btn') as HTMLButtonElement;

const onboardingOpenBtn = document.getElementById('onboarding-open-btn') as HTMLButtonElement;
const onboardingModal = document.getElementById('onboarding-modal') as HTMLElement;
const onboardingCloseBtn = document.getElementById('onboarding-close-btn') as HTMLButtonElement;
const onboardingDoneBtn = document.getElementById('onboarding-done-btn') as HTMLButtonElement;
const obAxBadge = document.getElementById('ob-ax-badge') as HTMLElement;

function formatDurationMs(ms: number): string {
  const totalTenths = Math.floor(ms / 100);
  const tenths = totalTenths % 10;
  const totalSeconds = Math.floor(totalTenths / 10);
  const seconds = totalSeconds % 60;
  const minutes = Math.floor(totalSeconds / 60);
  const mm = String(minutes).padStart(2, '0');
  const ss = String(seconds).padStart(2, '0');
  return `${mm}:${ss}.${tenths}`;
}

function showError(msg: string) {
  errorBannerText.textContent = msg;
  errorBanner.classList.remove('hidden');
}

function updateStatsDisplay() {
  const falseRate =
    totalTriggersCount > 0 ? ((falseTriggersCount / totalTriggersCount) * 100).toFixed(1) : '0.0';
  falseTriggerStat.textContent = `${falseTriggersCount} (${falseRate}%)`;
  canceledSendsStat.textContent = `${canceledSendsCount} saved`;
}

function updateAccessibilityBadge(trusted: boolean) {
  if (trusted) {
    axDot.className = 'dot dot-emerald';
    axText.textContent = 'Accessibility: Trusted';
    if (obAxBadge) {
      obAxBadge.textContent = 'TRUSTED';
      obAxBadge.className = 'ob-badge ob-ok';
    }
  } else {
    axDot.className = 'dot dot-amber';
    axText.textContent = 'Grant Accessibility Permission';
    if (obAxBadge) {
      obAxBadge.textContent = 'CLICK TO GRANT';
      obAxBadge.className = 'ob-badge';
    }
  }
}

function renderModelsDropdown() {
  modelSelect.innerHTML = '';
  for (const m of modelsCatalog) {
    const opt = document.createElement('option');
    opt.value = m.id;
    opt.textContent = m.installed
      ? `${m.label} [Installed]`
      : `${m.label} [Click Download]`;
    if (m.id === selectedModelId) {
      opt.selected = true;
    }
    modelSelect.appendChild(opt);
  }
  activeModelPill.textContent = `whisper-${selectedModelId}`;
  updateModelDownloadButtonState();
}

function updateModelDownloadButtonState() {
  const chosenId = modelSelect.value;
  const info = modelsCatalog.find((m) => m.id === chosenId);
  if (info && !info.installed) {
    downloadModelBtn.classList.remove('hidden');
    downloadModelBtn.textContent = `Download ${info.id}`;
  } else {
    downloadModelBtn.classList.add('hidden');
  }
}

function showPreviewBanner(ev: PreviewStartedEvent) {
  activePreviewTotalMs = ev.countdownMs || 1800;
  previewHud.classList.remove('hidden');
  previewTextInput.value = ev.cleanup.cleanedText;

  if (ev.cleanup.modified) {
    previewRawDiff.classList.remove('hidden');
    previewRawText.textContent = ev.cleanup.rawText;
  } else {
    previewRawDiff.classList.add('hidden');
  }

  previewPolicyBadge.className = 'preview-policy-badge';
  if (ev.targetPolicy.policy === 'destructive_guard') {
    previewPolicyBadge.classList.add('policy-danger');
    previewPolicyBadge.textContent = `DESTRUCTIVE GUARD: Paste-Only (${ev.targetPolicy.appName})`;
  } else if (ev.targetPolicy.policy === 'paste_only' || !ev.targetPolicy.willPressEnter) {
    previewPolicyBadge.classList.add('policy-warn');
    previewPolicyBadge.textContent = `Target: ${ev.targetPolicy.appName} (Paste-Only)`;
  } else {
    previewPolicyBadge.textContent = `Target: ${ev.targetPolicy.appName} (Paste + Auto-Enter)`;
  }

  previewCountdownText.textContent = `Sending in ${(activePreviewTotalMs / 1000).toFixed(1)}s...`;
  previewProgressFill.style.width = '100%';
}

function hidePreviewBanner() {
  if (simulatedPreviewTimer !== null) {
    clearInterval(simulatedPreviewTimer);
    simulatedPreviewTimer = null;
  }
  previewHud.classList.add('hidden');
}

function updatePipelineUiState(
  state: string,
  listeningDurationMs: number,
  previewRemainingMs: number
) {
  pipelineStateLabel.textContent = state;
  pipelineBadge.className = 'gate-status-badge';
  commandDeck.classList.remove('is-armed', 'is-listening');

  stepIdle.classList.toggle('active', state === 'IDLE');
  stepArmed.classList.toggle('active', state === 'ARMED');
  stepListening.classList.toggle('active', state === 'LISTENING');
  stepTranscribing.classList.toggle('active', state === 'TRANSCRIBING');
  stepPreview.classList.toggle('active', state === 'PREVIEW');

  if (state === 'PREVIEW') {
    const pct = Math.max(0, Math.min(100, (previewRemainingMs / activePreviewTotalMs) * 100));
    previewProgressFill.style.width = `${pct}%`;
    previewCountdownText.textContent = `Sending in ${(previewRemainingMs / 1000).toFixed(1)}s...`;
    deckStatus.textContent = 'Preview Window: Review or Cancel';
    deckSubtitle.textContent = 'Press Esc or shake head to cancel; press Enter or nod to send now';
    startListenBtn.disabled = true;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = false;
  } else if (state === 'ARMED') {
    pipelineBadge.classList.add('state-armed');
    commandDeck.classList.add('is-armed');
    deckStatus.textContent = 'Armed: Facing Screen (Speak to Trigger)';
    deckSubtitle.textContent =
      '500 ms audio ring buffer active. Start talking to trigger hands-free recording.';
    utteranceTimer.textContent = '00:00.0';
    startListenBtn.disabled = false;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = true;
  } else if (state === 'LISTENING') {
    pipelineBadge.classList.add('state-listening');
    commandDeck.classList.add('is-listening');
    deckStatus.textContent = 'Listening & Capturing Utterance...';
    deckSubtitle.textContent =
      'Close mouth + pause for 650 ms, say "send it", or nod head to transcribe';
    utteranceTimer.textContent = formatDurationMs(listeningDurationMs);
    startListenBtn.disabled = true;
    stopListenBtn.disabled = false;
    cancelListenBtn.disabled = false;
  } else if (state === 'TRANSCRIBING') {
    pipelineBadge.classList.add('state-transcribing');
    deckStatus.textContent = 'Transcribing & Running Developer Cleanup...';
    deckSubtitle.textContent = 'Running Metal whisper.cpp + code identifier cleanup pass';
    startListenBtn.disabled = true;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = true;
  } else {
    deckStatus.textContent = cameraRunning
      ? 'Idle: Look at Screen to Arm'
      : 'Ready for Voice Input';
    deckSubtitle.innerHTML = cameraRunning
      ? 'Face not facing screen cone. Turn toward monitor to arm hands-free gate.'
      : 'Click <strong>Enable Camera Gate</strong> for hands-free look-and-talk, or press <kbd>Option</kbd> + <kbd>Space</kbd>';
    utteranceTimer.textContent = '00:00.0';
    startListenBtn.disabled = false;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = true;
  }
}

// MediaPipe Vision Gate Math
function distance2D(a: NormalizedLandmark, b: NormalizedLandmark): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return Math.sqrt(dx * dx + dy * dy);
}

function computeMAR(landmarks: NormalizedLandmark[]): number {
  const vCenter = distance2D(landmarks[13], landmarks[14]);
  const vLeft = distance2D(landmarks[82], landmarks[87]);
  const vRight = distance2D(landmarks[312], landmarks[317]);
  const horiz = distance2D(landmarks[78], landmarks[308]);
  if (horiz < 1e-6) return 0;
  return (vCenter + vLeft + vRight) / (3.0 * horiz);
}

function extractHeadPose(matrixData: number[]): { yaw: number; pitch: number } {
  const r02 = matrixData[8];
  const r12 = matrixData[9];
  const r22 = matrixData[10];
  const yawRad = Math.atan2(r02, r22);
  const pitchRad = Math.asin(-Math.max(-1, Math.min(1, r12)));
  return {
    yaw: yawRad * (180 / Math.PI),
    pitch: pitchRad * (180 / Math.PI),
  };
}

function detectHeadGesture(now: number): string {
  if (now - lastGestureTime < 1200 || poseWindow.length < 6) {
    return 'none';
  }
  const yaws = poseWindow.map((p) => p.yaw);
  const pitches = poseWindow.map((p) => p.pitch);
  const yawSpan = Math.max(...yaws) - Math.min(...yaws);
  const pitchSpan = Math.max(...pitches) - Math.min(...pitches);

  if (yawSpan >= 16.0 && yawSpan > pitchSpan * 1.4) {
    lastGestureTime = now;
    return 'shake';
  }
  if (pitchSpan >= 11.0 && pitchSpan > yawSpan * 1.3) {
    lastGestureTime = now;
    return 'nod';
  }
  return 'none';
}

async function ensureFaceLandmarker(): Promise<FaceLandmarker> {
  if (faceLandmarker) return faceLandmarker;
  const vision = await FilesetResolver.forVisionTasks('/wasm');
  faceLandmarker = await FaceLandmarker.createFromOptions(vision, {
    baseOptions: {
      modelAssetPath: '/models/face_landmarker.task',
      delegate: 'GPU',
    },
    outputFaceBlendshapes: true,
    outputFacialTransformationMatrixes: true,
    runningMode: 'VIDEO',
    numFaces: 1,
  });
  return faceLandmarker;
}

async function toggleCameraGate() {
  if (cameraRunning) {
    cameraRunning = false;
    stopVisionTicker();
    if (mediaStream) {
      mediaStream.getTracks().forEach((t) => t.stop());
      mediaStream = null;
    }
    webcamVideo.srcObject = null;
    webcamPlaceholder.classList.remove('hidden');
    toggleCameraBtn.classList.remove('camera-active');
    toggleCameraLabel.textContent = 'Enable Camera Gate';
    gazePill.textContent = 'GAZE: OFF';
    gazePill.className = 'gauge-tag';
    lipPill.textContent = 'LIPS: STILL';
    lipPill.className = 'gauge-tag';

    try {
      await invoke('set_camera_gate_enabled', { enabled: false });
    } catch (err) {
      showError(`Failed to pause camera gate in backend: ${String(err)}`);
    }
    return;
  }

  try {
    toggleCameraBtn.disabled = true;
    toggleCameraLabel.textContent = 'Loading MediaPipe...';
    await ensureFaceLandmarker();

    mediaStream = await navigator.mediaDevices.getUserMedia({
      video: { width: { ideal: 640 }, height: { ideal: 360 }, frameRate: { ideal: 15 } },
      audio: false,
    });
    webcamVideo.srcObject = mediaStream;
    await webcamVideo.play();

    await invoke('set_camera_gate_enabled', { enabled: true });
    cameraRunning = true;
    webcamPlaceholder.classList.add('hidden');
    toggleCameraBtn.classList.add('camera-active');
    toggleCameraLabel.textContent = 'Pause Camera Gate';
    startVisionTicker();
  } catch (err) {
    showError(`Camera initialization error: ${String(err)}`);
    toggleCameraLabel.textContent = 'Enable Camera Gate';
  } finally {
    toggleCameraBtn.disabled = false;
  }
}

function drawLipLandmarks(landmarks: NormalizedLandmark[], facing: boolean, lipsMoving: boolean) {
  const w = webcamOverlay.width;
  const h = webcamOverlay.height;
  overlayCtx.clearRect(0, 0, w, h);

  const lipIndices = [78, 308, 13, 14, 82, 87, 312, 317];
  overlayCtx.fillStyle = lipsMoving ? '#f43f5e' : facing ? '#10b981' : '#f59e0b';
  for (const idx of lipIndices) {
    const pt = landmarks[idx];
    if (!pt) continue;
    overlayCtx.beginPath();
    overlayCtx.arc(pt.x * w, pt.y * h, 3, 0, Math.PI * 2);
    overlayCtx.fill();
  }
}

function processVisionFrame(now: number) {
  if (!cameraRunning || !faceLandmarker) return;

  // Drop ticks that bunch up after a stall; detectForVideo needs increasing timestamps.
  if (now - lastVisionFrameTime < VISION_TICK_MS * 0.8) return;
  lastVisionFrameTime = now;

  // Some webviews pause <video> playback when the window is hidden. Resume it so frames keep flowing.
  if (webcamVideo.paused && mediaStream) {
    void webcamVideo.play().catch(() => undefined);
  }

  // When the main window is hidden, skip DOM and canvas work and only feed the gate.
  const renderUi = !document.hidden;

  if (webcamVideo.readyState >= 2) {
    if (
      renderUi &&
      (webcamOverlay.width !== webcamVideo.videoWidth ||
        webcamOverlay.height !== webcamVideo.videoHeight)
    ) {
      webcamOverlay.width = webcamVideo.videoWidth || 640;
      webcamOverlay.height = webcamVideo.videoHeight || 360;
    }

    const result = faceLandmarker.detectForVideo(webcamVideo, now);
    if (result.faceLandmarks && result.faceLandmarks.length > 0) {
      const landmarks = result.faceLandmarks[0];
      const mar = computeMAR(landmarks);

      marWindow.push({ timestamp: now, mar });
      while (marWindow.length > 0 && now - marWindow[0].timestamp > 320) {
        marWindow.shift();
      }

      const meanMar = marWindow.reduce((acc, p) => acc + p.mar, 0) / marWindow.length;
      const marVariance =
        marWindow.reduce((acc, p) => acc + Math.pow(p.mar - meanMar, 2), 0) / marWindow.length;

      let yaw = 0;
      let pitch = 0;
      if (
        result.facialTransformationMatrixes &&
        result.facialTransformationMatrixes.length > 0
      ) {
        const pose = extractHeadPose(
          Array.from(result.facialTransformationMatrixes[0].data)
        );
        yaw = pose.yaw;
        pitch = pose.pitch;
      }

      poseWindow.push({ timestamp: now, yaw, pitch });
      while (poseWindow.length > 0 && now - poseWindow[0].timestamp > 750) {
        poseWindow.shift();
      }

      let jawOpen = 0;
      if (result.faceBlendshapes && result.faceBlendshapes.length > 0) {
        const categories = result.faceBlendshapes[0].categories;
        jawOpen = categories.find((c) => c.categoryName === 'jawOpen')?.score ?? 0;
      }

      const facingScreen = Math.abs(yaw) <= 22.0 && Math.abs(pitch) <= 20.0;
      const isYawning = mar > 0.45 || jawOpen > 0.35;
      const activeLipMotion = !isYawning && mar >= 0.018 && marVariance >= 0.000012;

      if (activeLipMotion) {
        lastLipsMovingTimestamp = now;
      }
      const lipsMoving = facingScreen && !isYawning && now - lastLipsMovingTimestamp <= 300;
      const mouthClosed = mar < 0.032 && marVariance < 0.00004;
      const gesture = detectHeadGesture(now);

      if (renderUi) {
        drawLipLandmarks(landmarks, facingScreen, lipsMoving);

        valPose.textContent = `${yaw.toFixed(1)}° / ${pitch.toFixed(1)}°`;
        valMar.textContent = `${mar.toFixed(3)} / ${(marVariance * 1000).toFixed(2)}`;
        if (gesture !== 'none') {
          valGesture.textContent = gesture.toUpperCase();
        }

        gazePill.textContent = facingScreen ? 'GAZE: ON SCREEN' : 'GAZE: AWAY';
        gazePill.className = facingScreen ? 'gauge-tag tag-ok' : 'gauge-tag tag-warn';

        lipPill.textContent = isYawning
          ? 'LIPS: YAWN CEILING'
          : lipsMoving
            ? 'LIPS: MOVING'
            : 'LIPS: STILL';
        lipPill.className = lipsMoving ? 'gauge-tag tag-ok' : 'gauge-tag';
      }

      void invoke('update_vision_telemetry', {
        telemetry: {
          facePresent: true,
          facingScreen,
          lipsMoving,
          mouthClosed,
          mar,
          marVariance,
          yaw,
          pitch,
          jawOpen,
          gesture,
        } satisfies VisionTelemetryPayload,
      });
    } else {
      if (renderUi) {
        overlayCtx.clearRect(0, 0, webcamOverlay.width, webcamOverlay.height);
        gazePill.textContent = 'NO FACE';
        gazePill.className = 'gauge-tag tag-warn';
        lipPill.textContent = 'LIPS: --';
        lipPill.className = 'gauge-tag';
      }

      void invoke('update_vision_telemetry', {
        telemetry: {
          facePresent: false,
          facingScreen: false,
          lipsMoving: false,
          mouthClosed: true,
          mar: 0,
          marVariance: 0,
          yaw: 0,
          pitch: 0,
          jawOpen: 0,
          gesture: 'none',
        } satisfies VisionTelemetryPayload,
      });
    }
  }
}

function drawVadChart() {
  const dpr = window.devicePixelRatio || 1;
  const rect = vadCanvas.getBoundingClientRect();
  if (
    vadCanvas.width !== Math.floor(rect.width * dpr) ||
    vadCanvas.height !== Math.floor(rect.height * dpr)
  ) {
    vadCanvas.width = Math.floor(rect.width * dpr);
    vadCanvas.height = Math.floor(rect.height * dpr);
  }

  const ctx = vadCtx;
  const w = vadCanvas.width;
  const h = vadCanvas.height;
  ctx.clearRect(0, 0, w, h);

  ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
  ctx.lineWidth = 1;
  for (const frac of [0.25, 0.5, 0.75]) {
    const y = h * frac;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
    ctx.stroke();
  }

  const threshY = h - currentConfig.vadThreshold * (h - 16) - 8;
  ctx.save();
  ctx.strokeStyle = 'rgba(244, 63, 94, 0.65)';
  ctx.setLineDash([5 * dpr, 5 * dpr]);
  ctx.lineWidth = 1.5 * dpr;
  ctx.beginPath();
  ctx.moveTo(0, threshY);
  ctx.lineTo(w, threshY);
  ctx.stroke();
  ctx.restore();

  if (chartHistory.length < 2) return;

  const stepX = w / (MAX_CHART_POINTS - 1);

  ctx.beginPath();
  for (let i = 0; i < chartHistory.length; i++) {
    const x = i * stepX;
    const normRms = Math.min(1.0, chartHistory[i].rms * 6.0);
    const y = h - normRms * (h - 16) - 8;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.strokeStyle = '#10b981';
  ctx.lineWidth = 1.8 * dpr;
  ctx.stroke();

  ctx.beginPath();
  for (let i = 0; i < chartHistory.length; i++) {
    const x = i * stepX;
    const prob = Math.max(0, Math.min(1.0, chartHistory[i].vadProb));
    const y = h - prob * (h - 16) - 8;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.strokeStyle = '#6366f1';
  ctx.lineWidth = 2.4 * dpr;
  ctx.stroke();
}

function appendUtteranceCard(ev: UtteranceCompleteEvent) {
  hidePreviewBanner();
  emptyFeed.classList.add('hidden');
  totalTriggersCount += 1;
  updateStatsDisplay();

  const { transcription, cleanup, targetPolicy, injection, injectionError, triggerSource, timestampMs } = ev;
  valLatency.textContent = `${transcription.inferenceMs} ms`;
  rtfTag.textContent = `RTF ${transcription.rtf.toFixed(2)}x`;

  const card = document.createElement('div');
  card.className = 'utterance-card';

  const timeStr = new Date(timestampMs || Date.now()).toLocaleTimeString();
  const displayTranscript =
    cleanup.cleanedText.length > 0 ? cleanup.cleanedText : '(No speech detected in audio segment)';

  const injectBadgeHtml = injection?.success
    ? `<span class="u-badge u-badge-ok">${targetPolicy.appName}: ${injection.pressedEnter ? 'Pasted + Enter' : 'Paste-Only'}</span>`
    : injectionError
      ? `<span class="u-badge">Clipboard Fallback</span>`
      : `<span class="u-badge">Transcribed</span>`;

  const cleanupBadgeHtml = cleanup.modified
    ? `<span class="u-badge u-badge-ok">Cleaned CLI/Code</span>`
    : '';

  card.innerHTML = `
    <div class="utterance-meta">
      <span>${timeStr} (${triggerSource})</span>
      <div class="utterance-badges">
        <span class="u-badge">${transcription.inferenceMs} ms (${transcription.rtf.toFixed(2)}x)</span>
        ${cleanupBadgeHtml}
        ${injectBadgeHtml}
      </div>
    </div>
    <div class="utterance-text"></div>
    <div class="utterance-actions">
      <button class="btn-xs btn-flag-false" title="Mark this utterance as an unintended false trigger">Flag False Trigger</button>
      <button class="btn-xs btn-copy">Copy</button>
      <button class="btn-xs btn-accent btn-delayed-inject" title="Click into your terminal within 2s to type this">Inject in 2s</button>
    </div>
  `;

  const textEl = card.querySelector('.utterance-text') as HTMLElement;
  textEl.textContent = displayTranscript;

  let flagged = false;
  const flagBtn = card.querySelector('.btn-flag-false') as HTMLButtonElement;
  flagBtn.addEventListener('click', () => {
    flagged = !flagged;
    if (flagged) {
      falseTriggersCount += 1;
      card.classList.add('flagged-false');
      flagBtn.textContent = 'Flagged False Trigger';
    } else {
      falseTriggersCount = Math.max(0, falseTriggersCount - 1);
      card.classList.remove('flagged-false');
      flagBtn.textContent = 'Flag False Trigger';
    }
    updateStatsDisplay();
  });

  const copyBtn = card.querySelector('.btn-copy') as HTMLButtonElement;
  copyBtn.addEventListener('click', async () => {
    await navigator.clipboard.writeText(cleanup.cleanedText);
    copyBtn.textContent = 'Copied!';
    setTimeout(() => {
      copyBtn.textContent = 'Copy';
    }, 1200);
  });

  const delayedInjectBtn = card.querySelector('.btn-delayed-inject') as HTMLButtonElement;
  delayedInjectBtn.addEventListener('click', async () => {
    if (!cleanup.cleanedText) return;
    delayedInjectBtn.disabled = true;
    delayedInjectBtn.textContent = 'Focus Terminal (2s)...';
    try {
      await invoke('manual_inject_text', {
        text: cleanup.cleanedText,
        pressEnter: currentConfig.autoPressEnter,
        useClipboardPaste: currentConfig.useClipboardPaste,
        delayMs: 2000,
      });
      delayedInjectBtn.textContent = 'Injected!';
    } catch (err) {
      showError(String(err));
      delayedInjectBtn.textContent = 'Failed';
    } finally {
      setTimeout(() => {
        delayedInjectBtn.disabled = false;
        delayedInjectBtn.textContent = 'Inject in 2s';
      }, 1500);
    }
  });

  transcriptList.prepend(card);
}

async function syncConfigToBackend() {
  const allowlist = allowlistInput.value
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  currentConfig = {
    ...currentConfig,
    handsFreeEnabled: toggleHandsFree.checked,
    vadThreshold: parseFloat(vadThreshSlider.value),
    previewCountdownMs: parseInt(previewCountdownSlider.value, 10),
    cleanupEnabled: toggleCleanup.checked,
    previewEnabled: togglePreview.checked,
    perAppRulesEnabled: togglePerApp.checked,
    autoSendAllowlist: allowlist,
    autoPressEnter: toggleAutoEnter.checked,
    hudEnabled: toggleHud.checked,
    preferredDevice: deviceSelect.value ? deviceSelect.value : null,
  };

  vadThreshDisplay.textContent = currentConfig.vadThreshold.toFixed(2);
  previewCountdownDisplay.textContent = `${currentConfig.previewCountdownMs} ms`;

  try {
    await invoke('update_dictation_config', { config: currentConfig });
  } catch (err) {
    showError(`Failed to update config: ${String(err)}`);
  }
}

async function initApp() {
  try {
    const snapshot = await invoke<SystemStatusSnapshot>('get_system_status');
    currentConfig = snapshot.config;
    modelsCatalog = snapshot.models;
    selectedModelId = snapshot.selectedModelId;

    updateAccessibilityBadge(snapshot.accessibilityTrusted);

    deviceSelect.innerHTML = '';
    const defaultOpt = document.createElement('option');
    defaultOpt.value = '';
    defaultOpt.textContent = 'System Default Microphone';
    deviceSelect.appendChild(defaultOpt);

    for (const dev of snapshot.devices) {
      const opt = document.createElement('option');
      opt.value = dev;
      opt.textContent = dev;
      if (currentConfig.preferredDevice === dev) {
        opt.selected = true;
      }
      deviceSelect.appendChild(opt);
    }

    renderModelsDropdown();

    vadThreshSlider.value = String(currentConfig.vadThreshold);
    vadThreshDisplay.textContent = currentConfig.vadThreshold.toFixed(2);
    previewCountdownSlider.value = String(currentConfig.previewCountdownMs);
    previewCountdownDisplay.textContent = `${currentConfig.previewCountdownMs} ms`;
    allowlistInput.value = currentConfig.autoSendAllowlist.join(', ');

    toggleCleanup.checked = currentConfig.cleanupEnabled;
    togglePreview.checked = currentConfig.previewEnabled;
    togglePerApp.checked = currentConfig.perAppRulesEnabled;
    toggleHandsFree.checked = currentConfig.handsFreeEnabled;
    toggleAutoEnter.checked = currentConfig.autoPressEnter;
    toggleHud.checked = currentConfig.hudEnabled;

    updatePipelineUiState(snapshot.state, 0, 0);
  } catch (err) {
    showError(`Backend connection warning: ${String(err)}`);
  }

  await listen<AudioTelemetryEvent>('audio-telemetry', (event) => {
    const t = event.payload;
    updatePipelineUiState(t.state, t.listeningDurationMs, t.previewRemainingMs);

    valVadProb.textContent = t.vadProb.toFixed(2);
    if (t.isSpeech) {
      vadStateTag.textContent = 'SPEECH';
      vadStateTag.className = 'gauge-tag tag-ok';
    } else {
      vadStateTag.textContent = 'SILENCE';
      vadStateTag.className = 'gauge-tag';
    }

    valRms.textContent = t.rms.toFixed(3);
    micDeviceTag.textContent = t.deviceName;
    prerollVal.textContent = `${t.preRollMs} ms`;

    chartHistory.push({ vadProb: t.vadProb, rms: t.rms });
    if (chartHistory.length > MAX_CHART_POINTS) {
      chartHistory.shift();
    }
    if (!document.hidden) {
      drawVadChart();
    }
  });

  await listen<PreviewStartedEvent>('preview-started', (event) => {
    showPreviewBanner(event.payload);
  });

  await listen<string>('preview-canceled', () => {
    hidePreviewBanner();
    canceledSendsCount += 1;
    updateStatsDisplay();
  });

  await listen<UtteranceCompleteEvent>('utterance-complete', (event) => {
    appendUtteranceCard(event.payload);
  });

  await listen<string>('pipeline-error', (event) => {
    showError(event.payload);
  });

  await listen('tray-toggle-camera', () => {
    void toggleCameraGate();
  });

  await listen<DictationConfig>('dictation-config-updated', (event) => {
    currentConfig = event.payload;
    toggleHud.checked = currentConfig.hudEnabled;
  });
}

// Event Listeners
toggleCameraBtn.addEventListener('click', toggleCameraGate);

startListenBtn.addEventListener('click', async () => {
  try {
    await invoke('start_listening', { source: 'manual_button' });
  } catch (err) {
    showError(String(err));
  }
});

stopListenBtn.addEventListener('click', async () => {
  try {
    await invoke('stop_and_transcribe');
  } catch (err) {
    showError(String(err));
  }
});

cancelListenBtn.addEventListener('click', async () => {
  try {
    await invoke('cancel_preview', { reason: 'User clicked Cancel' });
    await invoke('cancel_listening');
    hidePreviewBanner();
  } catch (err) {
    showError(String(err));
  }
});

previewSendBtn.addEventListener('click', async () => {
  if (simulatedPreviewTimer !== null) {
    clearInterval(simulatedPreviewTimer);
    simulatedPreviewTimer = null;
    const text = previewTextInput.value.trim();
    hidePreviewBanner();
    if (text) {
      const line = document.createElement('div');
      line.className = 'term-line term-cmd';
      line.textContent = `saywhen@mac ~ % ${text}`;
      terminalHistory.appendChild(line);
      terminalHistory.scrollTop = terminalHistory.scrollHeight;
    }
    return;
  }
  try {
    await invoke('confirm_preview_now', { overrideText: previewTextInput.value });
  } catch (err) {
    showError(String(err));
  }
});

previewCancelBtn.addEventListener('click', async () => {
  if (simulatedPreviewTimer !== null) {
    clearInterval(simulatedPreviewTimer);
    simulatedPreviewTimer = null;
    hidePreviewBanner();
    canceledSendsCount += 1;
    updateStatsDisplay();
    return;
  }
  try {
    await invoke('cancel_preview', { reason: 'User canceled in Preview HUD' });
  } catch (err) {
    showError(String(err));
  }
});

window.addEventListener('keydown', async (e) => {
  if (e.key === 'Escape' && !previewHud.classList.contains('hidden')) {
    e.preventDefault();
    previewCancelBtn.click();
  }
});

previewTextInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    previewSendBtn.click();
  }
});

demoCleanupBtn.addEventListener('click', async () => {
  try {
    const sample = 'Um, Cargo test dash dash release slash src slash main dot rs.';
    const cleanup = await invoke<CleanupResult>('test_cleanup_pass', { sampleText: sample });
    const totalMs = currentConfig.previewCountdownMs || 1800;
    let remainingMs = totalMs;

    showPreviewBanner({
      transcription: {
        text: sample,
        audioDurationMs: 2400,
        inferenceMs: 145,
        rtf: 0.06,
        modelUsed: selectedModelId,
        peakVadProb: 0.94,
        endPhraseTriggered: true,
      },
      cleanup,
      targetPolicy: {
        appName: 'saywhen (Terminal Sandbox)',
        policy: 'auto_send',
        willPressEnter: true,
        reason: 'Terminal allowlist',
      },
      countdownMs: totalMs,
      triggerSource: 'simulated_demo',
    });

    if (simulatedPreviewTimer !== null) clearInterval(simulatedPreviewTimer);
    simulatedPreviewTimer = window.setInterval(() => {
      remainingMs -= 60;
      if (remainingMs <= 0) {
        previewSendBtn.click();
      } else {
        const pct = Math.max(0, (remainingMs / totalMs) * 100);
        previewProgressFill.style.width = `${pct}%`;
        previewCountdownText.textContent = `Sending in ${(remainingMs / 1000).toFixed(1)}s...`;
      }
    }, 60);
  } catch (err) {
    showError(String(err));
  }
});

accessibilityPill.addEventListener('click', async () => {
  try {
    await invoke('request_accessibility_settings');
  } catch (err) {
    showError(String(err));
  }
});

errorDismissBtn.addEventListener('click', () => {
  errorBanner.classList.add('hidden');
});

deviceSelect.addEventListener('change', syncConfigToBackend);
vadThreshSlider.addEventListener('input', syncConfigToBackend);
previewCountdownSlider.addEventListener('input', syncConfigToBackend);
allowlistInput.addEventListener('change', syncConfigToBackend);
toggleCleanup.addEventListener('change', syncConfigToBackend);
togglePreview.addEventListener('change', syncConfigToBackend);
togglePerApp.addEventListener('change', syncConfigToBackend);
toggleHandsFree.addEventListener('change', syncConfigToBackend);
toggleAutoEnter.addEventListener('change', syncConfigToBackend);
toggleHud.addEventListener('change', syncConfigToBackend);

modelSelect.addEventListener('change', async () => {
  const chosenId = modelSelect.value;
  updateModelDownloadButtonState();
  const info = modelsCatalog.find((m) => m.id === chosenId);
  if (info && info.installed) {
    try {
      await invoke('select_whisper_model', { modelId: chosenId });
      selectedModelId = chosenId;
      activeModelPill.textContent = `whisper-${selectedModelId}`;
    } catch (err) {
      showError(String(err));
    }
  }
});

downloadModelBtn.addEventListener('click', async () => {
  const chosenId = modelSelect.value;
  downloadModelBtn.disabled = true;
  downloadModelBtn.textContent = 'Downloading...';
  try {
    const updated = await invoke<ModelInfo>('download_whisper_model', { modelId: chosenId });
    const idx = modelsCatalog.findIndex((m) => m.id === updated.id);
    if (idx !== -1) {
      modelsCatalog[idx] = updated;
    }
    await invoke('select_whisper_model', { modelId: chosenId });
    selectedModelId = chosenId;
    renderModelsDropdown();
  } catch (err) {
    showError(`Model download failed: ${String(err)}`);
  } finally {
    downloadModelBtn.disabled = false;
    updateModelDownloadButtonState();
  }
});

terminalInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    const cmd = terminalInput.value.trim();
    if (!cmd) return;
    const line = document.createElement('div');
    line.className = 'term-line term-cmd';
    line.textContent = `saywhen@mac ~ % ${cmd}`;
    terminalHistory.appendChild(line);
    terminalHistory.scrollTop = terminalHistory.scrollHeight;
    terminalInput.value = '';
  }
});

clearTerminalBtn.addEventListener('click', () => {
  terminalHistory.innerHTML =
    '<div class="term-line term-muted"># Focus the input line below, speak or click "Simulate Cleanup Preview"</div>';
});

clearFeedBtn.addEventListener('click', () => {
  transcriptList.innerHTML = '';
  transcriptList.appendChild(emptyFeed);
  emptyFeed.classList.remove('hidden');
});

onboardingOpenBtn.addEventListener('click', () => {
  onboardingModal.classList.remove('hidden');
});

onboardingCloseBtn.addEventListener('click', () => {
  onboardingModal.classList.add('hidden');
});

onboardingDoneBtn.addEventListener('click', () => {
  onboardingModal.classList.add('hidden');
});

window.addEventListener('resize', drawVadChart);

function applyDocumentationDemoState(demoMode: string) {
  const stageTagEl = document.querySelector('.stage-tag') as HTMLElement | null;
  deviceSelect.innerHTML = '<option selected>MacBook Pro Microphone (16 kHz)</option>';
  modelSelect.innerHTML = '<option selected>Whisper Tiny.en (Fastest, ~75 MB) [Installed]</option>';

  updateAccessibilityBadge(true);
  prerollVal.textContent = '500 ms';
  micDeviceTag.textContent = 'MacBook Pro Mic (16 kHz)';
  valLatency.textContent = '128 ms';
  rtfTag.textContent = 'RTF 0.09x';

  // Generate realistic VAD + RMS waveform
  chartHistory.length = 0;
  for (let i = 0; i < MAX_CHART_POINTS; i++) {
    const inSpeechBurst = (i > 22 && i < 58) || (i > 74 && i < 112);
    const vadProb = inSpeechBurst
      ? 0.78 + 0.18 * Math.sin(i * 0.35)
      : 0.03 + 0.02 * Math.cos(i * 0.2);
    const rms = inSpeechBurst
      ? 0.08 + 0.05 * Math.abs(Math.sin(i * 0.5))
      : 0.004;
    chartHistory.push({ vadProb, rms });
  }
  drawVadChart();

  // Populate realistic Utterance Feed
  appendUtteranceCard({
    transcription: {
      text: 'git status --short',
      audioDurationMs: 1450,
      inferenceMs: 128,
      rtf: 0.09,
      modelUsed: 'tiny.en',
      peakVadProb: 0.95,
      endPhraseTriggered: true,
    },
    cleanup: {
      rawText: 'Git status dash dash short send it.',
      cleanedText: 'git status --short',
      modified: true,
      modeUsed: 'local_dev_rules',
      isDestructiveCommand: false,
    },
    targetPolicy: {
      appName: 'Terminal',
      policy: 'auto_send',
      willPressEnter: true,
      reason: 'Terminal allowlist',
    },
    injection: {
      success: true,
      method: 'clipboard_paste',
      pressedEnter: true,
      targetApp: 'Terminal',
      policyApplied: 'auto_send',
      accessibilityTrusted: true,
      message: 'Injected into Terminal and pressed Enter',
    },
    injectionError: null,
    triggerSource: 'hands_free_multimodal',
    timestampMs: Date.now() - 42000,
  });

  if (demoMode === 'stage2') {
    if (stageTagEl) stageTagEl.textContent = 'STAGE 2: HANDY-CORE AUDIO DICTATION';
    updatePipelineUiState('IDLE', 0, 0);
    valVadProb.textContent = '0.04';
    vadStateTag.textContent = 'SILENCE';
    vadStateTag.className = 'gauge-tag';
    valRms.textContent = '0.004';
  } else if (demoMode === 'stage3') {
    if (stageTagEl) stageTagEl.textContent = 'STAGE 3: GATED AUTO START/STOP';
    cameraRunning = true;
    toggleCameraBtn.classList.add('camera-active');
    toggleCameraLabel.textContent = 'Pause Camera Gate';
    webcamPlaceholder.classList.add('hidden');

    // Draw synthetic privacy-safe face & lip landmark mesh on webcam overlay
    webcamOverlay.width = 640;
    webcamOverlay.height = 360;
    overlayCtx.fillStyle = '#0b101b';
    overlayCtx.fillRect(0, 0, 640, 360);
    overlayCtx.strokeStyle = 'rgba(16, 185, 129, 0.28)';
    overlayCtx.lineWidth = 1.5;
    overlayCtx.beginPath();
    overlayCtx.ellipse(320, 175, 92, 122, 0, 0, Math.PI * 2);
    overlayCtx.stroke();

    // Synthetic lip contour & active landmarks
    const synthLips: [number, number][] = [
      [282, 228],
      [358, 228],
      [320, 214],
      [320, 244],
      [298, 218],
      [298, 240],
      [342, 218],
      [342, 240],
    ];
    overlayCtx.strokeStyle = 'rgba(244, 63, 94, 0.75)';
    overlayCtx.lineWidth = 2;
    overlayCtx.beginPath();
    overlayCtx.ellipse(320, 229, 38, 15, 0, 0, Math.PI * 2);
    overlayCtx.stroke();
    overlayCtx.fillStyle = '#f43f5e';
    for (const [lx, ly] of synthLips) {
      overlayCtx.beginPath();
      overlayCtx.arc(lx, ly, 4, 0, Math.PI * 2);
      overlayCtx.fill();
    }

    gazePill.textContent = 'GAZE: ON SCREEN';
    gazePill.className = 'gauge-tag tag-ok';
    lipPill.textContent = 'LIPS: MOVING';
    lipPill.className = 'gauge-tag tag-ok';
    valPose.textContent = '-1.8° / +2.4°';
    valMar.textContent = '0.148 / 1.62';
    valGesture.textContent = 'NONE';

    valVadProb.textContent = '0.94';
    vadStateTag.textContent = 'SPEECH';
    vadStateTag.className = 'gauge-tag tag-ok';
    valRms.textContent = '0.118';
    updatePipelineUiState('LISTENING', 2400, 0);
  } else if (demoMode === 'stage4') {
    cameraRunning = true;
    toggleCameraBtn.classList.add('camera-active');
    toggleCameraLabel.textContent = 'Pause Camera Gate';
    webcamPlaceholder.classList.add('hidden');

    webcamOverlay.width = 640;
    webcamOverlay.height = 360;
    overlayCtx.fillStyle = '#0b101b';
    overlayCtx.fillRect(0, 0, 640, 360);
    overlayCtx.strokeStyle = 'rgba(16, 185, 129, 0.35)';
    overlayCtx.lineWidth = 1.5;
    overlayCtx.beginPath();
    overlayCtx.ellipse(320, 175, 92, 122, 0, 0, Math.PI * 2);
    overlayCtx.stroke();

    overlayCtx.strokeStyle = 'rgba(16, 185, 129, 0.8)';
    overlayCtx.lineWidth = 2;
    overlayCtx.beginPath();
    overlayCtx.moveTo(284, 228);
    overlayCtx.lineTo(356, 228);
    overlayCtx.stroke();

    gazePill.textContent = 'GAZE: ON SCREEN';
    gazePill.className = 'gauge-tag tag-ok';
    lipPill.textContent = 'LIPS: CLOSED';
    lipPill.className = 'gauge-tag';
    valPose.textContent = '-0.6° / +1.2°';
    valMar.textContent = '0.021 / 0.01';
    valGesture.textContent = 'NOD READY';

    canceledSendsCount = 2;
    updateStatsDisplay();

    showPreviewBanner({
      transcription: {
        text: 'Um, Cargo test dash dash release slash src slash main dot rs.',
        audioDurationMs: 2650,
        inferenceMs: 142,
        rtf: 0.05,
        modelUsed: 'tiny.en',
        peakVadProb: 0.96,
        endPhraseTriggered: true,
      },
      cleanup: {
        rawText: 'Um, Cargo test dash dash release slash src slash main dot rs.',
        cleanedText: 'cargo test --release /src/main.rs',
        modified: true,
        modeUsed: 'local_dev_rules',
        isDestructiveCommand: false,
      },
      targetPolicy: {
        appName: 'Terminal',
        policy: 'auto_send',
        willPressEnter: true,
        reason: 'Terminal allowlist',
      },
      countdownMs: 1800,
      triggerSource: 'hands_free_multimodal',
    });

    updatePipelineUiState('PREVIEW', 0, 1350);
    terminalInput.value = 'cargo test --release /src/main.rs';
  } else if (demoMode === 'stage5') {
    if (stageTagEl) stageTagEl.textContent = 'STAGE 5: CROSS-PLATFORM POLISH & ONBOARDING';
    updatePipelineUiState('IDLE', 0, 0);
    onboardingModal.classList.remove('hidden');
  }
}

const urlParams = new URLSearchParams(window.location.search);
const demoParam = urlParams.get('demo');
if (demoParam) {
  setTimeout(() => applyDocumentationDemoState(demoParam), 150);
} else {
  initApp();
}
