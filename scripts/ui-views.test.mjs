import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import { importTypeScript } from './load-ts.mjs';

const history = await importTypeScript('src/ui/history-view.ts');
const views = await importTypeScript('src/ui/product-views.ts');
const icons = await importTypeScript('src/design/icons.ts');
const tray = await importTypeScript('src/ui/tray-copy.ts');

const now = new Date(2026, 10, 26, 15, 0, 0).getTime();
const yesterday = new Date(2026, 10, 25, 12, 0, 0).getTime();

test('history cards group by day and count words', () => {
  assert.equal(history.historyDayGroup(now - 60_000, now), 'Today');
  assert.equal(history.historyDayGroup(yesterday, now), 'Yesterday');
  assert.equal(history.relativeTimeLabel(now - 5 * 60_000, now), '5m ago');
  const card = history.historyCardModel({
    cleanedText: 'cargo test --release',
    rawText: 'cargo test dash dash release',
    appName: 'Terminal',
    triggerSource: 'hands_free_multimodal',
    timestampMs: now - 5 * 60_000,
    nowMs: now,
    inferenceMs: 120,
    pressedEnter: true,
    injected: true,
    failed: false,
    modified: true,
    destructive: false,
    sourceLabel: 'Hands-free',
  });
  assert.equal(card.dayGroup, 'Today');
  assert.equal(card.wordCount, 3);
  assert.equal(card.wordLabel, '3 words');
  assert.equal(card.deliveryLabel, 'Sent to Terminal');
  assert.equal(card.relativeLabel, '5m ago');
  assert.ok(history.highlightMatch('cargo test', 'test').includes('<mark>test</mark>'));
  const rows = history.historyRows(['Today', 'Today', 'Yesterday']);
  assert.deepEqual(rows.map((row) => (row.kind === 'day' ? row.label : row.index)), ['Today', 0, 1, 'Yesterday', 2]);
});

test('license, onboarding, quiet hours, and hints are specific', () => {
  const trial = views.licenseView({ tier: 'trial', trialDaysLeft: 6, checker: 'none', message: '' });
  assert.equal(trial.headline, '6 days left in the trial.');
  const pro = views.licenseView({ tier: 'pro', trialDaysLeft: 0, checker: 'server', message: 'Confirmed.' });
  assert.equal(pro.headline, 'You are on Pro.');
  assert.equal(views.onboardingProgress(2, 4), '2 of 4 ready');
  assert.equal(views.hudHint('PREVIEW'), 'Check');
  assert.equal(views.ONBOARDING_STEPS.length, 4);
  assert.match(views.quietHoursSummary(true, '22:00', '07:00'), /22:00/);
  assert.equal(views.quietHoursSummary(false, '22:00', '07:00').includes('off'), true);
});

test('the icon set names ten icons and renders svg', () => {
  assert.equal(icons.ICON_NAMES.length, 10);
  assert.equal(icons.isIconName('shield'), true);
  assert.equal(icons.isIconName('nope'), false);
  assert.match(icons.iconSvg('mic'), /<svg/);
  assert.equal(views.densityMode('compact'), 'compact');
  assert.equal(views.densityMode('roomy'), 'comfortable');
});

test('menu bar copy matches the rust tray labels', () => {
  const rust = fs.readFileSync('src-tauri/src/tray_menu.rs', 'utf8');
  assert.equal(tray.TRAY_ITEMS.length, 6);
  for (const item of tray.TRAY_ITEMS) {
    assert.match(rust, new RegExp(`id: "${item.id}"`));
    assert.match(rust, new RegExp(`label: "${item.label}"`));
  }
});
