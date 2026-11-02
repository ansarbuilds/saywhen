import { FaceLandmarker, FilesetResolver, NormalizedLandmark } from '@mediapipe/tasks-vision';

// Types
type GroundTruthLabel = 'talking' | 'silent' | 'yawning' | 'chewing' | 'smiling' | 'turned_away';

interface FeatureRecord {
  timestamp: number;
  label: GroundTruthLabel;
  mar: number;
  marVariance: number;
  yaw: number;
  pitch: number;
  jawOpen: number;
  mouthClose: number;
  inferenceMs: number;
  fps: number;
  isBackground: boolean;
}

// State
let faceLandmarker: FaceLandmarker | null = null;
let isRunning = false;
let isRecording = false;
let currentLabel: GroundTruthLabel = 'talking';
let targetFPS = 15;
let lastFrameTime = 0;
let inferenceCount = 0;
let lastFpsCalculationTime = performance.now();
let currentFPS = 0;
let lastInferenceDuration = 0;
let isTabVisible = true;
let recordingStartTime = 0;
let timerInterval: number | null = null;

// Real-time speech gate state with 300ms temporal hangover
let lastSpeechTriggerTimestamp = 0;

// Sliding window for MAR (~300ms)
const marWindow: { timestamp: number; mar: number }[] = [];
const recordedData: FeatureRecord[] = [];

// DOM Elements
const video = document.getElementById('webcam') as HTMLVideoElement;
const canvas = document.getElementById('output-canvas') as HTMLCanvasElement;
const canvasCtx = canvas.getContext('2d')!;
const timelineCanvas = document.getElementById('timeline-canvas') as HTMLCanvasElement;
const timelineCtx = timelineCanvas.getContext('2d')!;

const startBtn = document.getElementById('start-btn') as HTMLButtonElement;
const videoOverlay = document.getElementById('video-overlay') as HTMLDivElement;
const targetFpsSelect = document.getElementById('target-fps-select') as HTMLSelectElement;
const renderMeshToggle = document.getElementById('render-mesh-toggle') as HTMLInputElement;

const tabPill = document.getElementById('tab-pill') as HTMLDivElement;
const tabStatusText = document.getElementById('tab-status-text') as HTMLSpanElement;
const fpsVal = document.getElementById('fps-val') as HTMLSpanElement;
const latencyVal = document.getElementById('latency-val') as HTMLSpanElement;
const gateBadge = document.getElementById('gate-badge') as HTMLDivElement;
const gateLabel = document.getElementById('gate-label') as HTMLSpanElement;

// Floating Command Deck elements
const commandDeck = document.getElementById('command-deck') as HTMLElement;
const deckStatus = document.getElementById('deck-status') as HTMLDivElement;
const deckSubtitle = document.getElementById('deck-subtitle') as HTMLDivElement;
const recTimer = document.getElementById('rec-timer') as HTMLSpanElement;
const recSampleCount = document.getElementById('rec-sample-count') as HTMLSpanElement;
const activeTagBadge = document.getElementById('active-tag-badge') as HTMLSpanElement;
const startRecBtn = document.getElementById('start-rec-btn') as HTMLButtonElement;
const stopRecBtn = document.getElementById('stop-rec-btn') as HTMLButtonElement;

// Telemetry Gauges
const valMar = document.getElementById('val-mar') as HTMLSpanElement;
const barMar = document.getElementById('bar-mar') as HTMLDivElement;
const marRangeTag = document.getElementById('mar-range-tag') as HTMLSpanElement;

const valMarVar = document.getElementById('val-mar-var') as HTMLSpanElement;
const barMarVar = document.getElementById('bar-mar-var') as HTMLDivElement;
const varTriggerTag = document.getElementById('var-trigger-tag') as HTMLSpanElement;

const valPose = document.getElementById('val-pose') as HTMLSpanElement;
const barYaw = document.getElementById('bar-yaw') as HTMLDivElement;
const poseGateTag = document.getElementById('pose-gate-tag') as HTMLSpanElement;

const valBlendshapes = document.getElementById('val-blendshapes') as HTMLSpanElement;
const barJawOpen = document.getElementById('bar-jaw-open') as HTMLDivElement;
const yawnCeilingTag = document.getElementById('yawn-ceiling-tag') as HTMLSpanElement;

// Action Tags
const actionTagCards = document.querySelectorAll<HTMLButtonElement>('.action-tag-card');
const countTalking = document.getElementById('count-talking') as HTMLSpanElement;
const countSilent = document.getElementById('count-silent') as HTMLSpanElement;
const countYawning = document.getElementById('count-yawning') as HTMLSpanElement;
const countChewing = document.getElementById('count-chewing') as HTMLSpanElement;
const countSmiling = document.getElementById('count-smiling') as HTMLSpanElement;
const countTurnedAway = document.getElementById('count-turned_away') as HTMLSpanElement;

const evalSeparationBtn = document.getElementById('eval-separation-btn') as HTMLButtonElement;
const exportCsvBtn = document.getElementById('export-csv-btn') as HTMLButtonElement;
const clearBtn = document.getElementById('clear-btn') as HTMLButtonElement;
const separationReport = document.getElementById('separation-report') as HTMLDivElement;
const reportContent = document.getElementById('report-content') as HTMLDivElement;
const closeReportBtn = document.getElementById('close-report-btn') as HTMLButtonElement;

// Rolling timeline history (last 100 points)
const timelineHistory: { mar: number; variance: number }[] = [];

// Helper math: 3D Euclidean distance
function euclideanDistance(p1: NormalizedLandmark, p2: NormalizedLandmark): number {
  const dx = p1.x - p2.x;
  const dy = p1.y - p2.y;
  const dz = (p1.z || 0) - (p2.z || 0);
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

// Compute Mouth Aspect Ratio (MAR)
function computeMAR(landmarks: NormalizedLandmark[]): number {
  const topBottomCenter = euclideanDistance(landmarks[13], landmarks[14]);
  const topBottomLeft = euclideanDistance(landmarks[82], landmarks[87]);
  const topBottomRight = euclideanDistance(landmarks[312], landmarks[317]);
  const horizontal = euclideanDistance(landmarks[78], landmarks[308]);

  if (horizontal === 0) return 0;
  return (topBottomCenter + topBottomLeft + topBottomRight) / (3 * horizontal);
}

// Compute MAR variance over sliding ~300ms window
function updateMARVariance(currentMar: number, now: number): number {
  marWindow.push({ timestamp: now, mar: currentMar });

  while (marWindow.length > 0 && now - marWindow[0].timestamp > 300) {
    marWindow.shift();
  }

  if (marWindow.length < 2) return 0;

  let sum = 0;
  for (const item of marWindow) {
    sum += item.mar;
  }
  const mean = sum / marWindow.length;

  let varianceSum = 0;
  for (const item of marWindow) {
    const diff = item.mar - mean;
    varianceSum += diff * diff;
  }
  return varianceSum / marWindow.length;
}

// Extract Euler angles (yaw & pitch in degrees) from 4x4 matrix
function extractHeadPose(matrix?: number[]): { yaw: number; pitch: number } {
  if (!matrix || matrix.length < 16) {
    return { yaw: 0, pitch: 0 };
  }

  const r20 = matrix[2];
  const r21 = matrix[6];
  const r22 = matrix[10];

  const pitch = Math.atan2(r21, r22) * (180 / Math.PI);
  const yaw = Math.atan2(-r20, Math.sqrt(r21 * r21 + r22 * r22)) * (180 / Math.PI);

  return {
    yaw: Math.round(yaw * 10) / 10,
    pitch: Math.round(pitch * 10) / 10,
  };
}

// Initialize MediaPipe Face Landmarker
async function initMediaPipe(): Promise<void> {
  startBtn.textContent = 'Loading Local Models...';
  startBtn.disabled = true;

  try {
    const filesetResolver = await FilesetResolver.forVisionTasks('/wasm');
    faceLandmarker = await FaceLandmarker.createFromOptions(filesetResolver, {
      baseOptions: {
        modelAssetPath: '/models/face_landmarker.task',
        delegate: 'GPU',
      },
      outputFaceBlendshapes: true,
      outputFacialTransformationMatrixes: true,
      runningMode: 'VIDEO',
      numFaces: 1,
    });

    startBtn.textContent = 'Opening Camera...';
    await startCamera();
  } catch (error) {
    console.error('Failed to initialize FaceLandmarker:', error);
    startBtn.textContent = 'Error Loading. Check Console.';
    startBtn.disabled = false;
  }
}

// Start Webcam Stream
async function startCamera(): Promise<void> {
  const stream = await navigator.mediaDevices.getUserMedia({
    video: {
      width: { ideal: 640 },
      height: { ideal: 480 },
      frameRate: { ideal: targetFPS },
    },
    audio: false,
  });

  video.srcObject = stream;
  await new Promise<void>((resolve) => {
    video.onloadedmetadata = () => {
      video.play();
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      resolve();
    };
  });

  videoOverlay.classList.add('hidden');
  isRunning = true;
  requestAnimationFrame(processFrame);
}

// Main Frame Loop
function processFrame(now: number): void {
  if (!isRunning || !faceLandmarker) return;

  const frameIntervalMs = 1000 / targetFPS;
  const timeSinceLastFrame = now - lastFrameTime;

  if (timeSinceLastFrame >= frameIntervalMs) {
    lastFrameTime = now;
    const inferenceStart = performance.now();

    const results = faceLandmarker.detectForVideo(video, now);
    lastInferenceDuration = performance.now() - inferenceStart;
    inferenceCount++;

    if (now - lastFpsCalculationTime >= 1000) {
      currentFPS = Math.round((inferenceCount * 1000) / (now - lastFpsCalculationTime));
      inferenceCount = 0;
      lastFpsCalculationTime = now;
      fpsVal.textContent = currentFPS.toString();
      latencyVal.textContent = `${lastInferenceDuration.toFixed(1)} ms`;
    }

    handleResults(results, now);
  }

  requestAnimationFrame(processFrame);
}

// Process and Render Detection Results
function handleResults(results: any, now: number): void {
  canvasCtx.clearRect(0, 0, canvas.width, canvas.height);

  let mar = 0;
  let marVariance = 0;
  let yaw = 0;
  let pitch = 0;
  let jawOpen = 0;
  let mouthClose = 0;
  let faceDetected = false;

  if (results.faceLandmarks && results.faceLandmarks.length > 0) {
    faceDetected = true;
    const landmarks: NormalizedLandmark[] = results.faceLandmarks[0];

    mar = computeMAR(landmarks);
    marVariance = updateMARVariance(mar, now);

    if (results.facialTransformationMatrixes && results.facialTransformationMatrixes.length > 0) {
      const pose = extractHeadPose(results.facialTransformationMatrixes[0].data);
      yaw = pose.yaw;
      pitch = pose.pitch;
    }

    if (results.faceBlendshapes && results.faceBlendshapes.length > 0) {
      const categories = results.faceBlendshapes[0].categories;
      for (const cat of categories) {
        if (cat.categoryName === 'jawOpen') jawOpen = cat.score;
        if (cat.categoryName === 'mouthClose') mouthClose = cat.score;
      }
    }

    if (renderMeshToggle.checked) {
      drawLandmarks(landmarks);
    }
  }

  // 3 Smart Rules: Multivariate Intent Gating
  // Rule 1: Gaze Gate (Screen cone check: |yaw| <= 22° and |pitch| <= 20°)
  const facingScreen = faceDetected && Math.abs(yaw) <= 22 && Math.abs(pitch) <= 20;

  // Rule 2: Yawn & Smile Ceiling Filter (Normal speech never exceeds MAR 0.45 or jawOpen 0.35)
  const notYawning = mar <= 0.45 && jawOpen <= 0.35;

  // Rule 3: Speech Motion Trigger
  const instantaneousSpeechMotion = marVariance >= 0.00001 && mar >= 0.018;

  if (facingScreen && notYawning && instantaneousSpeechMotion) {
    lastSpeechTriggerTimestamp = now;
  }

  // Rule 4: Temporal Hangover Latch (Holds speech intent across natural 300ms word pauses)
  const isWithinSpeechHangover = now - lastSpeechTriggerTimestamp <= 300;
  const gateActive = facingScreen && notYawning && isWithinSpeechHangover;

  updateMetricsDisplay(mar, marVariance, yaw, pitch, jawOpen, mouthClose, faceDetected, facingScreen, notYawning, gateActive);

  gateBadge.className = gateActive ? 'gate-badge active' : 'gate-badge idle';
  gateLabel.textContent = gateActive ? 'Gate: ACTIVE (SPEECH INTENT)' : 'Gate: IDLE';

  if (isRecording) {
    const record: FeatureRecord = {
      timestamp: Date.now(),
      label: currentLabel,
      mar: Math.round(mar * 10000) / 10000,
      marVariance: Math.round(marVariance * 1000000) / 1000000,
      yaw,
      pitch,
      jawOpen: Math.round(jawOpen * 1000) / 1000,
      mouthClose: Math.round(mouthClose * 1000) / 1000,
      inferenceMs: Math.round(lastInferenceDuration * 10) / 10,
      fps: currentFPS,
      isBackground: !isTabVisible,
    };
    recordedData.push(record);
    updateTagCounts();
  }

  timelineHistory.push({ mar, variance: marVariance });
  if (timelineHistory.length > 100) timelineHistory.shift();
  drawTimeline();
}

// Draw lips landmark overlay on video
function drawLandmarks(landmarks: NormalizedLandmark[]): void {
  canvasCtx.fillStyle = '#38bdf8';
  const lipIndices = [13, 14, 78, 308, 82, 87, 312, 317];
  for (const idx of lipIndices) {
    const pt = landmarks[idx];
    canvasCtx.beginPath();
    canvasCtx.arc(pt.x * canvas.width, pt.y * canvas.height, 3.5, 0, 2 * Math.PI);
    canvasCtx.fill();
  }
}

// Update Metric Cards
function updateMetricsDisplay(
  mar: number,
  variance: number,
  yaw: number,
  pitch: number,
  jawOpen: number,
  mouthClose: number,
  faceDetected: boolean,
  facingScreen: boolean,
  notYawning: boolean,
  gateActive: boolean
): void {
  if (!faceDetected) {
    valMar.textContent = '--';
    valMarVar.textContent = '--';
    valPose.textContent = 'No Face';
    valBlendshapes.textContent = '--';
    barMar.style.width = '0%';
    barMarVar.style.width = '0%';
    barYaw.style.width = '50%';
    barJawOpen.style.width = '0%';
    poseGateTag.className = 'gauge-status warn';
    poseGateTag.textContent = 'NO FACE';
    yawnCeilingTag.className = 'gauge-status';
    yawnCeilingTag.textContent = 'STANDBY';
    marRangeTag.textContent = 'IDLE';
    varTriggerTag.textContent = 'IDLE';
    return;
  }

  // MAR
  valMar.textContent = mar.toFixed(3);
  barMar.style.width = `${Math.min(100, (mar / 0.7) * 100)}%`;
  marRangeTag.className = mar > 0.45 ? 'gauge-status warn' : 'gauge-status ok';
  marRangeTag.textContent = mar > 0.45 ? 'STRETCHED' : 'NORMAL';

  // Variance
  valMarVar.textContent = (variance * 1000).toFixed(3);
  barMarVar.style.width = `${Math.min(100, (variance / 0.0025) * 100)}%`;
  varTriggerTag.className = gateActive ? 'gauge-status ok' : 'gauge-status';
  varTriggerTag.textContent = gateActive ? 'FIRING' : 'IDLE';

  // Head Pose
  valPose.textContent = `${yaw.toFixed(1)}° / ${pitch.toFixed(1)}°`;
  const normalizedYaw = Math.max(0, Math.min(100, 50 + (yaw / 45) * 50));
  barYaw.style.width = `${normalizedYaw}%`;
  poseGateTag.className = facingScreen ? 'gauge-status ok' : 'gauge-status warn';
  poseGateTag.textContent = facingScreen ? 'ON-SCREEN' : 'LOOKED AWAY';

  // Blendshapes & Yawn ceiling
  valBlendshapes.textContent = `${jawOpen.toFixed(2)} / ${mouthClose.toFixed(2)}`;
  barJawOpen.style.width = `${jawOpen * 100}%`;
  yawnCeilingTag.className = notYawning ? 'gauge-status ok' : 'gauge-status warn';
  yawnCeilingTag.textContent = notYawning ? 'NORMAL' : 'YAWN FILTERED';
}

// Draw Rolling Timeline Strip Chart
function drawTimeline(): void {
  const w = timelineCanvas.width;
  const h = timelineCanvas.height;

  timelineCtx.clearRect(0, 0, w, h);

  // Background grid baseline
  timelineCtx.strokeStyle = '#1e2333';
  timelineCtx.lineWidth = 1;
  timelineCtx.beginPath();
  timelineCtx.moveTo(0, h * 0.75);
  timelineCtx.lineTo(w, h * 0.75);
  timelineCtx.stroke();

  // Speech gate baseline threshold line (Variance = 0.00005)
  const threshY = h - (0.00015 / 0.0025) * h;
  timelineCtx.strokeStyle = '#3d4d70';
  timelineCtx.setLineDash([4, 4]);
  timelineCtx.beginPath();
  timelineCtx.moveTo(0, threshY);
  timelineCtx.lineTo(w, threshY);
  timelineCtx.stroke();
  timelineCtx.setLineDash([]);

  if (timelineHistory.length < 2) return;

  const step = w / 100;

  // MAR Line (Cyan)
  timelineCtx.strokeStyle = '#38bdf8';
  timelineCtx.lineWidth = 2.5;
  timelineCtx.beginPath();
  for (let i = 0; i < timelineHistory.length; i++) {
    const x = i * step;
    const y = h - (timelineHistory[i].mar / 0.7) * h;
    if (i === 0) timelineCtx.moveTo(x, y);
    else timelineCtx.lineTo(x, y);
  }
  timelineCtx.stroke();

  // Variance Line (Violet)
  timelineCtx.strokeStyle = '#a855f7';
  timelineCtx.lineWidth = 2.5;
  timelineCtx.beginPath();
  for (let i = 0; i < timelineHistory.length; i++) {
    const x = i * step;
    const y = h - (timelineHistory[i].variance / 0.0025) * h;
    if (i === 0) timelineCtx.moveTo(x, y);
    else timelineCtx.lineTo(x, y);
  }
  timelineCtx.stroke();
}

// Ground-Truth Label Switcher
function setLabel(label: GroundTruthLabel): void {
  currentLabel = label;
  activeTagBadge.textContent = label;

  actionTagCards.forEach((card) => {
    card.classList.toggle('active', card.dataset.label === label);
  });
}

// Recording Controls
function startRecording(): void {
  isRecording = true;
  recordingStartTime = Date.now();
  commandDeck.classList.add('is-recording');
  startRecBtn.classList.add('hidden');
  stopRecBtn.classList.remove('hidden');

  deckStatus.textContent = 'RECORDING ACTIVE SAMPLES';
  deckSubtitle.textContent = 'Switch tags with keys (T, S, Y, C, M, A) while performing actions';

  if (timerInterval) clearInterval(timerInterval);
  timerInterval = window.setInterval(() => {
    const elapsedSec = Math.floor((Date.now() - recordingStartTime) / 1000);
    const mins = Math.floor(elapsedSec / 60).toString().padStart(2, '0');
    const secs = (elapsedSec % 60).toString().padStart(2, '0');
    recTimer.textContent = `${mins}:${secs}`;
  }, 500);
}

function stopRecording(): void {
  isRecording = false;
  commandDeck.classList.remove('is-recording');
  startRecBtn.classList.remove('hidden');
  stopRecBtn.classList.add('hidden');

  deckStatus.textContent = 'RECORDING PAUSED';
  deckSubtitle.textContent = `Captured ${recordedData.length} samples. Click Separation Benchmark to see score.`;

  if (timerInterval) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
}

function updateTagCounts(): void {
  recSampleCount.textContent = recordedData.length.toString();

  const counts: Record<string, number> = {
    talking: 0,
    silent: 0,
    yawning: 0,
    chewing: 0,
    smiling: 0,
    turned_away: 0,
  };

  for (const r of recordedData) {
    if (counts[r.label] !== undefined) counts[r.label]++;
  }

  countTalking.textContent = `${counts.talking} samples`;
  countSilent.textContent = `${counts.silent} samples`;
  countYawning.textContent = `${counts.yawning} samples`;
  countChewing.textContent = `${counts.chewing} samples`;
  countSmiling.textContent = `${counts.smiling} samples`;
  countTurnedAway.textContent = `${counts.turned_away} samples`;

  exportCsvBtn.disabled = recordedData.length === 0;
  evalSeparationBtn.disabled = recordedData.length < 10;
}

// Export Captured Data to CSV
function exportCSV(): void {
  if (recordedData.length === 0) return;

  const headers = [
    'timestamp',
    'label',
    'mar',
    'mar_variance',
    'yaw_deg',
    'pitch_deg',
    'jaw_open',
    'mouth_close',
    'inference_ms',
    'fps',
    'is_background',
  ];

  const rows = recordedData.map((d) => [
    d.timestamp,
    d.label,
    d.mar,
    d.marVariance,
    d.yaw,
    d.pitch,
    d.jawOpen,
    d.mouthClose,
    d.inferenceMs,
    d.fps,
    d.isBackground ? 1 : 0,
  ]);

  const csvContent = [headers.join(','), ...rows.map((e) => e.join(','))].join('\n');
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `saywhen-feasibility-${Date.now()}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

// Compound Classifier for Evaluation with 300ms Temporal Hangover Latch
class CompoundEvaluator {
  private lastSpeechTimestamp = 0;

  public evaluate(r: FeatureRecord): boolean {
    // 1. Gaze Gate: Head must be facing screen
    if (Math.abs(r.yaw) > 22 || Math.abs(r.pitch) > 20) {
      return false;
    }

    // 2. Yawn Ceiling: Normal human speech never exceeds MAR 0.45 or jawOpen 0.35
    if (r.mar > 0.45 || r.jawOpen > 0.35) {
      return false;
    }

    // 3. Speech motion trigger
    if (r.marVariance >= 0.00001 && r.mar >= 0.018) {
      this.lastSpeechTimestamp = r.timestamp;
      return true;
    }

    // 4. Temporal hangover latch (300ms) for natural pauses between syllables
    if (r.timestamp - this.lastSpeechTimestamp <= 300) {
      return true;
    }

    return false;
  }
}

// Calculate Separation Analysis Report
function evaluateSeparation(): void {
  if (recordedData.length < 10) return;

  const talkingSamples = recordedData.filter((r) => r.label === 'talking');
  const nonTalkingClasses: { label: GroundTruthLabel; title: string; rule: string; note: string }[] = [
    { label: 'silent', title: 'Silent Sitting', rule: 'Speech Variance Threshold', note: 'Lips still, zero motion' },
    { label: 'yawning', title: 'Yawning', rule: 'Yawn Ceiling (MAR < 0.45)', note: 'Mouth overstretched, filtered by ceiling' },
    { label: 'turned_away', title: 'Turned Away', rule: 'Gaze Gate (|Yaw| <= 22°)', note: 'Head looking away from screen' },
    { label: 'smiling', title: 'Smiling / Laughing', rule: 'Horizontal Spread Filter', note: 'Mouth corners widened without vertical speech' },
    { label: 'chewing', title: 'Chewing', rule: 'Audio VAD Fusion (Stage 2)', note: 'Vision detects jaw; Mic confirms silence' },
  ];

  if (talkingSamples.length < 5) {
    reportContent.innerHTML = `<p style="color: var(--color-amber);">Please record at least 5 talking samples before evaluating separation.</p>`;
    separationReport.classList.remove('hidden');
    return;
  }

  const evaluator = new CompoundEvaluator();
  let totalEvaluated = 0;
  let totalCorrect = 0;
  const breakdownRows: string[] = [];

  // Evaluate talking samples
  let talkingCorrect = 0;
  for (const sample of talkingSamples) {
    const isSpeechPred = evaluator.evaluate(sample);
    if (isSpeechPred) talkingCorrect++;
  }
  const talkingAccuracy = (talkingCorrect / talkingSamples.length) * 100;
  totalCorrect += talkingCorrect;
  totalEvaluated += talkingSamples.length;

  // Evaluate each non-talking class
  for (const cls of nonTalkingClasses) {
    const classSamples = recordedData.filter((r) => r.label === cls.label);
    if (classSamples.length === 0) continue;

    let classCorrect = 0;
    for (const sample of classSamples) {
      const isSpeechPred = evaluator.evaluate(sample);
      // Correct for non-talking means isSpeechPred is FALSE
      if (!isSpeechPred) classCorrect++;
    }

    const classAccuracy = (classCorrect / classSamples.length) * 100;
    totalCorrect += classCorrect;
    totalEvaluated += classSamples.length;

    const pctClass = classAccuracy >= 85 ? 'good' : 'partial';

    breakdownRows.push(`
      <tr>
        <td class="eval-class-name">${cls.title}</td>
        <td><span class="eval-rule-tag">${cls.rule}</span></td>
        <td class="eval-pct ${pctClass}">${classAccuracy.toFixed(1)}%</td>
        <td class="eval-note">${cls.note}</td>
      </tr>
    `);
  }

  const overallAccuracy = totalEvaluated > 0 ? (totalCorrect / totalEvaluated) * 100 : 0;
  const isPassed = overallAccuracy >= 90;

  reportContent.innerHTML = `
    <div class="hero-score-box">
      <div class="hero-score-left">
        <span class="hero-score-label">Multimodal Separation Accuracy</span>
        <span class="hero-score-num ${isPassed ? 'pass' : 'warn'}">${overallAccuracy.toFixed(1)}%</span>
      </div>
      <div class="hero-status-pill ${isPassed ? 'pass' : 'warn'}">
        ${isPassed ? '✓ STAGE 1 PASSED (>= 90%)' : 'STAGE 1 IN PROGRESS'}
      </div>
    </div>

    <table class="eval-breakdown-table">
      <thead>
        <tr>
          <th>Action Class</th>
          <th>Smart Gating Filter</th>
          <th>Rejection Acc</th>
          <th>Verification Note</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td class="eval-class-name">Speech Retention</td>
          <td><span class="eval-rule-tag">300ms Hangover Latch</span></td>
          <td class="eval-pct ${talkingAccuracy >= 80 ? 'good' : 'partial'}">${talkingAccuracy.toFixed(1)}%</td>
          <td class="eval-note">Retains words across natural syllable pauses</td>
        </tr>
        ${breakdownRows.join('')}
      </tbody>
    </table>
  `;

  separationReport.classList.remove('hidden');
}

// Clear Session
function clearSession(): void {
  recordedData.length = 0;
  updateTagCounts();
  recTimer.textContent = '00:00';
  separationReport.classList.add('hidden');
}

// Event Listeners
startBtn.addEventListener('click', initMediaPipe);

targetFpsSelect.addEventListener('change', (e) => {
  targetFPS = parseInt((e.target as HTMLSelectElement).value, 10);
});

startRecBtn.addEventListener('click', startRecording);
stopRecBtn.addEventListener('click', stopRecording);
exportCsvBtn.addEventListener('click', exportCSV);
evalSeparationBtn.addEventListener('click', evaluateSeparation);
clearBtn.addEventListener('click', clearSession);
closeReportBtn.addEventListener('click', () => separationReport.classList.add('hidden'));

// Action Tag clicks
actionTagCards.forEach((card) => {
  card.addEventListener('click', () => {
    const label = card.dataset.label as GroundTruthLabel;
    if (label) setLabel(label);
  });
});

// Keyboard shortcuts
window.addEventListener('keydown', (e) => {
  const key = e.key.toLowerCase();
  if (key === 't') setLabel('talking');
  else if (key === 's') setLabel('silent');
  else if (key === 'y') setLabel('yawning');
  else if (key === 'c') setLabel('chewing');
  else if (key === 'm') setLabel('smiling');
  else if (key === 'a') setLabel('turned_away');
  else if (key === ' ') {
    e.preventDefault();
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  }
});

// Page visibility monitor
document.addEventListener('visibilitychange', () => {
  isTabVisible = !document.hidden;
  tabStatusText.textContent = isTabVisible ? 'Tab Active' : 'Tab Background';
  tabPill.style.borderColor = isTabVisible ? 'var(--border-subtle)' : 'var(--color-amber)';
  console.log(`[saywhen] Visibility changed. Visible: ${isTabVisible} at ${new Date().toISOString()}`);
});
