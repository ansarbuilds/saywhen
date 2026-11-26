export interface LicenseView {
  eyebrow: string;
  headline: string;
  detail: string;
  action: string;
}

export function licenseView(input: {
  tier: string;
  trialDaysLeft: number;
  checker: string;
  message: string;
}): LicenseView {
  if (input.tier === 'pro') {
    return {
      eyebrow: 'Pro',
      headline: 'You are on Pro.',
      detail: input.message,
      action: 'Manage key',
    };
  }
  if (input.tier === 'trial') {
    const days = Math.max(0, input.trialDaysLeft);
    return {
      eyebrow: 'Trial',
      headline: days === 1 ? '1 day left in the trial.' : `${days} days left in the trial.`,
      detail: 'Enforcement is off, so nothing is locked. A real license server is required before this can charge money.',
      action: 'Add a license key',
    };
  }
  return {
    eyebrow: 'Free',
    headline: 'The trial has ended.',
    detail: input.checker === 'development'
      ? 'The development checker is public. Do not sell keys it accepts.'
      : input.message,
    action: 'Add a license key',
  };
}

export function onboardingProgress(done: number, total: number): string {
  const safeTotal = Math.max(1, total);
  const safeDone = Math.max(0, Math.min(safeTotal, done));
  return `${safeDone} of ${safeTotal} ready`;
}

export function hudHint(state: string): string {
  switch (state) {
    case 'LISTENING':
      return 'Listening';
    case 'TRANSCRIBING':
      return 'Writing';
    case 'PREVIEW':
      return 'Check';
    case 'ARMED':
      return 'Ready';
    default:
      return 'Waiting';
  }
}

export function quietHoursSummary(enabled: boolean, start: string, end: string): string {
  if (!enabled) return 'Quiet hours are off. Hands-free can start any time.';
  return `Hands-free stays off from ${start} to ${end}. The hotkey still works.`;
}

export type Density = 'comfortable' | 'compact';

export function densityMode(value: string | null | undefined): Density {
  return value === 'compact' ? 'compact' : 'comfortable';
}

export const ONBOARDING_STEPS = [
  { id: 'model', title: 'Speech model' },
  { id: 'mic', title: 'Microphone' },
  { id: 'camera', title: 'Camera' },
  { id: 'typing', title: 'Typing' },
] as const;
