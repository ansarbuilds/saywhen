// A short, skippable brand moment for the first launch. It never blocks the
// engine and it remembers that it has been seen.
const SEEN_KEY = 'saywhen-intro-seen-v1';

let hideTimer: number | null = null;

export function openIntro(): void {
  const root = document.getElementById('intro');
  if (!root) return;
  if (document.querySelector('.command-palette:not(.hidden), .modal-backdrop:not(.hidden)')) return;
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
    hideTimer = null;
  }
  root.classList.remove('hidden');
  root.setAttribute('aria-hidden', 'false');
  const skip = document.getElementById('intro-skip');
  window.requestAnimationFrame(() => skip?.focus());
  const reduce =
    window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
    document.documentElement.dataset.motion === 'reduced';
  hideTimer = window.setTimeout(closeIntro, reduce ? 700 : 3200);
}

export function closeIntro(): void {
  const root = document.getElementById('intro');
  if (!root) return;
  if (document.querySelector('.command-palette:not(.hidden), .modal-backdrop:not(.hidden)')) return;
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
    hideTimer = null;
  }
  root.classList.add('hidden');
  root.setAttribute('aria-hidden', 'true');
  try {
    localStorage.setItem(SEEN_KEY, '1');
  } catch {
    // Storage can be unavailable; the intro simply shows again next launch.
  }
}

export function initIntro(): void {
  const root = document.getElementById('intro');
  if (!root) return;
  const skip = document.getElementById('intro-skip');
  skip?.addEventListener('click', closeIntro);
  root.addEventListener('click', (event) => {
    if (event.target === root) closeIntro();
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && !root.classList.contains('hidden')) {
      event.preventDefault();
      closeIntro();
    }
  });

  let seen = false;
  try {
    seen = localStorage.getItem(SEEN_KEY) === '1';
  } catch {
    seen = false;
  }
  if (!seen) {
    window.setTimeout(openIntro, 420);
  }
}
