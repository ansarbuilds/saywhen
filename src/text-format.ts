export function formatSignedDeg(deg: number): string {
  const rounded = Math.abs(deg) < 0.05 ? 0 : deg;
  return `${rounded >= 0 ? '+' : ''}${rounded.toFixed(1)}°`;
}

export function formatDurationMs(ms: number): string {
  const totalTenths = Math.floor(ms / 100);
  const tenths = totalTenths % 10;
  const totalSeconds = Math.floor(totalTenths / 10);
  const seconds = totalSeconds % 60;
  const minutes = Math.floor(totalSeconds / 60);
  const mm = String(minutes).padStart(2, '0');
  const ss = String(seconds).padStart(2, '0');
  return `${mm}:${ss}.${tenths}`;
}

export function formatCountdownSec(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

export function escapeHtml(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export function formatTriggerSource(source: string): string {
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

export function formatAccelerator(accelerator: string, mac: boolean): string {
  const parts = (accelerator || 'Alt+Space').split('+');
  const labels: Record<string, string> = mac
    ? { Cmd: '⌘', Ctrl: '⌃', Alt: '⌥', Shift: '⇧' }
    : { Cmd: 'Win', Ctrl: 'Ctrl', Alt: 'Alt', Shift: 'Shift' };
  return parts
    .map((part) => labels[part] ?? (part === 'Space' ? 'Space' : part.replace(/^Key/, '')))
    .join(' + ');
}

export function acceleratorFromCodes(input: {
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  code: string;
}): string | null {
  const modifiers: string[] = [];
  if (input.metaKey) modifiers.push('Cmd');
  if (input.ctrlKey) modifiers.push('Ctrl');
  if (input.altKey) modifiers.push('Alt');
  if (input.shiftKey) modifiers.push('Shift');
  if (modifiers.length === 0) return null;
  if (/^(Meta|Control|Alt|Shift)/.test(input.code)) return null;
  return [...modifiers, input.code].join('+');
}

export function splitAllowlist(value: string): string[] {
  const seen = new Set<string>();
  const items: string[] = [];
  for (const raw of value.split(',')) {
    const name = raw.trim();
    if (!name) continue;
    const key = name.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    items.push(name);
  }
  return items;
}
