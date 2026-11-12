// Floating HUD pill. Runs in the transparent, click-through, always-on-top `hud`
// window. It only listens to backend events; it never invokes commands and never
// takes focus, so the frontmost app stays the injection target.
import { listen } from '@tauri-apps/api/event';
import './fonts';

interface AudioTelemetryEvent {
  state: string;
  rms: number;
  vadProb: number;
  isSpeech: boolean;
  listeningDurationMs: number;
  previewRemainingMs: number;
  silenceRemainingMs?: number;
  silenceTotalMs?: number;
  postRollActive?: boolean;
  visionActive: boolean;
  visionArmed: boolean;
  cameraEnabled: boolean;
  hudEnabled: boolean;
  uiPalette?: string;
  moodColors?: boolean;
  saveTuningAudioEnabled?: boolean;
  partialTranscript?: string;
  partialEndsWithConnective?: boolean;
  sendMode?: string;
  wakePending?: boolean;
}

interface PreviewStartedEvent {
  cleanup: { cleanedText: string; isDestructiveCommand: boolean };
  targetPolicy: { appName: string; policy: string; willPressEnter: boolean };
  countdownMs: number;
  waitsForNod?: boolean;
}

interface UtteranceCompleteEvent {
  targetPolicy: { appName: string };
  injection: {
    success: boolean;
    pressedEnter: boolean;
    targetApp: string;
    policyApplied: string;
    message: string;
  } | null;
  injectionError: string | null;
}

interface DictationConfigLite {
  hudEnabled: boolean;
  uiPalette?: string;
  moodColors?: boolean;
  sendMode?: string;
  cancelOnSpeech?: boolean;
  saveTuningAudioEnabled?: boolean;
  partialTranscript?: string;
  partialEndsWithConnective?: boolean;
}

const FLASH_MS = 2600;

const root = document.getElementById('hud-root') as HTMLElement;
const label = document.getElementById('hud-label') as HTMLElement;
const HUD_STATE_WORDS: Record<string, string> = {
  IDLE: 'Waiting',
  ARMED: 'Ready',
  LISTENING: 'Listening',
  TRANSCRIBING: 'Writing',
  PREVIEW: 'Check',
};
const recBadge = document.getElementById('hud-rec') as HTMLElement | null;
const detail = document.getElementById('hud-detail') as HTMLElement;
const meterBars = Array.from(document.querySelectorAll<HTMLElement>('#hud-meter i'));
const previewCard = document.getElementById('hud-preview') as HTMLElement;
const previewTarget = document.getElementById('hud-preview-target') as HTMLElement;
const previewCountdown = document.getElementById('hud-preview-countdown') as HTMLElement;
const previewText = document.getElementById('hud-preview-text') as HTMLElement;
const previewFill = document.getElementById('hud-preview-fill') as HTMLElement;
const previewHint = document.getElementById('hud-preview-hint') as HTMLElement;

const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
function formatHotkey(raw: string): string {
  const labels: Record<string, string> = isMac
    ? { Cmd: '⌘', Ctrl: '⌃', Alt: '⌥', Shift: '⇧' }
    : { Cmd: 'Win', Ctrl: 'Ctrl', Alt: 'Alt', Shift: 'Shift' };
  return (raw || 'Alt+Space')
    .split('+')
    .map((part) => labels[part] ?? (part === 'Space' ? 'Space' : part.replace(/^Key/, '')))
    .join('+');
}

let hotkeyLabel = formatHotkey(
  (() => {
    try {
      return localStorage.getItem('saywhen-hotkey') || 'Alt+Space';
    } catch {
      return 'Alt+Space';
    }
  })()
);
window.addEventListener('storage', (event) => {
  if (event.key === 'saywhen-hotkey') hotkeyLabel = formatHotkey(event.newValue || 'Alt+Space');
});

let previewTotalMs = 1800;
let previewWaitsForNod = false;
let sendMode = 'pause';
let cancelOnSpeech = true;
let flash: { text: string; kind: 'ok' | 'error'; until: number } | null = null;
let hudEnabled = true;

function formatSeconds(ms: number): string {
  const s = ms / 1000;
  const m = Math.floor(s / 60);
  const rest = (s % 60).toFixed(1).padStart(4, '0');
  return `${m}:${rest}`;
}

function setFlash(text: string, kind: 'ok' | 'error') {
  flash = { text, kind, until: performance.now() + FLASH_MS };
}

// Center bars respond most, edges least, so the meter reads as a waveform.
const BAR_WEIGHTS = [0.55, 0.85, 1, 0.8, 0.6];
let meterLevel = 0;

function renderMeter(t: AudioTelemetryEvent) {
  // RMS of normal speech sits around 0.02 to 0.15; VAD probability gives the speech shape.
  const raw = Math.min(1, Math.max(t.isSpeech ? t.vadProb : 0, t.rms * 8));
  // Fast attack, slower release keeps the bars lively without flicker.
  meterLevel = raw > meterLevel ? raw : meterLevel * 0.7 + raw * 0.3;
  const now = performance.now();
  meterBars.forEach((bar, i) => {
    const weight = BAR_WEIGHTS[i % BAR_WEIGHTS.length];
    const wobble = meterLevel > 0.05 ? 0.12 * Math.sin(now / 90 + i * 1.7) : 0;
    const scale = Math.max(0.18, Math.min(1, meterLevel * weight + wobble));
    bar.classList.toggle('on', meterLevel >= (i + 0.5) / meterBars.length || scale > 0.3);
    bar.style.setProperty('--s', scale.toFixed(3));
  });
}

let lastRenderedState = '';

function replaySwap() {
  root.classList.remove('is-swapping');
  void root.offsetWidth;
  root.classList.add('is-swapping');
}

function render(t: AudioTelemetryEvent) {
  hudEnabled = t.hudEnabled;
  if (t.sendMode) sendMode = t.sendMode;
  if (t.uiPalette && root.dataset.palette !== t.uiPalette) root.dataset.palette = t.uiPalette;
  const mood = t.moodColors === false ? 'off' : 'on';
  if (root.dataset.moodColors !== mood) root.dataset.moodColors = mood;
  root.classList.toggle('hud-off', !hudEnabled);
  if (t.state !== lastRenderedState) {
    lastRenderedState = t.state;
    root.dataset.state = t.state;
    label.textContent = HUD_STATE_WORDS[t.state] ?? t.state;
    replaySwap();
  }
  if (recBadge) {
    recBadge.classList.toggle('hidden', !t.saveTuningAudioEnabled);
  }

  const stalled = t.cameraEnabled && !t.visionActive;
  const now = performance.now();
  if (flash && now > flash.until) flash = null;

  let text: string;
  let warn = false;
  switch (t.state) {
    case 'LISTENING': {
      const rem = t.silenceRemainingMs ?? 0;
      const tot = t.silenceTotalMs ?? 0;
      const partial = (t.partialTranscript ?? '').trim();
      const snippet =
        partial.length > 34 ? `...${partial.slice(-31)}` : partial;
      if (t.wakePending) {
        text = `${formatSeconds(t.listeningDurationMs)}  checking for the wake word...`;
      } else if (t.postRollActive) {
        text = `${formatSeconds(t.listeningDurationMs)}  finishing...`;
      } else if (tot > 0 && rem < tot && rem <= tot - 120) {
        const holdTag = t.partialEndsWithConnective ? ' (connective hold)' : '';
        text = `${formatSeconds(t.listeningDurationMs)}  end in ${(rem / 1000).toFixed(1)}s${holdTag}`;
      } else if (snippet) {
        text = `${formatSeconds(t.listeningDurationMs)}  "${snippet}"`;
      } else {
        text = `${formatSeconds(t.listeningDurationMs)}  ${listeningHint()}`;
      }
      break;
    }
    case 'TRANSCRIBING':
      text = 'Writing it down...';
      break;
    case 'PREVIEW': {
      const remaining = Math.max(0, t.previewRemainingMs);
      text = previewWaitsForNod
        ? `Nod to send (${(remaining / 1000).toFixed(0)}s)`
        : `Sending in ${(remaining / 1000).toFixed(1)}s`;
      previewCountdown.textContent = `${(remaining / 1000).toFixed(1)}s`;
      const pct = Math.max(0, Math.min(100, (remaining / previewTotalMs) * 100));
      previewFill.style.width = `${pct}%`;
      break;
    }
    case 'ARMED':
      text = 'Ready. Start talking';
      break;
    default:
      if (stalled) {
        text = 'Camera gate stalled: no vision frames. Open saywhen to check.';
        warn = true;
      } else if (t.cameraEnabled) {
        text = `Look at the screen and talk, or ${hotkeyLabel}`;
      } else {
        text = `${hotkeyLabel} to talk. Camera is off`;
      }
  }

  if (flash && (t.state === 'IDLE' || t.state === 'ARMED')) {
    text = flash.text;
    warn = flash.kind === 'error';
  }

  root.classList.toggle('is-stalled', warn && !flash);
  root.classList.toggle('is-error', !!flash && flash.kind === 'error');
  detail.textContent = text;

  if (t.state !== 'PREVIEW') {
    previewCard.classList.remove('is-open');
  }
  renderMeter(t);
}

function listeningHint(): string {
  switch (sendMode) {
    case 'nod':
      return 'nod when done';
    case 'phrase':
      return 'say "send it" to send';
    case 'paste_only':
      return 'stop talking to type it';
    default:
      return 'stop talking or nod';
  }
}

function previewHintText(): string {
  if (previewWaitsForNod) return 'Nod or Enter to send. Shake or Esc to cancel.';
  return cancelOnSpeech
    ? 'Say "cancel", shake or Esc to stop. Nod to send now.'
    : 'Shake or Esc to stop. Nod to send now.';
}

function showPreview(ev: PreviewStartedEvent) {
  previewTotalMs = ev.countdownMs || 1800;
  previewWaitsForNod = Boolean(ev.waitsForNod);
  previewCard.classList.toggle('waits-for-nod', previewWaitsForNod);
  previewText.textContent = ev.cleanup.cleanedText;
  previewTarget.className = 'preview-target';
  const app = ev.targetPolicy.appName || 'focused app';
  if (ev.targetPolicy.policy === 'self_guard') {
    previewTarget.classList.add('policy-danger');
    previewTarget.textContent = 'Target is saywhen: focus a terminal or editor';
  } else if (ev.targetPolicy.policy === 'destructive_guard') {
    previewTarget.classList.add('policy-danger');
    previewTarget.textContent = `Destructive command: paste only into ${app}`;
  } else if (!ev.targetPolicy.willPressEnter) {
    previewTarget.classList.add('policy-warn');
    previewTarget.textContent = `Paste into ${app} (no Enter)`;
  } else {
    previewTarget.textContent = `Paste + Enter into ${app}`;
  }
  previewHint.textContent = previewHintText();
  previewCountdown.textContent = `${(previewTotalMs / 1000).toFixed(1)}s`;
  previewFill.style.width = '100%';
  previewCard.classList.add('is-open');
}

function onUtteranceComplete(ev: UtteranceCompleteEvent) {
  if (ev.injectionError) {
    setFlash(`Injection failed: ${ev.injectionError}`, 'error');
  } else if (ev.injection) {
    const app = ev.injection.targetApp || ev.targetPolicy.appName;
    if (ev.injection.policyApplied === 'target_changed_paste_only') {
      setFlash(ev.injection.message || `Target changed: pasted into ${app} without Enter`, 'error');
    } else {
      setFlash(
        ev.injection.message || (ev.injection.pressedEnter ? `Sent to ${app}` : `Pasted into ${app}`),
        'ok'
      );
    }
  } else {
    setFlash('Transcribed (auto-inject off)', 'ok');
  }
  previewCard.classList.remove('is-open');
}

async function initHud() {
  await listen<AudioTelemetryEvent>('audio-telemetry', (e) => render(e.payload));
  await listen<PreviewStartedEvent>('preview-started', (e) => showPreview(e.payload));
  await listen<string>('preview-canceled', (e) => {
    previewCard.classList.remove('is-open');
    setFlash(`Canceled: ${e.payload}`, 'ok');
  });
  await listen<string>('utterance-canceled', () => setFlash('Utterance canceled', 'ok'));
  await listen<string>('utterance-discarded', (e) => setFlash(e.payload, 'ok'));
  await listen<UtteranceCompleteEvent>('utterance-complete', (e) => onUtteranceComplete(e.payload));
  await listen<string>('pipeline-error', (e) => setFlash(e.payload, 'error'));
  await listen<DictationConfigLite>('dictation-config-updated', (e) => {
    hudEnabled = e.payload.hudEnabled;
    root.classList.toggle('hud-off', !hudEnabled);
    if (e.payload.sendMode) sendMode = e.payload.sendMode;
    if (typeof e.payload.cancelOnSpeech === 'boolean') cancelOnSpeech = e.payload.cancelOnSpeech;
    if (e.payload.uiPalette) root.dataset.palette = e.payload.uiPalette;
    root.dataset.moodColors = e.payload.moodColors === false ? 'off' : 'on';
    if (recBadge && typeof e.payload.saveTuningAudioEnabled === 'boolean') {
      recBadge.classList.toggle('hidden', !e.payload.saveTuningAudioEnabled);
    }
  });
}

// Static states for browser screenshots: hud.html?demo=listening etc.
function applyDemo(state: string) {
  const base: AudioTelemetryEvent = {
    state: 'IDLE',
    rms: 0,
    vadProb: 0,
    isSpeech: false,
    listeningDurationMs: 0,
    previewRemainingMs: 0,
    visionActive: true,
    visionArmed: false,
    cameraEnabled: true,
    hudEnabled: true,
  };
  document.body.style.background = '#e6d5bd';
  if (state === 'preview') {
    showPreview({
      cleanup: { cleanedText: 'cargo test --release src/main.rs', isDestructiveCommand: false },
      targetPolicy: { appName: 'Terminal', policy: 'auto_send', willPressEnter: true },
      countdownMs: 1800,
    });
    render({ ...base, state: 'PREVIEW', previewRemainingMs: 1100 });
  } else if (state === 'listening') {
    render({ ...base, state: 'LISTENING', rms: 0.08, vadProb: 0.9, isSpeech: true, listeningDurationMs: 3400 });
  } else if (state === 'armed') {
    render({ ...base, state: 'ARMED', visionArmed: true });
  } else if (state === 'transcribing') {
    render({ ...base, state: 'TRANSCRIBING' });
  } else if (state === 'stalled') {
    render({ ...base, visionActive: false });
  } else {
    render({ ...base, cameraEnabled: false });
  }
}

const demo = new URLSearchParams(window.location.search).get('demo');
if (demo) {
  document.documentElement.classList.add('no-motion');
  applyDemo(demo);
} else {
  initHud().catch((err) => {
    // Fail loudly inside the pill itself; there is nowhere else to show it.
    root.classList.add('is-error');
    detail.textContent = `HUD could not subscribe to events: ${String(err)}`;
  });
}
