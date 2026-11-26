import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const ts = require('typescript');
const root = join(dirname(fileURLToPath(import.meta.url)), '..');

async function importTypeScript(relativePath) {
  const source = readFileSync(join(root, relativePath), 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: relativePath,
  }).outputText;
  const dir = mkdtempSync(join(tmpdir(), 'saywhen-ts-'));
  const file = join(dir, 'module.mjs');
  writeFileSync(file, compiled);
  return import(pathToFileURL(file).href);
}

const { chooseVisionTickMs, visionInferencesPerMinute } = await importTypeScript('src/vision-budget.ts');
const { parseProfileLines, parseSnippetLines } = await importTypeScript('src/product-settings.ts');

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
