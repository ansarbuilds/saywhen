// Presentation-only helpers for the main window: sidebar navigation, view
// transitions, slider fills, the state orb's live audio level and hero copy
// swap animation. Nothing here talks to the backend.

type TabId = 'home' | 'preferences' | 'lab';

const TAB_META: Record<TabId, { num: string; title: string; subtitle: string }> = {
  home: { num: '01', title: 'Home', subtitle: 'Talk, check, send' },
  preferences: { num: '02', title: 'Settings', subtitle: 'Listening, sending and privacy' },
  lab: { num: '03', title: 'Lab', subtitle: 'Live numbers and a practice box' },
};

const STATE_WORDS: Record<string, string> = {
  IDLE: 'Waiting',
  ARMED: 'Ready',
  LISTENING: 'Listening',
  TRANSCRIBING: 'Writing',
  PREVIEW: 'Check',
};

export const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

let activeTab: TabId = 'home';
let lastState = '';
let smoothedLevel = 0;
let levelFrame = 0;
let targetLevel = 0;

function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function replay(el: Element | null, cls: string) {
  if (!el) return;
  el.classList.remove(cls);
  void (el as HTMLElement).offsetWidth;
  el.classList.add(cls);
}

function moveNavIndicator() {
  const nav = document.getElementById('app-nav');
  const indicator = document.getElementById('nav-indicator');
  const active = nav?.querySelector<HTMLElement>('.nav-item.is-active');
  if (!nav || !indicator || !active) return;
  indicator.style.setProperty('--nav-y', `${active.offsetTop}px`);
  indicator.style.height = `${active.offsetHeight}px`;
}

export function switchTab(tab: TabId) {
  if (tab === activeTab && document.querySelector(`.view[data-view="${tab}"].is-active`)) return;
  const prev = document.querySelector<HTMLElement>('.view.is-active');
  const next = document.querySelector<HTMLElement>(`.view[data-view="${tab}"]`);
  if (!next) return;

  document.querySelectorAll<HTMLElement>('.nav-item').forEach((btn) => {
    const on = btn.dataset.tab === tab;
    btn.classList.toggle('is-active', on);
    if (on) btn.setAttribute('aria-current', 'page');
    else btn.removeAttribute('aria-current');
  });
  moveNavIndicator();

  const heading = document.querySelector('.page-heading');
  const title = document.getElementById('page-title');
  const subtitle = document.getElementById('page-subtitle');
  const kicker = document.querySelector('.page-kicker .stage-tag');
  if (kicker) kicker.textContent = `${TAB_META[tab].num} / saywhen`;
  if (title) title.textContent = TAB_META[tab].title;
  if (subtitle) subtitle.textContent = TAB_META[tab].subtitle;
  replay(heading, 'is-swapping');

  if (prev && prev !== next) {
    prev.classList.remove('is-active');
    if (!prefersReducedMotion()) {
      prev.classList.add('is-leaving');
      window.setTimeout(() => prev.classList.remove('is-leaving'), 180);
    }
  }
  next.classList.add('is-active');
  next.scrollTop = 0;
  activeTab = tab;

  // Canvases measure their box on draw; nudge them once the view is laid out.
  requestAnimationFrame(() => window.dispatchEvent(new Event('resize')));
}

export function refreshRangeFills() {
  document.querySelectorAll<HTMLInputElement>('input[type="range"].range-slider').forEach((input) => {
    const min = parseFloat(input.min || '0');
    const max = parseFloat(input.max || '100');
    const val = parseFloat(input.value);
    const pct = max > min ? ((val - min) / (max - min)) * 100 : 0;
    input.style.setProperty('--pct', `${pct}%`);
  });
}

function tickLevel() {
  smoothedLevel += (targetLevel - smoothedLevel) * 0.35;
  if (Math.abs(targetLevel - smoothedLevel) < 0.005) smoothedLevel = targetLevel;
  document.body.style.setProperty('--level', smoothedLevel.toFixed(3));
  levelFrame = smoothedLevel !== targetLevel ? requestAnimationFrame(tickLevel) : 0;
}

/** Feed the hero orb a 0..1 loudness value derived from mic RMS and VAD. */
export function setAudioLevel(rms: number, vadProb: number, isSpeech: boolean) {
  const level = Math.min(1, Math.max(isSpeech ? vadProb * 0.6 : 0, rms * 7));
  targetLevel = level;
  if (!levelFrame) levelFrame = requestAnimationFrame(tickLevel);
}

/** Reflect the pipeline state on <body> so CSS can theme and animate it. */
export function setUiState(state: string) {
  if (state === lastState) return;
  lastState = state;
  document.body.dataset.state = state;
  const word = document.getElementById('hero-state-word');
  if (word) word.textContent = STATE_WORDS[state] ?? state;
  replay(document.getElementById('command-deck'), 'is-swapping');
  if (state !== 'LISTENING' && state !== 'ARMED') setAudioLevel(0, 0, false);
}

/** Reflect the chosen send mode on <body> so CSS can adapt hints and accents. */
export function setSendModeUi(mode: string) {
  document.body.dataset.sendMode = mode;
}

function initNav() {
  if (isMac) document.documentElement.classList.add('is-mac');
  if (new URLSearchParams(window.location.search).has('demo')) {
    document.documentElement.classList.add('no-motion');
  }
  const modLabel = isMac ? '⌘' : 'Ctrl ';
  document.querySelectorAll<HTMLElement>('.nav-kbd[data-mod]').forEach((k) => {
    k.textContent = `${modLabel}${k.textContent}`;
  });

  document.querySelectorAll<HTMLButtonElement>('.nav-item').forEach((btn) => {
    btn.addEventListener('click', () => switchTab(btn.dataset.tab as TabId));
  });

  window.addEventListener('keydown', (e) => {
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (!mod || e.altKey || e.shiftKey) return;
    const map: Record<string, TabId> = { '1': 'home', '2': 'preferences', '3': 'lab' };
    const tab = map[e.key];
    if (tab) {
      e.preventDefault();
      switchTab(tab);
    }
  });

  document.querySelectorAll<HTMLInputElement>('input[type="range"].range-slider').forEach((input) => {
    input.addEventListener('input', refreshRangeFills);
  });
  refreshRangeFills();

  window.addEventListener('resize', moveNavIndicator);
  requestAnimationFrame(moveNavIndicator);

  const kicker = document.querySelector('.page-kicker .stage-tag');
  if (kicker) kicker.textContent = `${TAB_META[activeTab].num} / saywhen`;

  const requested = new URLSearchParams(window.location.search).get('tab');
  if (requested === 'preferences' || requested === 'lab' || requested === 'home') {
    switchTab(requested);
  }
}

initNav();
