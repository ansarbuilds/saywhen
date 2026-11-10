// Living light, pointer glow, and the 7-second product film.
// Presentation only: it never starts the microphone, camera, or injection.

const SCRIPT = 'run the tests and fix any failures';
const DURATION = 8200;
const TYPE_START = 2100;
const TYPE_END = 4100;
const PREVIEW_START = 5000;
const PREVIEW_END = 6500;

type PillState = 'IDLE' | 'ARMED' | 'LISTENING' | 'TRANSCRIBING' | 'PREVIEW' | 'SENT';

const CUES: { t: number; state: PillState; step: string; beat: string; line: string }[] = [
  {
    t: 0,
    state: 'IDLE',
    step: 'look',
    beat: 'Your hands stay off the keyboard.',
    line: 'A terminal is focused. You do not touch it.',
  },
  {
    t: 1000,
    state: 'ARMED',
    step: 'look',
    beat: 'You look at the screen.',
    line: 'The camera sees your face, facing forward.',
  },
  {
    t: 1900,
    state: 'LISTENING',
    step: 'talk',
    beat: 'You talk the way you would to a person.',
    line: 'Lips moving. Voice detected. Listening.',
  },
  {
    t: 4300,
    state: 'TRANSCRIBING',
    step: 'check',
    beat: 'saywhen writes it down, here.',
    line: 'Speech becomes text on this computer.',
  },
  {
    t: PREVIEW_START,
    state: 'PREVIEW',
    step: 'check',
    beat: 'You get a moment to stop it.',
    line: 'Nod to send now. Shake your head or press Esc to cancel.',
  },
  {
    t: PREVIEW_END,
    state: 'SENT',
    step: 'sent',
    beat: 'You nod. It is sent.',
    line: 'Typed into Terminal. Enter pressed. Zero keys.',
  },
];

const PILL_WORD: Record<PillState, string> = {
  IDLE: 'Waiting',
  ARMED: 'Ready',
  LISTENING: 'Listening',
  TRANSCRIBING: 'Writing',
  PREVIEW: 'Check',
  SENT: 'Sent',
};

type Bit = { x: number; y: number; vx: number; vy: number; life: number; size: number; color: string };
type Wave = { x: number; y: number; r: number; life: number };

let closePremiere = () => {};

window.addEventListener('keydown', (event) => {
  const root = document.getElementById('premiere');
  if (!root || root.classList.contains('hidden') || event.key !== 'Escape') return;
  event.preventDefault();
  event.stopImmediatePropagation();
  closePremiere();
});

function motionAllowed(): boolean {
  return (
    !window.matchMedia('(prefers-reduced-motion: reduce)').matches &&
    !document.documentElement.classList.contains('no-motion')
  );
}

export function bootStage() {
  const motionOk = motionAllowed();
  mountAmbient(motionOk);
  mountPointer();
  if (motionOk) mountMagnet();
  mountPremiere(motionOk);
}

function mountPointer() {
  const content = document.querySelector<HTMLElement>('.content');
  const hero = document.getElementById('command-deck');
  if (!content) return;
  window.addEventListener('pointermove', (event) => {
    const rect = content.getBoundingClientRect();
    content.style.setProperty('--spot-x', `${event.clientX - rect.left}px`);
    content.style.setProperty('--spot-y', `${event.clientY - rect.top}px`);
    if (!hero) return;
    const heroRect = hero.getBoundingClientRect();
    const x = ((event.clientX - heroRect.left) / heroRect.width) * 100;
    hero.style.setProperty('--glow-x', `${Math.max(0, Math.min(100, x))}%`);
  });
}

function mountMagnet() {
  document.querySelectorAll<HTMLElement>('.hero .btn').forEach((el) => {
    el.addEventListener('pointermove', (event) => {
      const rect = el.getBoundingClientRect();
      const dx = ((event.clientX - rect.left) / rect.width - 0.5) * 8;
      const dy = ((event.clientY - rect.top) / rect.height - 0.5) * 6;
      el.style.setProperty('--dx', `${dx.toFixed(2)}px`);
      el.style.setProperty('--dy', `${dy.toFixed(2)}px`);
    });
    el.addEventListener('pointerleave', () => {
      el.style.setProperty('--dx', '0px');
      el.style.setProperty('--dy', '0px');
    });
  });
}

function mountAmbient(motionOk: boolean) {
  const canvas = document.createElement('canvas');
  canvas.id = 'ambient';
  canvas.setAttribute('aria-hidden', 'true');
  document.body.prepend(canvas);
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const motes = Array.from({ length: 32 }, () => ({
    x: Math.random(),
    y: Math.random(),
    s: 0.7 + Math.random() * 1.5,
    v: 0.006 + Math.random() * 0.018,
    a: 0.12 + Math.random() * 0.35,
  }));
  const blobs = [
    { x: 0.82, y: 0.06, r: 280, tone: 0, p: 0.4, sp: 0.32 },
    { x: 0.08, y: 0.18, r: 220, tone: 1, p: 1.6, sp: 0.24 },
    { x: 0.55, y: 0.92, r: 300, tone: 0, p: 2.4, sp: 0.2 },
    { x: 0.94, y: 0.72, r: 190, tone: 1, p: 0.8, sp: 0.36 },
  ];

  let accent = '#4a0b3e';
  let hot = '#fbb303';
  let themedAt = 0;
  let mx = 0.72;
  let my = 0.12;
  let tx = mx;
  let ty = my;

  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    canvas.width = Math.floor(window.innerWidth * dpr);
    canvas.height = Math.floor(window.innerHeight * dpr);
    canvas.style.width = `${window.innerWidth}px`;
    canvas.style.height = `${window.innerHeight}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };
  resize();
  window.addEventListener('resize', resize);
  window.addEventListener('pointermove', (event) => {
    tx = event.clientX / Math.max(window.innerWidth, 1);
    ty = event.clientY / Math.max(window.innerHeight, 1);
  });

  const draw = (now: number) => {
    if (now - themedAt > 1600) {
      themedAt = now;
      const styles = getComputedStyle(document.documentElement);
      accent = styles.getPropertyValue('--accent').trim() || accent;
      hot = styles.getPropertyValue('--on-dark').trim() || hot;
    }
    const w = window.innerWidth;
    const h = window.innerHeight;
    const level = Number(document.body.style.getPropertyValue('--level')) || 0;
    mx += (tx - mx) * 0.04;
    my += (ty - my) * 0.04;
    ctx.clearRect(0, 0, w, h);
    const t = now / 1000;
    for (const blob of blobs) {
      const x = blob.x * w + Math.sin(t * blob.sp + blob.p) * 36 + (mx - 0.5) * 48;
      const y = blob.y * h + Math.cos(t * blob.sp * 0.8 + blob.p) * 28 + (my - 0.5) * 36;
      const radius = blob.r * (1 + level * 0.18);
      const g = ctx.createRadialGradient(x, y, 0, x, y, radius);
      g.addColorStop(0, blob.tone === 0 ? hot : accent);
      g.addColorStop(1, 'transparent');
      ctx.globalAlpha = (blob.tone === 0 ? 0.16 : 0.09) + level * 0.08;
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    if (!motionOk) return;
    for (const mote of motes) {
      mote.y -= mote.v;
      if (mote.y < -0.02) mote.y = 1.02;
      ctx.globalAlpha = mote.a;
      ctx.fillStyle = hot;
      ctx.beginPath();
      ctx.arc(mote.x * w, mote.y * h, mote.s, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
  };

  draw(performance.now());
  if (!motionOk) return;

  let last = 0;
  const loop = (now: number) => {
    requestAnimationFrame(loop);
    if (document.hidden || document.body.classList.contains('premiere-open')) return;
    if (now - last < 34) return;
    last = now;
    draw(now);
  };
  requestAnimationFrame(loop);
}

function mountPremiere(motionOk: boolean) {
  const root = document.getElementById('premiere');
  const fx = document.getElementById('premiere-fx') as HTMLCanvasElement | null;
  const beat = document.getElementById('premiere-beat');
  const line = document.getElementById('premiere-line');
  const typed = document.getElementById('premiere-typed');
  const card = document.getElementById('premiere-card');
  const cardText = document.getElementById('premiere-card-text');
  const count = document.getElementById('premiere-count');
  const fill = document.getElementById('premiere-card-fill');
  const pill = document.getElementById('premiere-pill');
  const pillLabel = document.getElementById('premiere-pill-label');
  const keysNote = document.getElementById('premiere-keys-note');
  const scrub = document.getElementById('premiere-scrub-fill');
  const clock = document.getElementById('premiere-clock');
  const closeBtn = document.getElementById('premiere-close') as HTMLButtonElement | null;
  const replayBtn = document.getElementById('premiere-replay') as HTMLButtonElement | null;
  const steps = Array.from(document.querySelectorAll<HTMLElement>('#premiere-steps li'));
  if (!root || !fx || !beat || !line || !typed || !card || !cardText || !count || !fill || !pill || !pillLabel || !keysNote || !scrub || !clock || !closeBtn || !replayBtn) {
    return;
  }
  const fxCtx = fx.getContext('2d');
  if (!fxCtx) return;

  const bits: Bit[] = [];
  const waves: Wave[] = [];
  let raf = 0;
  let startedAt = 0;
  let frozen = false;
  let tonePlayed = false;
  let lastState = '';
  let audio: AudioContext | null = null;

  const resizeFx = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const w = root.clientWidth;
    const h = root.clientHeight;
    fx.width = Math.max(1, Math.floor(w * dpr));
    fx.height = Math.max(1, Math.floor(h * dpr));
    fxCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };

  const burst = () => {
    const pillRect = pill.getBoundingClientRect();
    const rootRect = root.getBoundingClientRect();
    const x = pillRect.left + pillRect.width / 2 - rootRect.left;
    const y = pillRect.top + pillRect.height / 2 - rootRect.top;
    waves.push({ x, y, r: 8, life: 1 });
    for (let i = 0; i < 64; i += 1) {
      const angle = (Math.PI * 2 * i) / 64 + Math.random() * 0.25;
      const speed = 1.4 + Math.random() * 4.2;
      const color = i % 3 === 0 ? '#fbb303' : i % 3 === 1 ? '#fff6df' : '#f0c4ee';
      bits.push({
        x,
        y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed - 1.4,
        life: 1,
        size: 1.8 + Math.random() * 2.8,
        color,
      });
    }
  };

  const playSentTone = () => {
    if (tonePlayed) return;
    tonePlayed = true;
    if (!motionOk) return;
    burst();
    try {
      const Ctx = window.AudioContext;
      if (!audio) audio = new Ctx();
      const now = audio.currentTime;
      const osc = audio.createOscillator();
      const gain = audio.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(494, now);
      osc.frequency.exponentialRampToValueAtTime(784, now + 0.18);
      gain.gain.setValueAtTime(0.0001, now);
      gain.gain.exponentialRampToValueAtTime(0.04, now + 0.03);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.46);
      osc.connect(gain);
      gain.connect(audio.destination);
      osc.start(now);
      osc.stop(now + 0.48);
      void audio.resume();
    } catch {
      // The film still finishes if sound is blocked.
    }
  };

  const apply = (time: number) => {
    const t = Math.max(0, Math.min(DURATION, time));
    let cue = CUES[0];
    for (const next of CUES) {
      if (next.t <= t) cue = next;
    }
    beat.textContent = cue.beat;
    line.textContent = cue.line;
    if (cue.state !== lastState) {
      lastState = cue.state;
      pill.dataset.state = cue.state;
      pillLabel.textContent = PILL_WORD[cue.state];
      pill.classList.remove('is-pop');
      void pill.offsetWidth;
      pill.classList.add('is-pop');
    }
    const order = ['look', 'talk', 'check', 'sent'];
    const index = order.indexOf(cue.step);
    steps.forEach((step, i) => {
      step.classList.toggle('is-now', i === index);
      step.classList.toggle('is-done', i < index);
    });
    let countChars = 0;
    if (t >= TYPE_END) countChars = SCRIPT.length;
    else if (t > TYPE_START) {
      const p = (t - TYPE_START) / (TYPE_END - TYPE_START);
      const eased = 1 - (1 - p) * (1 - p);
      countChars = Math.max(1, Math.round(eased * SCRIPT.length));
    }
    typed.textContent = SCRIPT.slice(0, countChars);
    const previewing = t >= PREVIEW_START && t < PREVIEW_END;
    card.classList.toggle('is-open', previewing);
    if (previewing) {
      const p = (t - PREVIEW_START) / (PREVIEW_END - PREVIEW_START);
      count.textContent = `${(1.8 * (1 - p)).toFixed(1)}s`;
      fill.style.width = `${((1 - p) * 100).toFixed(1)}%`;
      cardText.textContent = SCRIPT;
    }
    const sent = t >= PREVIEW_END;
    root.classList.toggle('is-sent', sent);
    keysNote.textContent = sent ? 'Still zero.' : '0 keys pressed';
    scrub.style.width = `${(t / DURATION) * 100}%`;
    clock.textContent = `${(t / 1000).toFixed(1)}s`;
    if (sent) playSentTone();
  };

  const drawFx = () => {
    const w = root.clientWidth;
    const h = root.clientHeight;
    fxCtx.clearRect(0, 0, w, h);
    for (let i = waves.length - 1; i >= 0; i -= 1) {
      const wave = waves[i];
      wave.r += 7;
      wave.life -= 0.025;
      if (wave.life <= 0) {
        waves.splice(i, 1);
        continue;
      }
      fxCtx.beginPath();
      fxCtx.arc(wave.x, wave.y, wave.r, 0, Math.PI * 2);
      fxCtx.strokeStyle = `rgba(251, 179, 3, ${wave.life})`;
      fxCtx.lineWidth = 2;
      fxCtx.stroke();
    }
    for (let i = bits.length - 1; i >= 0; i -= 1) {
      const bit = bits[i];
      bit.vy += 0.05;
      bit.x += bit.vx;
      bit.y += bit.vy;
      bit.life -= 0.014;
      if (bit.life <= 0) {
        bits.splice(i, 1);
        continue;
      }
      fxCtx.globalAlpha = Math.max(bit.life, 0);
      fxCtx.fillStyle = bit.color;
      fxCtx.beginPath();
      fxCtx.arc(bit.x, bit.y, bit.size, 0, Math.PI * 2);
      fxCtx.fill();
    }
    fxCtx.globalAlpha = 1;
  };

  const stop = () => {
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  };

  const tick = (now: number) => {
    if (root.classList.contains('hidden')) {
      raf = 0;
      return;
    }
    if (!frozen) {
      const elapsed = now - startedAt;
      if (elapsed >= DURATION) {
        frozen = true;
        apply(DURATION);
      } else {
        apply(elapsed);
      }
    }
    drawFx();
    if (frozen && bits.length === 0 && waves.length === 0) {
      raf = 0;
      return;
    }
    raf = requestAnimationFrame(tick);
  };

  const primeAudio = () => {
    if (!motionOk) return;
    try {
      if (!audio) audio = new AudioContext();
      void audio.resume();
    } catch {
      // Sound is optional.
    }
  };

  const replay = () => {
    stop();
    frozen = false;
    tonePlayed = false;
    lastState = '';
    bits.length = 0;
    waves.length = 0;
    root.classList.remove('is-sent');
    card.classList.remove('is-open');
    startedAt = performance.now();
    requestAnimationFrame(() => {
      resizeFx();
      if (!motionOk) {
        apply(DURATION);
        return;
      }
      apply(0);
      raf = requestAnimationFrame(tick);
    });
  };

  const open = () => {
    primeAudio();
    root.classList.remove('hidden');
    root.setAttribute('aria-hidden', 'false');
    document.body.classList.add('premiere-open');
    root.scrollTop = 0;
    replay();
    closeBtn.focus();
  };

  const close = () => {
    stop();
    bits.length = 0;
    waves.length = 0;
    root.classList.add('hidden');
    root.setAttribute('aria-hidden', 'true');
    document.body.classList.remove('premiere-open');
  };

  closePremiere = close;
  closeBtn.addEventListener('click', close);
  replayBtn.addEventListener('click', () => {
    primeAudio();
    replay();
  });
  document.querySelectorAll<HTMLElement>('[data-premiere-open]').forEach((btn) => {
    btn.addEventListener('click', open);
  });
  window.addEventListener('resize', () => {
    if (!root.classList.contains('hidden')) resizeFx();
  });

  const params = new URLSearchParams(window.location.search);
  if (params.get('premiere') === '1') open();
}
