export const ICON_NAMES = [
  'home',
  'settings',
  'activity',
  'book',
  'mic',
  'shield',
  'history',
  'spark',
  'check',
  'close',
] as const;

export type IconName = (typeof ICON_NAMES)[number];

const PATHS: Record<IconName, string> = {
  home: 'M3.5 9 10 3.8 16.5 9v6.7a1 1 0 0 1-1 1h-3.3v-4.4H7.8v4.4H4.5a1 1 0 0 1-1-1z',
  settings: 'M4 6h7M15 6h1M4 14h1M9 14h7M13 4.2a2 2 0 1 1 0 3.6M7 12.2a2 2 0 1 1 0 3.6',
  activity: 'M3 11h2.5l2-5 3 9 2-6 1.5 2H17',
  book: 'M4 4.5A1.5 1.5 0 0 1 5.5 3H15v11.5H5.5A1.5 1.5 0 0 0 4 16zM4 16a1.5 1.5 0 0 0 1.5 1.5H15V14.5M7.5 7h4.5M7.5 10h3',
  mic: 'M7.5 3h5v9a2.5 2.5 0 0 1-5 0zM5 9.5a5 5 0 0 0 10 0M10 14.5V17',
  shield: 'M10 2.8 4.5 5v4.4c0 3.6 2.4 6.4 5.5 7.8 3.1-1.4 5.5-4.2 5.5-7.8V5z',
  history: 'M10 4.5a5.5 5.5 0 1 1-4.8 8.2M10 7v3.2l2 1.2M4.2 8.2 5.5 6.2 7.6 7.4',
  spark: 'M10 2.5 11.4 8 17 9.2 11.4 10.6 10 16.5 8.6 10.6 3 9.2 8.6 8z',
  check: 'M4.5 10.2 8.2 14 15.5 6.2',
  close: 'm6 6 8 8M14 6l-8 8',
};

export function iconSvg(name: IconName, size = 20): string {
  const path = PATHS[name];
  return `<svg class="icon icon-${name}" viewBox="0 0 20 20" width="${size}" height="${size}" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="${path}" /></svg>`;
}

export function isIconName(value: string): value is IconName {
  return (ICON_NAMES as readonly string[]).includes(value);
}
