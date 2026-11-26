import assert from 'node:assert/strict';
import test from 'node:test';
import { importTypeScript } from './load-ts.mjs';

const math = await importTypeScript('src/vision-math.ts');
const text = await importTypeScript('src/text-format.ts');

test('a nod is a pitch swing and a shake is a yaw swing', () => {
  const nod = math.classifyHeadGesture([0, 0, 0, 0, 0, 0], [0, 8, 16, 4, 12, 0]);
  assert.equal(nod, 'nod');
  const shake = math.classifyHeadGesture([0, 12, 24, 6, 20, 2], [0, 1, 0, 1, 0, 1]);
  assert.equal(shake, 'shake');
  assert.equal(math.classifyHeadGesture([0, 1], [0, 1]), 'none');
});

test('mouth ratio grows when the lips open and head pose reads the matrix', () => {
  const landmarks = Array.from({ length: 400 }, () => ({ x: 0.5, y: 0.5 }));
  landmarks[13] = { x: 0.5, y: 0.48 };
  landmarks[14] = { x: 0.5, y: 0.52 };
  landmarks[82] = { x: 0.46, y: 0.49 };
  landmarks[87] = { x: 0.46, y: 0.51 };
  landmarks[312] = { x: 0.54, y: 0.49 };
  landmarks[317] = { x: 0.54, y: 0.51 };
  landmarks[78] = { x: 0.4, y: 0.5 };
  landmarks[308] = { x: 0.6, y: 0.5 };
  const open = math.computeMar(landmarks, 0);
  assert.ok(open > 0.05);
  const pose = math.extractHeadPose([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0]);
  assert.ok(Math.abs(pose.yaw) < 0.001);
  assert.ok(Math.abs(pose.pitch) < 0.001);
  assert.equal(math.medianValue([3, 1, 2]), 2);
});

test('text helpers escape html and keep shortcut labels', () => {
  assert.equal(text.escapeHtml(`<b class="x">`), '&lt;b class=&quot;x&quot;&gt;');
  assert.equal(text.formatAccelerator('Alt+Space', true), '⌥ + Space');
  assert.equal(text.formatDurationMs(65000), '01:05.0');
  assert.deepEqual(text.splitAllowlist('Cursor, cursor, Terminal'), ['Cursor', 'Terminal']);
  assert.equal(
    text.acceleratorFromCodes({ metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, code: 'KeyK' }),
    'Cmd+KeyK',
  );
  assert.equal(text.formatTriggerSource('hands_free_multimodal'), 'Hands-free');
});
