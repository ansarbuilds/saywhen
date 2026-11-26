export type HistoryDay = 'Today' | 'Yesterday' | 'Earlier';

export interface HistoryCardModel {
  text: string;
  rawTitle: string;
  timeLabel: string;
  relativeLabel: string;
  dayGroup: HistoryDay;
  sourceLabel: string;
  wordCount: number;
  wordLabel: string;
  inferenceLabel: string;
  deliveryLabel: string;
  cleaned: boolean;
  destructive: boolean;
  searchText: string;
}

export function historyDayGroup(timestampMs: number, nowMs: number): HistoryDay {
  const day = new Date(timestampMs);
  const now = new Date(nowMs);
  const start = (value: Date) => new Date(value.getFullYear(), value.getMonth(), value.getDate()).getTime();
  const delta = Math.round((start(now) - start(day)) / 86_400_000);
  if (delta <= 0) return 'Today';
  if (delta === 1) return 'Yesterday';
  return 'Earlier';
}

export function relativeTimeLabel(timestampMs: number, nowMs: number): string {
  const delta = Math.max(0, nowMs - timestampMs);
  const minutes = Math.floor(delta / 60_000);
  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function wordCount(text: string): number {
  const trimmed = text.trim();
  if (!trimmed) return 0;
  return trimmed.split(/\s+/).length;
}

export function historyCardModel(input: {
  cleanedText: string;
  rawText: string;
  appName: string;
  triggerSource: string;
  timestampMs: number;
  nowMs: number;
  inferenceMs: number;
  pressedEnter: boolean;
  injected: boolean;
  failed: boolean;
  modified: boolean;
  destructive: boolean;
  sourceLabel: string;
}): HistoryCardModel {
  const text = input.cleanedText.trim() || '(No speech detected)';
  let deliveryLabel = 'Transcribed';
  if (input.injected) deliveryLabel = input.pressedEnter ? `Sent to ${input.appName || 'app'}` : `Pasted into ${input.appName || 'app'}`;
  else if (input.failed) deliveryLabel = 'Kept on clipboard';
  const words = wordCount(input.cleanedText);
  return {
    text,
    rawTitle: input.rawText,
    timeLabel: new Date(input.timestampMs).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }),
    relativeLabel: relativeTimeLabel(input.timestampMs, input.nowMs),
    dayGroup: historyDayGroup(input.timestampMs, input.nowMs),
    sourceLabel: input.sourceLabel,
    wordCount: words,
    wordLabel: words === 1 ? '1 word' : `${words} words`,
    inferenceLabel: `${input.inferenceMs} ms`,
    deliveryLabel,
    cleaned: input.modified,
    destructive: input.destructive,
    searchText: `${input.cleanedText} ${input.rawText} ${input.appName} ${input.triggerSource}`.toLowerCase(),
  };
}

export function highlightMatch(text: string, query: string): string {
  const needle = query.trim();
  if (!needle || needle.startsWith('app:')) return escapeText(text);
  const lower = text.toLowerCase();
  const index = lower.indexOf(needle.toLowerCase());
  if (index < 0) return escapeText(text);
  return `${escapeText(text.slice(0, index))}<mark>${escapeText(text.slice(index, index + needle.length))}</mark>${escapeText(text.slice(index + needle.length))}`;
}

function escapeText(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

export type HistoryRow =
  | { kind: 'day'; label: string }
  | { kind: 'card'; index: number };

/** Insert a day heading only when the group changes, in the order cards are shown. */
export function historyRows(groups: string[]): HistoryRow[] {
  const rows: HistoryRow[] = [];
  let last = '';
  groups.forEach((group, index) => {
    const label = group || 'Earlier';
    if (label !== last) {
      rows.push({ kind: 'day', label });
      last = label;
    }
    rows.push({ kind: 'card', index });
  });
  return rows;
}

export function emptyHistoryCopy(): { title: string; body: string } {
  return {
    title: 'Nothing said yet',
    body: 'Look at the screen and talk. Finished lines land here for 7 days.',
  };
}
