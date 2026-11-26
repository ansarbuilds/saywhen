export interface Snippet {
  trigger: string;
  expansion: string;
}

export interface AppProfile {
  app: string;
  sendMode?: 'pause' | 'nod' | 'phrase' | 'paste_only';
  extraVocab: string;
  language?: string;
}

const SEND_MODES = new Set(['pause', 'nod', 'phrase', 'paste_only']);

export function parseSnippetLines(raw: string): Snippet[] {
  const snippets: Snippet[] = [];
  for (const line of raw.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const splitAt = trimmed.indexOf('=');
    if (splitAt <= 0) continue;
    const trigger = trimmed.slice(0, splitAt).trim().replace(/\s+/g, ' ');
    const expansion = trimmed.slice(splitAt + 1).trim();
    if (trigger.length < 2 || !expansion) continue;
    if (snippets.some((item) => item.trigger.toLowerCase() === trigger.toLowerCase())) continue;
    snippets.push({ trigger, expansion });
    if (snippets.length === 100) break;
  }
  return snippets;
}

export function formatSnippetLines(snippets: Snippet[] | undefined): string {
  return (snippets ?? []).map((item) => `${item.trigger} = ${item.expansion}`).join('\n');
}

export function parseProfileLines(raw: string): AppProfile[] {
  const profiles: AppProfile[] = [];
  for (const line of raw.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const parts = trimmed.split('|').map((part) => part.trim());
    const app = parts[0] ?? '';
    if (!app) continue;
    const sendRaw = (parts[1] ?? '').toLowerCase().replace(/[\s-]+/g, '_');
    const sendMode = SEND_MODES.has(sendRaw)
      ? (sendRaw as AppProfile['sendMode'])
      : undefined;
    const extraVocab = parts[2] ?? '';
    const language = (parts[3] ?? '').toLowerCase();
    if (profiles.some((item) => item.app.toLowerCase() === app.toLowerCase())) continue;
    profiles.push({
      app,
      sendMode,
      extraVocab,
      language: language || undefined,
    });
    if (profiles.length === 40) break;
  }
  return profiles;
}

export function formatProfileLines(profiles: AppProfile[] | undefined): string {
  return (profiles ?? [])
    .map((item) =>
      [item.app, item.sendMode ?? '', item.extraVocab ?? '', item.language ?? ''].join(' | '),
    )
    .join('\n');
}
