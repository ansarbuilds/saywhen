import assert from 'node:assert/strict';
import test from 'node:test';
import { chooseVisionTickMs, visionInferencesPerMinute } from '../src/vision-budget.ts';
import { parseProfileLines, parseSnippetLines } from '../src/product-settings.ts';

test('low power facing the screen asks for 47 percent fewer frames', () => {
  const active = visionInferencesPerMinute(chooseVisionTickMs('auto', true, false, true, true, 0));
  const low = visionInferencesPerMinute(chooseVisionTickMs('low', true, false, true, true, 0));
  assert.equal(active, 909);
  assert.equal(low, 480);
  const reduction = ((active - low) / active) * 100;
  assert.ok(Math.abs(reduction - 47.2) < 0.15);
  assert.equal(chooseVisionTickMs('low', true, true, true, true, 0), 66);
});

test('snippet and profile lines keep the first match', () => {
  const snippets = parseSnippetLines('my email = ada@example.com\nmy email = other\npr = pull request');
  assert.equal(snippets.length, 2);
  assert.equal(snippets[0].expansion, 'ada@example.com');
  const profiles = parseProfileLines('Cursor | phrase | MediaPipe\nSlack | paste only | | en');
  assert.equal(profiles[0].sendMode, 'phrase');
  assert.equal(profiles[1].sendMode, 'paste_only');
  assert.equal(profiles[1].language, 'en');
});
