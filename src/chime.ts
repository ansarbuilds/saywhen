// Soft two-note chime played when saywhen hears the wake word. Synthesized with Web Audio,
// so there is no sound file to ship. Kept quiet and short (about 0.25 s) so the microphone
// does not mistake it for speech.

let ctx: AudioContext | null = null;

function context(): AudioContext | null {
  if (ctx) return ctx;
  try {
    ctx = new AudioContext();
  } catch {
    ctx = null;
  }
  return ctx;
}

// Webviews may start audio suspended until the user interacts once. Unlock on the first
// click or key press in the window.
export function unlockChimeOnFirstInteraction() {
  const unlock = () => {
    void context()?.resume().catch(() => undefined);
    window.removeEventListener('pointerdown', unlock);
    window.removeEventListener('keydown', unlock);
  };
  window.addEventListener('pointerdown', unlock);
  window.addEventListener('keydown', unlock);
}

function note(audio: AudioContext, freq: number, start: number, length: number, peak: number) {
  const osc = audio.createOscillator();
  const gain = audio.createGain();
  osc.type = 'sine';
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0.0001, start);
  gain.gain.exponentialRampToValueAtTime(peak, start + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
  osc.connect(gain).connect(audio.destination);
  osc.start(start);
  osc.stop(start + length + 0.02);
}

/** Rising pair when the command is already coming; a single note when waiting for it. */
export async function playWakeChime(waitingForCommand: boolean): Promise<boolean> {
  const audio = context();
  if (!audio) return false;
  if (audio.state !== 'running') {
    try {
      await audio.resume();
    } catch {
      return false;
    }
  }
  const t = audio.currentTime + 0.01;
  note(audio, 880, t, 0.16, 0.07); // A5
  if (!waitingForCommand) {
    note(audio, 1318.5, t + 0.09, 0.18, 0.06); // E6
  }
  return audio.state === 'running';
}
