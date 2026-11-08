import { FaceLandmarker, FilesetResolver, NormalizedLandmark } from '@mediapipe/tasks-vision';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './fonts';
import { isMac, refreshRangeFills, setAudioLevel, setSendModeUi, setUiState } from './ui';

type SendMode = 'pause' | 'nod' | 'phrase' | 'paste_only';

interface DictationConfig {
  handsFreeEnabled: boolean;
  vadThreshold: number;
  silenceTimeoutMs: number;
  maxSilenceTimeoutMs: number;
  autoStopOnSilence: boolean;
  cleanupEnabled: boolean;
  previewEnabled: boolean;
  previewCountdownMs: number;
  sendMode: SendMode;
  cancelOnSpeech: boolean;
  cancelOnLookAway: boolean;
  perAppRulesEnabled: boolean;
  autoSendAllowlist: string[];
  autoInject: boolean;
  autoPressEnter: boolean;
  useClipboardPaste: boolean;
  preferredDevice: string | null;
  hudEnabled: boolean;
  onTargetChanged: 'abort' | 'paste_only';
  enterDelayMs: number;
  perAppEnterDelayMs: Record<string, number>;
  clipboardRestoreDelayMs: number;
  armYawEnterDeg: number;
  armYawExitDeg: number;
  armPitchEnterDeg: number;
  armPitchExitDeg: number;
  armDwellMs: number;
  disarmDwellMs: number;
  minSpeechMs: number;
  minSpeechHotkeyMs: number;
  postRollMs: number;
  sessionMetricsEnabled: boolean;
  saveTuningAudioEnabled: boolean;
  calibratedYawOffsetDeg: number;
  calibratedPitchOffsetDeg: number;
  calibratedRestingMar: number;
  adaptiveFpsEnabled: boolean;
  sideCameraMode: boolean;
  uiPalette: string;
  moodColors: boolean;
  customVocab: string;
  historyPersistenceEnabled: boolean;
  llmCleanupEnabled: boolean;
  llmEndpointUrl: string;
  llmModel: string;
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
  recentHistory?: UtteranceCompleteEvent[];
}

interface AudioTelemetryEvent {
  state: string;
  rms: number;
  vadProb: number;
  isSpeech: boolean;
  preRollMs: number;
  listeningDurationMs: number;
  previewRemainingMs: number;
  silenceRemainingMs?: number;
  silenceTotalMs?: number;
  postRollActive?: boolean;
  deviceName: string;
  speechDetectedInUtterance: boolean;
  visionActive: boolean;
  visionArmed: boolean;
  visionLipsMoving: boolean;
  cameraEnabled: boolean;
  hudEnabled: boolean;
  saveTuningAudioEnabled?: boolean;
  sessionMetricsEnabled?: boolean;
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
  sendMode?: SendMode;
  waitsForNod?: boolean;
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
  utteranceId?: number;
  transcription: TranscriptionResult;
  cleanup: CleanupResult;
  targetPolicy: AppTargetPolicy;
  injection: InjectionOutcome | null;
  injectionError: string | null;
  triggerSource: string;
  timestampMs: number;
  flaggedFalseTrigger?: boolean;
}

// App State
let currentConfig: DictationConfig = {
  handsFreeEnabled: true,
  vadThreshold: 0.45,
  silenceTimeoutMs: 650,
  maxSilenceTimeoutMs: 1350,
  autoStopOnSilence: true,
  cleanupEnabled: true,
  previewEnabled: true,
  previewCountdownMs: 1800,
  sendMode: 'pause',
  cancelOnSpeech: true,
  cancelOnLookAway: false,
  perAppRulesEnabled: true,
  autoSendAllowlist: [
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
  onTargetChanged: 'abort',
  enterDelayMs: 120,
  perAppEnterDelayMs: {},
  clipboardRestoreDelayMs: 600,
  armYawEnterDeg: 20.0,
  armYawExitDeg: 28.0,
  armPitchEnterDeg: 18.0,
  armPitchExitDeg: 25.0,
  armDwellMs: 180,
  disarmDwellMs: 300,
  minSpeechMs: 250,
  minSpeechHotkeyMs: 96,
  postRollMs: 200,
  sessionMetricsEnabled: false,
  saveTuningAudioEnabled: false,
  calibratedYawOffsetDeg: 0.0,
  calibratedPitchOffsetDeg: 0.0,
  calibratedRestingMar: 0.016,
  adaptiveFpsEnabled: true,
  sideCameraMode: false,
  uiPalette: 'lumen',
  moodColors: true,
  customVocab: '',
  historyPersistenceEnabled: true,
  llmCleanupEnabled: false,
  llmEndpointUrl: 'http://127.0.0.1:11434/v1/chat/completions',
  llmModel: 'qwen2.5-coder:1.5b',
};

let modelsCatalog: ModelInfo[] = [];
let selectedModelId = 'tiny.en';
let activePreviewTotalMs = 1800;
let activePreviewWaitsForNod = false;
let simulatedPreviewTimer: number | null = null;

// Stage 3 & 4 Telemetry
let totalTriggersCount = 0;
let falseTriggersCount = 0;
let canceledSendsCount = 0;
let accessibilityPermissionTrusted = false;
let accessibilityPollTimer: number | null = null;

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
const VISION_TICK_ACTIVE_MS = 66; // ~15.1 FPS when armed, listening, or facing screen
const VISION_TICK_IDLE_AWAY_MS = 125; // 8.0 FPS when face present in IDLE but turned away > 3s
const VISION_TICK_IDLE_NO_FACE_MS = 200; // 5.0 FPS when no face in frame > 3s (66.7% fewer inferences)
let currentVisionTickMs = VISION_TICK_ACTIVE_MS;
let lastFaceOnScreenTimestamp = 0;
let currentPipelineState = 'IDLE';
const STATE_LABELS: Record<string, string> = {
  IDLE: 'Waiting',
  ARMED: 'Ready',
  LISTENING: 'Listening',
  TRANSCRIBING: 'Writing',
  PREVIEW: 'Check',
};
let calibratingUntilTimestamp = 0;
const calibrationSamples: { yaw: number; pitch: number; mar: number }[] = [];

const VISION_TICKER_SRC = `
  let id = null;
  onmessage = (e) => {
    if (id !== null) { clearInterval(id); id = null; }
    if (e.data && e.data.intervalMs) {
      id = setInterval(() => postMessage(0), e.data.intervalMs);
    }
  };
`;

function setVisionTickerInterval(intervalMs: number) {
  if (!visionTicker || currentVisionTickMs === intervalMs) return;
  currentVisionTickMs = intervalMs;
  visionTicker.postMessage({ intervalMs });
}

function startVisionTicker() {
  stopVisionTicker();
  const url = URL.createObjectURL(new Blob([VISION_TICKER_SRC], { type: 'text/javascript' }));
  visionTicker = new Worker(url);
  URL.revokeObjectURL(url);
  currentVisionTickMs = VISION_TICK_ACTIVE_MS;
  lastFaceOnScreenTimestamp = performance.now();
  visionTicker.onmessage = () => processVisionFrame(performance.now());
  visionTicker.postMessage({ intervalMs: currentVisionTickMs });

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
const calibrateVisionBtn = document.getElementById('calibrate-vision-btn') as HTMLButtonElement;
const resetCalibrationBtn = document.getElementById('reset-calibration-btn') as HTMLButtonElement;
const fpsPill = document.getElementById('fps-pill') as HTMLElement;
const gazePill = document.getElementById('gaze-pill') as HTMLElement;
const lipPill = document.getElementById('lip-pill') as HTMLElement;
const valPose = document.getElementById('val-pose') as HTMLElement;
const valMar = document.getElementById('val-mar') as HTMLElement;
const valGesture = document.getElementById('val-gesture') as HTMLElement;
const valCalibration = document.getElementById('val-calibration') as HTMLElement;

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
const customVocabInput = document.getElementById('custom-vocab-input') as HTMLInputElement;

const toggleCleanup = document.getElementById('toggle-cleanup') as HTMLInputElement;
const togglePreview = document.getElementById('toggle-preview') as HTMLInputElement;
const togglePerApp = document.getElementById('toggle-per-app') as HTMLInputElement;
const toggleHandsFree = document.getElementById('toggle-hands-free') as HTMLInputElement;
const toggleCancelOnSpeech = document.getElementById('toggle-cancel-on-speech') as HTMLInputElement;
const toggleSideCamera = document.getElementById('toggle-side-camera') as HTMLInputElement;
const sendModeButtons = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-send-mode]'));
const toggleHud = document.getElementById('toggle-hud') as HTMLInputElement;
const toggleAdaptiveFps = document.getElementById('toggle-adaptive-fps') as HTMLInputElement;
const toggleHistoryPersistence = document.getElementById('toggle-history-persistence') as HTMLInputElement;
const toggleLlmCleanup = document.getElementById('toggle-llm-cleanup') as HTMLInputElement;
const llmCleanupFields = document.getElementById('llm-cleanup-fields') as HTMLElement;
const llmEndpointInput = document.getElementById('llm-endpoint-input') as HTMLInputElement;
const llmModelInput = document.getElementById('llm-model-input') as HTMLInputElement;
const toggleSessionMetrics = document.getElementById('toggle-session-metrics') as HTMLInputElement;
const toggleSaveAudio = document.getElementById('toggle-save-audio') as HTMLInputElement;

let facingScreenLatched = false;

function formatSignedDeg(deg: number): string {
  const rounded = Math.abs(deg) < 0.05 ? 0 : deg;
  return `${rounded >= 0 ? '+' : ''}${rounded.toFixed(1)}°`;
}

function updateCalibrationDisplay() {
  if (!valCalibration) return;
  const yaw0 = currentConfig.calibratedYawOffsetDeg || 0;
  const pitch0 = currentConfig.calibratedPitchOffsetDeg || 0;
  const mar0 = currentConfig.calibratedRestingMar || 0.016;
  valCalibration.textContent = `${formatSignedDeg(yaw0)} / ${formatSignedDeg(pitch0)} | ${mar0.toFixed(3)}`;
}

const transcriptList = document.getElementById('transcript-list') as HTMLElement;
const emptyFeed = document.getElementById('empty-feed') as HTMLElement;
const clearFeedBtn = document.getElementById('clear-feed-btn') as HTMLButtonElement;
const historySearchInput = document.getElementById('history-search-input') as HTMLInputElement;
const historyTtlBadge = document.getElementById('history-ttl-badge') as HTMLElement;
const issuesList = document.getElementById('issues-list') as HTMLElement;
const issuesEmpty = document.getElementById('issues-empty') as HTMLElement;
const clearIssuesBtn = document.getElementById('clear-issues-btn') as HTMLButtonElement;
const issuesCountBadge = document.getElementById('issues-count-badge') as HTMLElement;
const issuesPanel = document.getElementById('issues-panel') as HTMLElement;

type SessionIssue = { id: number; message: string; timestampMs: number };
const MAX_SESSION_ISSUES = 50;
let nextIssueId = 1;
const sessionIssues: SessionIssue[] = [];

function isAccessibilityIssueMessage(message: string): boolean {
  return /accessibility/i.test(message);
}

function renderIssuesList() {
  if (issuesCountBadge) {
    issuesCountBadge.textContent = String(sessionIssues.length);
  }
  if (clearIssuesBtn) {
    clearIssuesBtn.disabled = sessionIssues.length === 0;
  }

  issuesList.querySelectorAll('.issue-row').forEach((row) => row.remove());

  if (sessionIssues.length === 0) {
    issuesEmpty.classList.remove('hidden');
    return;
  }

  issuesEmpty.classList.add('hidden');
  const frag = document.createDocumentFragment();
  for (const issue of [...sessionIssues].reverse()) {
    const timeStr = new Date(issue.timestampMs).toLocaleTimeString([], {
      hour: 'numeric',
      minute: '2-digit',
      second: '2-digit',
    });
    const row = document.createElement('div');
    row.className = 'issue-row';
    const axBtn = isAccessibilityIssueMessage(issue.message)
      ? `<button type="button" class="btn-xs btn-accent" data-open-accessibility>Open Accessibility settings</button>`
      : '';
    row.innerHTML = `
      <div class="issue-text"></div>
      <div class="issue-meta">
        <span>${escapeHtml(timeStr)}</span>
        <div class="issue-actions">
          <button type="button" class="btn-xs btn-quiet" data-copy-issue data-issue-id="${issue.id}">Copy</button>
          ${axBtn}
        </div>
      </div>
    `;
    const textEl = row.querySelector('.issue-text') as HTMLElement;
    textEl.textContent = issue.message;
    frag.appendChild(row);
  }
  issuesList.appendChild(frag);
}

function logSessionIssue(message: string) {
  const text = message.trim();
  if (!text) return;
  sessionIssues.push({ id: nextIssueId++, message: text, timestampMs: Date.now() });
  while (sessionIssues.length > MAX_SESSION_ISSUES) {
    sessionIssues.shift();
  }
  renderIssuesList();
  if (!document.hidden && issuesPanel) {
    issuesPanel.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }
}

function updateHistoryBadgeDisplay() {
  if (!historyTtlBadge) return;
  const enabled = currentConfig.historyPersistenceEnabled !== false;
  historyTtlBadge.textContent = enabled ? '7d Local' : 'RAM Only';
  historyTtlBadge.className = enabled ? 'gauge-tag tag-ok' : 'gauge-tag';
}

function filterUtteranceFeed() {
  const q = (historySearchInput?.value || '').trim().toLowerCase();
  const cards = transcriptList.querySelectorAll<HTMLElement>('.utterance-card');
  for (const card of cards) {
    const hay = card.dataset.searchText || '';
    if (!q || hay.includes(q)) {
      card.classList.remove('hidden');
    } else {
      card.classList.add('hidden');
    }
  }
}

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
  const text = String(msg ?? '').trim();
  if (!text) return;
  errorBannerText.textContent = text;
  errorBanner.classList.remove('hidden');
  logSessionIssue(text);
}

function updateStatsDisplay() {
  const falseRate =
    totalTriggersCount > 0 ? ((falseTriggersCount / totalTriggersCount) * 100).toFixed(1) : '0.0';
  falseTriggerStat.textContent = `${falseTriggersCount} (${falseRate}%)`;
  canceledSendsStat.textContent = `${canceledSendsCount} saved`;
}

function updateAccessibilityBadge(trusted: boolean) {
  accessibilityPermissionTrusted = trusted;
  if (trusted) {
    axDot.className = 'dot dot-emerald';
    axText.textContent = 'Accessibility: Trusted';
    if (obAxBadge) {
      obAxBadge.textContent = 'TRUSTED';
      obAxBadge.className = 'ob-badge ob-ok';
    }
    if (accessibilityPollTimer !== null) {
      window.clearInterval(accessibilityPollTimer);
      accessibilityPollTimer = null;
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

async function refreshAccessibilityStatus() {
  try {
    const snapshot = await invoke<SystemStatusSnapshot>('get_system_status');
    if (snapshot.accessibilityTrusted !== accessibilityPermissionTrusted) {
      updateAccessibilityBadge(snapshot.accessibilityTrusted);
    }
  } catch {
    // Backend may be busy during transcribe; ignore and retry on next tick.
  }
}

function startAccessibilityPoll() {
  if (accessibilityPermissionTrusted || accessibilityPollTimer !== null) return;
  accessibilityPollTimer = window.setInterval(() => {
    void refreshAccessibilityStatus();
  }, 2000);
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
  activePreviewWaitsForNod = Boolean(ev.waitsForNod);
  previewHud.classList.remove('hidden');
  previewTextInput.value = ev.cleanup.cleanedText;

  if (ev.cleanup.modified) {
    previewRawDiff.classList.remove('hidden');
    previewRawText.textContent = ev.cleanup.rawText;
  } else {
    previewRawDiff.classList.add('hidden');
  }

  previewPolicyBadge.className = 'preview-policy-badge';
  if (ev.targetPolicy.policy === 'self_guard') {
    previewPolicyBadge.classList.add('policy-danger');
    previewPolicyBadge.textContent = 'SELF-WINDOW GUARD: Focus a terminal or editor to inject';
  } else if (ev.targetPolicy.policy === 'destructive_guard') {
    previewPolicyBadge.classList.add('policy-danger');
    previewPolicyBadge.textContent = `DESTRUCTIVE GUARD: Paste-Only (${ev.targetPolicy.appName})`;
  } else if (ev.targetPolicy.policy === 'paste_only' || !ev.targetPolicy.willPressEnter) {
    previewPolicyBadge.classList.add('policy-warn');
    previewPolicyBadge.textContent = `Target: ${ev.targetPolicy.appName} (Paste-Only)`;
  } else {
    previewPolicyBadge.textContent = `Target: ${ev.targetPolicy.appName} (Paste + Auto-Enter)`;
  }

  previewCountdownText.textContent = previewCountdownLabel(activePreviewTotalMs);
  previewProgressFill.style.width = '100%';
}

function previewCountdownLabel(remainingMs: number): string {
  const secs = `${(Math.max(0, remainingMs) / 1000).toFixed(1)}s`;
  if (activePreviewWaitsForNod) return `Waiting for your nod (${secs})`;
  return `Sending in ${secs}...`;
}

function listeningHint(): string {
  switch (currentConfig.sendMode) {
    case 'nod':
      return 'Nod when you are done. Shake your head to cancel.';
    case 'phrase':
      return 'End with "send it" to press Enter, or just stop talking to only type it.';
    case 'paste_only':
      return 'Stop talking when you are done. Your words are typed without pressing Enter.';
    default:
      return 'Stop talking when you are done, or nod to finish now.';
  }
}

const PALETTE_IDS = ['lumen', 'bloom', 'lumora', 'neon', 'choreo', 'field', 'blue-bloom'];
const paletteButtons = Array.from(
  document.querySelectorAll<HTMLButtonElement>('.palette-card[data-palette]')
);
const toggleMoodColors = document.getElementById('toggle-mood-colors') as HTMLInputElement;

function renderPalette() {
  const palette = PALETTE_IDS.includes(currentConfig.uiPalette) ? currentConfig.uiPalette : 'lumen';
  document.body.dataset.palette = palette;
  document.body.dataset.moodColors = currentConfig.moodColors === false ? 'off' : 'on';
  for (const btn of paletteButtons) {
    const on = btn.dataset.palette === palette;
    btn.classList.toggle('is-selected', on);
    btn.setAttribute('aria-pressed', on ? 'true' : 'false');
  }
  themeColorCache.at = 0;
}

for (const btn of paletteButtons) {
  btn.addEventListener('click', () => {
    const palette = btn.dataset.palette;
    if (!palette || palette === currentConfig.uiPalette) return;
    currentConfig = { ...currentConfig, uiPalette: palette };
    renderPalette();
    void syncConfigToBackend();
  });
}

// Canvas cannot read CSS variables, so resolve the live palette tokens (they animate
// between moods) a few times a second.
const themeColorCache: { at: number; values: Record<string, [number, number, number]> } = {
  at: 0,
  values: {},
};
function themeRgba(token: '--paper' | '--on-dark' | '--accent', alpha = 1): string {
  const now = performance.now();
  if (now - themeColorCache.at > 120) {
    const style = getComputedStyle(document.documentElement);
    for (const t of ['--paper', '--on-dark', '--accent']) {
      const m = style.getPropertyValue(t).match(/[\d.]+/g);
      if (m && m.length >= 3) {
        themeColorCache.values[t] = [Number(m[0]), Number(m[1]), Number(m[2])];
      }
    }
    themeColorCache.at = now;
  }
  const [r, g, b] = themeColorCache.values[token] ?? [230, 213, 189];
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function renderSendMode() {
  for (const btn of sendModeButtons) {
    const on = btn.dataset.sendMode === currentConfig.sendMode;
    btn.classList.toggle('is-selected', on);
    btn.setAttribute('aria-checked', on ? 'true' : 'false');
  }
  setSendModeUi(currentConfig.sendMode);
}

for (const btn of sendModeButtons) {
  btn.addEventListener('click', () => {
    const mode = btn.dataset.sendMode as SendMode;
    if (mode === currentConfig.sendMode) return;
    currentConfig = { ...currentConfig, sendMode: mode };
    renderSendMode();
    void syncConfigToBackend();
  });
}
renderSendMode();

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
  previewRemainingMs: number,
  postRollActive = false
) {
  currentPipelineState = state;
  pipelineStateLabel.textContent = STATE_LABELS[state] ?? state;
  pipelineBadge.className = 'gate-status-badge';
  commandDeck.classList.remove('is-armed', 'is-listening');

  stepIdle.classList.toggle('active', state === 'IDLE');
  stepArmed.classList.toggle('active', state === 'ARMED');
  stepListening.classList.toggle('active', state === 'LISTENING');
  stepTranscribing.classList.toggle('active', state === 'TRANSCRIBING');
  stepPreview.classList.toggle('active', state === 'PREVIEW');

  setUiState(state);

  if (state === 'PREVIEW') {
    const pct = Math.max(0, Math.min(100, (previewRemainingMs / activePreviewTotalMs) * 100));
    previewProgressFill.style.width = `${pct}%`;
    previewCountdownText.textContent = previewCountdownLabel(previewRemainingMs);
    if (activePreviewWaitsForNod) {
      deckStatus.textContent = 'Nod to send';
      deckSubtitle.textContent = 'Nod or press Enter to send it. Shake your head or press Esc to throw it away.';
    } else {
      deckStatus.textContent = 'Check it, then it sends';
      deckSubtitle.textContent = currentConfig.cancelOnSpeech
        ? 'To stop it, say "cancel", shake your head or press Esc. Nod or press Enter to send now.'
        : 'To stop it, shake your head or press Esc. Nod or press Enter to send now.';
    }
    startListenBtn.disabled = true;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = false;
  } else if (state === 'ARMED') {
    pipelineBadge.classList.add('state-armed');
    commandDeck.classList.add('is-armed');
    deckStatus.textContent = 'Ready. Start talking';
    deckSubtitle.textContent =
      'You are looking at the screen. Just start talking, your first word will not be cut off.';
    utteranceTimer.textContent = '00:00.0';
    startListenBtn.disabled = false;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = true;
  } else if (state === 'LISTENING') {
    pipelineBadge.classList.add('state-listening');
    commandDeck.classList.add('is-listening');
    deckStatus.textContent = 'Listening';
    deckSubtitle.textContent = postRollActive
      ? 'Got your nod. Capturing a short tail, then transcribing...'
      : listeningHint();
    utteranceTimer.textContent = formatDurationMs(listeningDurationMs);
    startListenBtn.disabled = true;
    stopListenBtn.disabled = false;
    cancelListenBtn.disabled = false;
  } else if (state === 'TRANSCRIBING') {
    pipelineBadge.classList.add('state-transcribing');
    deckStatus.textContent = 'Writing it down';
    deckSubtitle.textContent = 'Turning your speech into text, right here on this computer.';
    startListenBtn.disabled = true;
    stopListenBtn.disabled = true;
    cancelListenBtn.disabled = true;
  } else {
    deckStatus.textContent = cameraRunning ? 'Look at the screen to start' : 'Ready when you are';
    const idleSubtitle = cameraRunning
      ? 'Look at your screen and start talking. Look away and it stops listening.'
      : `Turn on the camera to talk hands-free, or press <kbd>${isMac ? 'Option' : 'Alt'}</kbd> <kbd>Space</kbd>`;
    if (deckSubtitle.innerHTML !== idleSubtitle) {
      deckSubtitle.innerHTML = idleSubtitle;
    }
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

// When the head turns away from the camera the mouth looks narrower (by cos(yaw)) while the
// lip gap stays about the same, so a plain ratio grows with head turn. Undo that so the
// resting baseline and speech thresholds mean the same thing at 0 and at 60 degrees.
const MAR_YAW_CORRECTION_MIN_COS = 0.4; // about 66 degrees

function computeMAR(landmarks: NormalizedLandmark[], cameraYawDeg = 0): number {
  const vCenter = distance2D(landmarks[13], landmarks[14]);
  const vLeft = distance2D(landmarks[82], landmarks[87]);
  const vRight = distance2D(landmarks[312], landmarks[317]);
  const horizSeen = distance2D(landmarks[78], landmarks[308]);
  const cosYaw = Math.max(MAR_YAW_CORRECTION_MIN_COS, Math.cos((cameraYawDeg * Math.PI) / 180));
  const horiz = horizSeen / cosYaw;
  if (horiz < 1e-6) return 0;
  return (vCenter + vLeft + vRight) / (3.0 * horiz);
}

// Must match turn_taking.rs (SIDE_CAMERA_YAW_*_DEG, MAX_CALIBRATED_YAW_OFFSET_DEG).
const SIDE_CAMERA_YAW_ENTER_DEG = 35.0;
const SIDE_CAMERA_YAW_EXIT_DEG = 48.0;
const MAX_CALIBRATED_YAW_OFFSET_DEG = 65.0;

// MediaPipe Face Landmarker outputs a 4x4 column-major facial transform.
// Use the rotation row (r20, r21, r22) so pitch moves on nods; the old third-column
// asin() pitch barely changed during vertical head motion in practice.
function extractHeadPose(matrixData: number[]): { yaw: number; pitch: number } {
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
  if (pitchSpan >= 9.0 && pitchSpan > yawSpan * 1.15) {
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
    toggleCameraLabel.textContent = 'Turn on camera';
    fpsPill.textContent = 'FPS: OFF';
    fpsPill.className = 'gauge-tag';
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
    toggleCameraLabel.textContent = 'Starting camera...';
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
    toggleCameraLabel.textContent = 'Turn off camera';
    startVisionTicker();
  } catch (err) {
    showError(`Camera initialization error: ${String(err)}`);
    toggleCameraLabel.textContent = 'Turn on camera';
  } finally {
    toggleCameraBtn.disabled = false;
  }
}

function drawLipLandmarks(landmarks: NormalizedLandmark[], facing: boolean, lipsMoving: boolean) {
  const w = webcamOverlay.width;
  const h = webcamOverlay.height;
  overlayCtx.clearRect(0, 0, w, h);

  const lipIndices = [78, 308, 13, 14, 82, 87, 312, 317];
  overlayCtx.fillStyle = lipsMoving ? themeRgba('--on-dark') : facing ? '#00aebb' : themeRgba('--paper');
  for (const idx of lipIndices) {
    const pt = landmarks[idx];
    if (!pt) continue;
    overlayCtx.beginPath();
    overlayCtx.arc(pt.x * w, pt.y * h, 3, 0, Math.PI * 2);
    overlayCtx.fill();
  }
}

function medianValue(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function finalizeVisionCalibration() {
  calibratingUntilTimestamp = 0;
  calibrateVisionBtn.disabled = false;
  calibrateVisionBtn.textContent = 'Calibrate Gaze (1.5s)';
  if (calibrationSamples.length < 5) {
    showError(
      'Calibration failed: face was not continuously detected. Look at your coding monitor with your face visible and try again.'
    );
    return;
  }
  const yaw0 = Math.max(
    -MAX_CALIBRATED_YAW_OFFSET_DEG,
    Math.min(MAX_CALIBRATED_YAW_OFFSET_DEG, medianValue(calibrationSamples.map((s) => s.yaw)))
  );
  const pitch0 = Math.max(
    -35,
    Math.min(35, medianValue(calibrationSamples.map((s) => s.pitch)))
  );
  const mar0 = Math.max(
    0.006,
    Math.min(0.045, medianValue(calibrationSamples.map((s) => s.mar)))
  );
  currentConfig.calibratedYawOffsetDeg = Number(yaw0.toFixed(2));
  currentConfig.calibratedPitchOffsetDeg = Number(pitch0.toFixed(2));
  currentConfig.calibratedRestingMar = Number(mar0.toFixed(4));
  updateCalibrationDisplay();
  void syncConfigToBackend();
}

function processVisionFrame(now: number) {
  if (!cameraRunning || !faceLandmarker) return;

  // Drop ticks that bunch up after a stall; detectForVideo needs increasing timestamps.
  if (now - lastVisionFrameTime < currentVisionTickMs * 0.78) return;
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

    const inferStart = performance.now();
    const result = faceLandmarker.detectForVideo(webcamVideo, now);
    const inferMs = performance.now() - inferStart;
    const isCalibrating = calibratingUntilTimestamp > 0;

    if (result.faceLandmarks && result.faceLandmarks.length > 0) {
      const landmarks = result.faceLandmarks[0];

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

      const mar = computeMAR(landmarks, yaw);

      marWindow.push({ timestamp: now, mar });
      while (marWindow.length > 0 && now - marWindow[0].timestamp > 320) {
        marWindow.shift();
      }

      const meanMar = marWindow.reduce((acc, p) => acc + p.mar, 0) / marWindow.length;
      const marVariance =
        marWindow.reduce((acc, p) => acc + Math.pow(p.mar - meanMar, 2), 0) / marWindow.length;

      if (isCalibrating) {
        calibrationSamples.push({ yaw, pitch, mar });
        if (now >= calibratingUntilTimestamp) {
          finalizeVisionCalibration();
        }
      }

      poseWindow.push({ timestamp: now, yaw, pitch });
      while (poseWindow.length > 0 && now - poseWindow[0].timestamp > 750) {
        poseWindow.shift();
      }

      let jawOpen = 0;
      let smileScore = 0;
      let mouthCloseScore = 0;
      let speechArticulationScore = 0;
      if (result.faceBlendshapes && result.faceBlendshapes.length > 0) {
        const categories = result.faceBlendshapes[0].categories;
        const getScore = (name: string) =>
          categories.find((c) => c.categoryName === name)?.score ?? 0;
        jawOpen = getScore('jawOpen');
        smileScore = (getScore('mouthSmileLeft') + getScore('mouthSmileRight')) * 0.5;
        mouthCloseScore = getScore('mouthClose');
        speechArticulationScore = Math.max(
          getScore('mouthFunnel'),
          getScore('mouthPucker'),
          getScore('mouthShrugUpper'),
          getScore('mouthLowerDownLeft'),
          getScore('mouthLowerDownRight')
        );
      }

      const yawOffset = currentConfig.calibratedYawOffsetDeg || 0.0;
      const pitchOffset = currentConfig.calibratedPitchOffsetDeg || 0.0;
      const relYaw = yaw - yawOffset;
      const relPitch = pitch - pitchOffset;

      const sideCamera = Boolean(currentConfig.sideCameraMode);
      const baseEnterYaw = currentConfig.armYawEnterDeg || 20.0;
      const baseExitYaw = currentConfig.armYawExitDeg || 28.0;
      const enterYaw = sideCamera ? Math.max(baseEnterYaw, SIDE_CAMERA_YAW_ENTER_DEG) : baseEnterYaw;
      const exitYaw = sideCamera ? Math.max(baseExitYaw, SIDE_CAMERA_YAW_EXIT_DEG) : baseExitYaw;
      const enterPitch = currentConfig.armPitchEnterDeg || 18.0;
      const exitPitch = currentConfig.armPitchExitDeg || 25.0;

      if (!facingScreenLatched) {
        if (Math.abs(relYaw) <= enterYaw && Math.abs(relPitch) <= enterPitch) {
          facingScreenLatched = true;
        }
      } else if (Math.abs(relYaw) > exitYaw || Math.abs(relPitch) > exitPitch) {
        facingScreenLatched = false;
      }

      const facingScreen = facingScreenLatched;
      if (facingScreen) {
        lastFaceOnScreenTimestamp = now;
      }

      const restingMar = Math.max(
        0.006,
        Math.min(0.045, currentConfig.calibratedRestingMar || 0.016)
      );
      const isYawning = mar > 0.45 || jawOpen > 0.35;
      const isSmiling =
        !isYawning &&
        smileScore >= 0.45 &&
        speechArticulationScore < 0.14 &&
        mar < restingMar + 0.068;
      const isChewing =
        !isYawning &&
        !isSmiling &&
        jawOpen >= 0.055 &&
        (mouthCloseScore >= 0.20 || mar < restingMar + 0.011) &&
        speechArticulationScore < 0.15;

      const minSpeechMar = Math.max(0.016, restingMar + 0.002);
      const closedMarCeiling = Math.max(0.030, restingMar + 0.016);

      const activeLipMotion =
        !isYawning &&
        !isSmiling &&
        !isChewing &&
        mar >= minSpeechMar &&
        marVariance >= 0.000012;

      if (activeLipMotion) {
        lastLipsMovingTimestamp = now;
      }
      const lipsMoving =
        facingScreen &&
        !isYawning &&
        !isSmiling &&
        !isChewing &&
        now - lastLipsMovingTimestamp <= 300;
      const mouthClosed =
        isChewing ||
        (isSmiling && mar < restingMar + 0.020) ||
        (!isYawning && mar < closedMarCeiling && marVariance < 0.00004);
      const gesture = detectHeadGesture(now);

      // Adaptive FPS scheduling: 15 FPS when active or on-screen, 8 FPS when face away > 3s in IDLE
      const pipelineActive = currentPipelineState !== 'IDLE' || isCalibrating;
      const desiredTickMs =
        !currentConfig.adaptiveFpsEnabled ||
        pipelineActive ||
        facingScreen ||
        now - lastFaceOnScreenTimestamp < 3000
          ? VISION_TICK_ACTIVE_MS
          : VISION_TICK_IDLE_AWAY_MS;
      setVisionTickerInterval(desiredTickMs);

      if (renderUi) {
        drawLipLandmarks(landmarks, facingScreen, lipsMoving);

        valPose.textContent = `${relYaw.toFixed(1)}° / ${relPitch.toFixed(1)}°`;
        valMar.textContent = `${mar.toFixed(3)} / ${(marVariance * 1000).toFixed(2)}`;
        valGesture.textContent = gesture === 'none' ? 'None' : gesture.toUpperCase();

        const nominalFps = Math.round(1000 / currentVisionTickMs);
        fpsPill.textContent =
          currentVisionTickMs === VISION_TICK_ACTIVE_MS
            ? `${nominalFps} FPS (${inferMs.toFixed(1)} ms)`
            : `${nominalFps} FPS ECO (${inferMs.toFixed(1)} ms)`;
        fpsPill.className =
          currentVisionTickMs === VISION_TICK_ACTIVE_MS ? 'gauge-tag tag-ok' : 'gauge-tag';

        gazePill.textContent = facingScreen ? 'GAZE: ON SCREEN' : 'GAZE: AWAY';
        gazePill.className = facingScreen ? 'gauge-tag tag-ok' : 'gauge-tag tag-warn';

        lipPill.textContent = isYawning
          ? 'LIPS: YAWN CEILING'
          : isSmiling
            ? 'LIPS: SMILE FILTER'
            : isChewing
              ? 'LIPS: CHEW FILTER'
              : lipsMoving
                ? 'LIPS: MOVING'
                : !mouthClosed
                  ? 'LIPS: OPEN (THINKING)'
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
      if (isCalibrating && now >= calibratingUntilTimestamp) {
        finalizeVisionCalibration();
      }

      const pipelineActive = currentPipelineState !== 'IDLE' || isCalibrating;
      const desiredTickMs =
        !currentConfig.adaptiveFpsEnabled ||
        pipelineActive ||
        now - lastFaceOnScreenTimestamp < 3000
          ? VISION_TICK_ACTIVE_MS
          : VISION_TICK_IDLE_NO_FACE_MS;
      setVisionTickerInterval(desiredTickMs);

      if (renderUi) {
        overlayCtx.clearRect(0, 0, webcamOverlay.width, webcamOverlay.height);
        const nominalFps = Math.round(1000 / currentVisionTickMs);
        fpsPill.textContent =
          currentVisionTickMs === VISION_TICK_ACTIVE_MS
            ? `${nominalFps} FPS`
            : `${nominalFps} FPS ECO`;
        fpsPill.className = 'gauge-tag';
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

  ctx.strokeStyle = themeRgba('--paper', 0.08);
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
  ctx.strokeStyle = themeRgba('--on-dark', 0.75);
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
  ctx.strokeStyle = 'rgba(0, 174, 187, 0.9)';
  ctx.lineWidth = 1.5 * dpr;
  ctx.stroke();

  ctx.beginPath();
  for (let i = 0; i < chartHistory.length; i++) {
    const x = i * stepX;
    const prob = Math.max(0, Math.min(1.0, chartHistory[i].vadProb));
    const y = h - prob * (h - 16) - 8;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.strokeStyle = themeRgba('--paper');
  ctx.lineWidth = 2 * dpr;
  ctx.lineJoin = 'round';
  ctx.stroke();

  // Soft area under the VAD curve.
  ctx.lineTo((chartHistory.length - 1) * stepX, h);
  ctx.lineTo(0, h);
  ctx.closePath();
  const fill = ctx.createLinearGradient(0, 0, 0, h);
  fill.addColorStop(0, themeRgba('--paper', 0.22));
  fill.addColorStop(1, themeRgba('--paper', 0));
  ctx.fillStyle = fill;
  ctx.fill();
}

// App names, window titles and error text come from other processes: never put them in HTML raw.
function escapeHtml(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function formatTriggerSource(source: string): string {
  switch (source) {
    case 'hands_free_multimodal':
      return 'Hands-free';
    case 'hotkey':
      return 'Hotkey';
    case 'manual':
    case 'manual_stop':
      return 'Manual';
    case 'simulated_demo':
      return 'Simulated';
    default:
      return String(source || '').replace(/_/g, ' ');
  }
}

function appendUtteranceCard(
  ev: UtteranceCompleteEvent,
  options?: { fromHistory?: boolean }
) {
  if (!options?.fromHistory) {
    hidePreviewBanner();
  }
  emptyFeed.classList.add('hidden');
  totalTriggersCount += 1;
  if (ev.flaggedFalseTrigger) {
    falseTriggersCount += 1;
  }
  updateStatsDisplay();

  const { transcription, cleanup, targetPolicy, injection, injectionError, triggerSource, timestampMs } = ev;
  if (injectionError && !options?.fromHistory) {
    showError(injectionError);
  }
  valLatency.textContent = `${transcription.inferenceMs} ms`;
  rtfTag.textContent = `RTF ${transcription.rtf.toFixed(2)}x`;

  const card = document.createElement('div');
  card.className = 'utterance-card';
  card.dataset.searchText = `${cleanup.cleanedText} ${cleanup.rawText} ${targetPolicy.appName} ${triggerSource}`.toLowerCase();

  const timeStr = new Date(timestampMs || Date.now()).toLocaleTimeString([], {
    hour: 'numeric',
    minute: '2-digit',
  });
  const sourceLabel = formatTriggerSource(triggerSource);
  const displayTranscript =
    cleanup.cleanedText.length > 0 ? cleanup.cleanedText : '(No speech detected in audio segment)';

  const injectBadgeHtml = injection?.success
    ? `<span class="u-badge u-badge-ok" title="${escapeHtml(injection.message)}">${injection.pressedEnter ? 'Sent to' : 'Pasted into'} ${escapeHtml(injection.targetApp || targetPolicy.appName)}</span>`
    : injectionError
      ? `<span class="u-badge" title="${escapeHtml(injectionError)}">Kept on clipboard</span>`
      : `<span class="u-badge">Transcribed</span>`;

  const cleanupBadgeHtml = cleanup.modified
    ? `<span class="u-badge" title="${escapeHtml(cleanup.rawText)}">Cleaned</span>`
    : '';

  card.innerHTML = `
    <div class="utterance-text"></div>
    <div class="utterance-meta">
      <span>${escapeHtml(timeStr)}</span>
      <span>${escapeHtml(sourceLabel)}</span>
      <span title="Whisper inference time">${transcription.inferenceMs} ms</span>
      <div class="utterance-badges">
        ${injectBadgeHtml}
        ${cleanupBadgeHtml}
      </div>
    </div>
    <div class="utterance-actions">
      <button class="btn-xs btn-flag-false" title="Mark this utterance as an unintended false trigger">Flag False Trigger</button>
      <button class="btn-xs btn-copy">Copy</button>
      <button class="btn-xs btn-accent btn-delayed-inject" title="Click into your terminal within 2s to type this">Inject in 2s</button>
    </div>
  `;

  const textEl = card.querySelector('.utterance-text') as HTMLElement;
  textEl.textContent = displayTranscript;

  let flagged = Boolean(ev.flaggedFalseTrigger);
  const flagBtn = card.querySelector('.btn-flag-false') as HTMLButtonElement;
  if (flagged) {
    card.classList.add('flagged-false');
    flagBtn.textContent = 'Flagged False Trigger';
  }
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
    if (typeof ev.utteranceId === 'number') {
      void invoke('flag_false_trigger', { utteranceId: ev.utteranceId, flagged }).catch(() => undefined);
    }
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
  filterUtteranceFeed();
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
    customVocab: customVocabInput.value.trim(),
    // Enter is decided by the send mode now; app rules and the destructive guard still apply.
    autoPressEnter: true,
    sendMode: currentConfig.sendMode,
    cancelOnSpeech: toggleCancelOnSpeech.checked,
    sideCameraMode: toggleSideCamera.checked,
    uiPalette: currentConfig.uiPalette,
    moodColors: toggleMoodColors.checked,
    hudEnabled: toggleHud.checked,
    adaptiveFpsEnabled: toggleAdaptiveFps.checked,
    historyPersistenceEnabled: toggleHistoryPersistence.checked,
    llmCleanupEnabled: toggleLlmCleanup.checked,
    llmEndpointUrl: llmEndpointInput.value.trim() || 'http://127.0.0.1:11434/v1/chat/completions',
    llmModel: llmModelInput.value.trim() || 'qwen2.5-coder:1.5b',
    sessionMetricsEnabled: toggleSessionMetrics.checked,
    saveTuningAudioEnabled: toggleSaveAudio.checked,
    preferredDevice: deviceSelect.value ? deviceSelect.value : null,
  };

  vadThreshDisplay.textContent = currentConfig.vadThreshold.toFixed(2);
  previewCountdownDisplay.textContent = `${currentConfig.previewCountdownMs} ms`;
  llmCleanupFields?.classList.toggle('hidden', !currentConfig.llmCleanupEnabled);
  updateCalibrationDisplay();
  updateHistoryBadgeDisplay();
  refreshRangeFills();

  try {
    await invoke('update_dictation_config', { config: currentConfig });
  } catch (err) {
    showError(`Failed to update config: ${String(err)}`);
  }
}

function renderDevicesDropdown(devices: string[]) {
  deviceSelect.innerHTML = '';
  const defaultOpt = document.createElement('option');
  defaultOpt.value = '';
  defaultOpt.textContent = 'System Default Microphone';
  deviceSelect.appendChild(defaultOpt);

  for (const dev of devices) {
    const opt = document.createElement('option');
    opt.value = dev;
    opt.textContent = dev;
    if (currentConfig.preferredDevice === dev) {
      opt.selected = true;
    }
    deviceSelect.appendChild(opt);
  }
}

async function initApp() {
  try {
    const snapshot = await invoke<SystemStatusSnapshot>('get_system_status');
    currentConfig = snapshot.config;
    modelsCatalog = snapshot.models;
    selectedModelId = snapshot.selectedModelId;

    updateAccessibilityBadge(snapshot.accessibilityTrusted);
    startAccessibilityPoll();

    renderDevicesDropdown(snapshot.devices);

    renderModelsDropdown();

    vadThreshSlider.value = String(currentConfig.vadThreshold);
    vadThreshDisplay.textContent = currentConfig.vadThreshold.toFixed(2);
    previewCountdownSlider.value = String(currentConfig.previewCountdownMs);
    previewCountdownDisplay.textContent = `${currentConfig.previewCountdownMs} ms`;
    allowlistInput.value = currentConfig.autoSendAllowlist.join(', ');
    customVocabInput.value = currentConfig.customVocab || '';

    toggleCleanup.checked = currentConfig.cleanupEnabled;
    togglePreview.checked = currentConfig.previewEnabled;
    togglePerApp.checked = currentConfig.perAppRulesEnabled;
    toggleHandsFree.checked = currentConfig.handsFreeEnabled;
    // Older settings files: "Auto-press Enter" off meant type only.
    if (!currentConfig.sendMode) currentConfig.sendMode = 'pause';
    if (currentConfig.autoPressEnter === false && currentConfig.sendMode === 'pause') {
      currentConfig.sendMode = 'paste_only';
      currentConfig.autoPressEnter = true;
      void invoke('update_dictation_config', { config: currentConfig }).catch(() => undefined);
    }
    toggleCancelOnSpeech.checked = currentConfig.cancelOnSpeech !== false;
    toggleSideCamera.checked = Boolean(currentConfig.sideCameraMode);
    toggleMoodColors.checked = currentConfig.moodColors !== false;
    renderPalette();
    renderSendMode();
    toggleHud.checked = currentConfig.hudEnabled;
    toggleAdaptiveFps.checked = currentConfig.adaptiveFpsEnabled !== false;
    toggleHistoryPersistence.checked = currentConfig.historyPersistenceEnabled !== false;
    toggleLlmCleanup.checked = Boolean(currentConfig.llmCleanupEnabled);
    llmEndpointInput.value =
      currentConfig.llmEndpointUrl || 'http://127.0.0.1:11434/v1/chat/completions';
    llmModelInput.value = currentConfig.llmModel || 'qwen2.5-coder:1.5b';
    llmCleanupFields?.classList.toggle('hidden', !currentConfig.llmCleanupEnabled);
    toggleSessionMetrics.checked = Boolean(currentConfig.sessionMetricsEnabled);
    toggleSaveAudio.checked = Boolean(currentConfig.saveTuningAudioEnabled);
    updateCalibrationDisplay();
    updateHistoryBadgeDisplay();

    if (Array.isArray(snapshot.recentHistory) && snapshot.recentHistory.length > 0) {
      for (const item of snapshot.recentHistory) {
        appendUtteranceCard(item, { fromHistory: true });
      }
    }

    updatePipelineUiState(snapshot.state, 0, 0);
    refreshRangeFills();
  } catch (err) {
    showError(`Backend connection warning: ${String(err)}`);
  }

  await listen<AudioTelemetryEvent>('audio-telemetry', (event) => {
    const t = event.payload;
    updatePipelineUiState(
      t.state,
      t.listeningDurationMs,
      t.previewRemainingMs,
      Boolean(t.postRollActive)
    );

    valVadProb.textContent = t.vadProb.toFixed(2);
    if (t.isSpeech) {
      vadStateTag.textContent = 'SPEECH';
      vadStateTag.className = 'gauge-tag tag-ok';
    } else {
      vadStateTag.textContent = 'SILENCE';
      vadStateTag.className = 'gauge-tag';
    }

    valRms.textContent = t.rms.toFixed(3);
    setAudioLevel(t.rms, t.vadProb, t.isSpeech);
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
    toggleAdaptiveFps.checked = currentConfig.adaptiveFpsEnabled !== false;
    toggleHistoryPersistence.checked = currentConfig.historyPersistenceEnabled !== false;
    toggleLlmCleanup.checked = Boolean(currentConfig.llmCleanupEnabled);
    llmEndpointInput.value =
      currentConfig.llmEndpointUrl || 'http://127.0.0.1:11434/v1/chat/completions';
    llmModelInput.value = currentConfig.llmModel || 'qwen2.5-coder:1.5b';
    llmCleanupFields?.classList.toggle('hidden', !currentConfig.llmCleanupEnabled);
    toggleSessionMetrics.checked = Boolean(currentConfig.sessionMetricsEnabled);
    toggleSaveAudio.checked = Boolean(currentConfig.saveTuningAudioEnabled);
    updateCalibrationDisplay();
    updateHistoryBadgeDisplay();
  });

  await listen<string[]>('audio-devices-changed', (event) => {
    if (Array.isArray(event.payload)) {
      renderDevicesDropdown(event.payload);
    }
  });

  await listen<string>('utterance-discarded', (event) => {
    deckSubtitle.textContent = event.payload;
  });
}

// Event Listeners
toggleCameraBtn.addEventListener('click', toggleCameraGate);

calibrateVisionBtn.addEventListener('click', () => {
  if (!cameraRunning) {
    showError(
      'Enable Camera Gate first, then look naturally at your target coding screen with a relaxed closed mouth and click Calibrate Gaze.'
    );
    return;
  }
  calibrationSamples.length = 0;
  calibratingUntilTimestamp = performance.now() + 1500;
  calibrateVisionBtn.disabled = true;
  calibrateVisionBtn.textContent = 'Calibrating (Look at Screen)...';
  setVisionTickerInterval(VISION_TICK_ACTIVE_MS);
});

resetCalibrationBtn.addEventListener('click', () => {
  calibratingUntilTimestamp = 0;
  calibrationSamples.length = 0;
  calibrateVisionBtn.disabled = false;
  calibrateVisionBtn.textContent = 'Calibrate Gaze (1.5s)';
  currentConfig.calibratedYawOffsetDeg = 0.0;
  currentConfig.calibratedPitchOffsetDeg = 0.0;
  currentConfig.calibratedRestingMar = 0.016;
  updateCalibrationDisplay();
  void syncConfigToBackend();
});

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

clearIssuesBtn?.addEventListener('click', () => {
  sessionIssues.length = 0;
  renderIssuesList();
});

issuesList?.addEventListener('click', (event) => {
  const target = event.target as HTMLElement;
  const copyBtn = target.closest('[data-copy-issue]') as HTMLElement | null;
  if (copyBtn) {
    const id = Number(copyBtn.dataset.issueId);
    const issue = sessionIssues.find((item) => item.id === id);
    if (issue) {
      void navigator.clipboard.writeText(issue.message);
    }
    return;
  }
  if (target.closest('[data-open-accessibility]')) {
    void invoke('request_accessibility_settings').catch((err) => showError(String(err)));
  }
});

renderIssuesList();

deviceSelect.addEventListener('change', syncConfigToBackend);
vadThreshSlider.addEventListener('input', syncConfigToBackend);
previewCountdownSlider.addEventListener('input', syncConfigToBackend);
allowlistInput.addEventListener('change', syncConfigToBackend);
customVocabInput.addEventListener('change', syncConfigToBackend);
toggleCleanup.addEventListener('change', syncConfigToBackend);
togglePreview.addEventListener('change', syncConfigToBackend);
togglePerApp.addEventListener('change', syncConfigToBackend);
toggleHandsFree.addEventListener('change', syncConfigToBackend);
toggleCancelOnSpeech.addEventListener('change', syncConfigToBackend);
toggleSideCamera.addEventListener('change', syncConfigToBackend);
toggleMoodColors.addEventListener('change', () => {
  currentConfig = { ...currentConfig, moodColors: toggleMoodColors.checked };
  renderPalette();
  void syncConfigToBackend();
});
obAxBadge?.addEventListener('click', () => {
  if (accessibilityPermissionTrusted) return;
  void invoke('request_accessibility_settings').catch((err) => showError(String(err)));
});
toggleHud.addEventListener('change', syncConfigToBackend);
toggleAdaptiveFps.addEventListener('change', syncConfigToBackend);
toggleHistoryPersistence.addEventListener('change', syncConfigToBackend);
toggleLlmCleanup.addEventListener('change', syncConfigToBackend);
llmEndpointInput.addEventListener('change', syncConfigToBackend);
llmModelInput.addEventListener('change', syncConfigToBackend);
toggleSessionMetrics.addEventListener('change', syncConfigToBackend);
toggleSaveAudio.addEventListener('change', syncConfigToBackend);

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
    line.textContent = `saywhen ~ % ${cmd}`;
    terminalHistory.appendChild(line);
    terminalHistory.scrollTop = terminalHistory.scrollHeight;
    terminalInput.value = '';
  }
});

clearTerminalBtn.addEventListener('click', () => {
  terminalHistory.innerHTML =
    '<div class="term-line term-muted"># Focus the input below and speak, or click Simulate preview</div>';
});

historySearchInput.addEventListener('input', filterUtteranceFeed);

clearFeedBtn.addEventListener('click', () => {
  transcriptList.innerHTML = '';
  transcriptList.appendChild(emptyFeed);
  emptyFeed.classList.remove('hidden');
  totalTriggersCount = 0;
  falseTriggersCount = 0;
  updateStatsDisplay();
  void invoke('clear_utterance_history').catch(() => undefined);
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
    toggleCameraLabel.textContent = 'Turn off camera';
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

    fpsPill.textContent = '15 FPS (4.1 ms)';
    fpsPill.className = 'gauge-tag tag-ok';
    gazePill.textContent = 'GAZE: ON SCREEN';
    gazePill.className = 'gauge-tag tag-ok';
    lipPill.textContent = 'LIPS: MOVING';
    lipPill.className = 'gauge-tag tag-ok';
    valPose.textContent = '-1.8° / +2.4°';
    valMar.textContent = '0.148 / 1.62';
    valGesture.textContent = 'NONE';
    updateCalibrationDisplay();

    valVadProb.textContent = '0.94';
    vadStateTag.textContent = 'SPEECH';
    vadStateTag.className = 'gauge-tag tag-ok';
    valRms.textContent = '0.118';
    updatePipelineUiState('LISTENING', 2400, 0);
  } else if (demoMode === 'stage4') {
    cameraRunning = true;
    toggleCameraBtn.classList.add('camera-active');
    toggleCameraLabel.textContent = 'Turn off camera';
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

    fpsPill.textContent = '15 FPS (3.9 ms)';
    fpsPill.className = 'gauge-tag tag-ok';
    gazePill.textContent = 'GAZE: ON SCREEN';
    gazePill.className = 'gauge-tag tag-ok';
    lipPill.textContent = 'LIPS: CLOSED';
    lipPill.className = 'gauge-tag';
    valPose.textContent = '-0.6° / +1.2°';
    valMar.textContent = '0.021 / 0.01';
    valGesture.textContent = 'NOD READY';
    updateCalibrationDisplay();

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
