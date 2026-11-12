// First-run setup checklist shown on Home until every step is done once.
// Presentation only: main.ts owns the real state and passes it in through
// updateSetupChecklist(). Nothing here talks to the backend.

export interface SetupState {
  /** False until the backend snapshot has loaded, so we never show guesses. */
  ready: boolean;
  modelInstalled: boolean;
  typingAllowed: boolean;
  cameraOn: boolean;
  calibrated: boolean;
  spoke: boolean;
}

export interface SetupActions {
  downloadModel: () => Promise<void> | void;
  openTypingSettings: () => Promise<void> | void;
  turnOnCamera: () => Promise<void> | void;
  calibrate: () => Promise<void> | void;
}

type StepId = 'model' | 'typing' | 'camera' | 'calibrate' | 'speak';

const STEP_ORDER: StepId[] = ['model', 'typing', 'camera', 'calibrate', 'speak'];
const COMPLETE_KEY = 'saywhen-setup-complete';
const HIDDEN_KEY = 'saywhen-setup-hidden';
const BUSY_LABELS: Partial<Record<StepId, string>> = {
  model: 'Downloading...',
  camera: 'Starting...',
  calibrate: 'Hold still...',
};

const state: SetupState = {
  ready: false,
  modelInstalled: false,
  typingAllowed: false,
  cameraOn: false,
  calibrated: false,
  spoke: false,
};

const busy = new Set<StepId>();
let celebrateTimer: number | null = null;

function readFlag(key: string): boolean {
  try {
    return localStorage.getItem(key) === '1';
  } catch {
    return false;
  }
}

function writeFlag(key: string, on: boolean) {
  try {
    if (on) localStorage.setItem(key, '1');
    else localStorage.removeItem(key);
  } catch {
    // Storage can be unavailable. The checklist then simply shows again next launch.
  }
}

function isDone(step: StepId): boolean {
  switch (step) {
    case 'model':
      return state.modelInstalled;
    case 'typing':
      return state.typingAllowed;
    case 'camera':
      // Calibrating or talking proves the camera worked, even if it is off right now.
      return state.cameraOn || state.calibrated;
    case 'calibrate':
      return state.calibrated;
    case 'speak':
      return state.spoke;
  }
}

function render() {
  const card = document.getElementById('setup-card');
  if (!card) return;

  const doneCount = STEP_ORDER.filter(isDone).length;
  const allDone = doneCount === STEP_ORDER.length;
  const celebrating = celebrateTimer !== null;

  if (allDone && state.ready && !readFlag(COMPLETE_KEY)) {
    writeFlag(COMPLETE_KEY, true);
    // Only congratulate when the card was on screen while the last step finished.
    if (!card.classList.contains('hidden')) {
      card.classList.add('is-complete');
      celebrateTimer = window.setTimeout(() => {
        celebrateTimer = null;
        card.classList.add('hidden');
        card.classList.remove('is-complete');
      }, 4500);
    }
  }

  const show =
    celebrateTimer !== null ||
    celebrating ||
    (state.ready && !readFlag(COMPLETE_KEY) && !readFlag(HIDDEN_KEY));
  card.classList.toggle('hidden', !show);
  if (!show) return;

  const count = document.getElementById('setup-count');
  if (count) count.textContent = `${doneCount} of ${STEP_ORDER.length} done`;
  const fill = document.getElementById('setup-bar-fill');
  if (fill) fill.style.width = `${(doneCount / STEP_ORDER.length) * 100}%`;
  const title = document.getElementById('setup-title');
  if (title) title.textContent = allDone ? 'You are all set' : 'Get set up';
  const sub = document.getElementById('setup-sub');
  if (sub) {
    sub.textContent = allDone
      ? 'Look at your screen and talk. Open Setup guide in the sidebar any time.'
      : 'Five quick steps, then you can talk to your terminal. Everything stays on this computer.';
  }

  const current = STEP_ORDER.find((s) => !isDone(s));
  for (const step of STEP_ORDER) {
    const row = card.querySelector<HTMLElement>(`[data-step="${step}"]`);
    if (!row) continue;
    const done = isDone(step);
    row.classList.toggle('is-done', done);
    row.classList.toggle('is-current', step === current);
    row.setAttribute('aria-current', step === current ? 'step' : 'false');

    const status = row.querySelector<HTMLElement>('.setup-status');
    if (status) status.textContent = done ? 'Done' : step === current ? 'Next' : 'To do';

    const btn = row.querySelector<HTMLButtonElement>('[data-setup-action]');
    if (!btn) continue;
    if (!btn.dataset.label) btn.dataset.label = btn.textContent || '';
    btn.classList.toggle('hidden', done);
    btn.classList.toggle('btn-accent', step === current);
    const isBusy = busy.has(step);
    // Calibrating needs a live camera picture.
    const blocked = step === 'calibrate' && !state.cameraOn;
    btn.disabled = isBusy || blocked;
    btn.textContent = isBusy ? BUSY_LABELS[step] || btn.dataset.label : btn.dataset.label;
    btn.title = blocked ? 'Turn on the camera first' : '';
  }
}

export function updateSetupChecklist(partial: Partial<SetupState>) {
  Object.assign(state, partial);
  render();
}

/** Bring the checklist back after the user hid it or finished it. */
export function reopenSetupChecklist() {
  writeFlag(HIDDEN_KEY, false);
  writeFlag(COMPLETE_KEY, false);
  render();
  document.getElementById('setup-card')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
}

export function initSetupChecklist(actions: SetupActions) {
  const card = document.getElementById('setup-card');
  if (!card) return;

  const handlers: Partial<Record<StepId, () => Promise<void> | void>> = {
    model: actions.downloadModel,
    typing: actions.openTypingSettings,
    camera: actions.turnOnCamera,
    calibrate: actions.calibrate,
  };

  card.addEventListener('click', (e) => {
    const target = e.target as HTMLElement;

    if (target.closest('#setup-hide-btn')) {
      writeFlag(HIDDEN_KEY, true);
      render();
      return;
    }

    const btn = target.closest<HTMLButtonElement>('[data-setup-action]');
    if (!btn || btn.disabled) return;
    const step = btn.dataset.setupAction as StepId;
    const run = handlers[step];
    if (!run) return;

    busy.add(step);
    render();
    // Errors are reported by the action itself (main.ts shows them loudly).
    void Promise.resolve()
      .then(run)
      .catch(() => undefined)
      .finally(() => {
        busy.delete(step);
        render();
      });
  });

  render();
}
