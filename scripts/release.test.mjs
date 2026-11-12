import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, copyFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));

test('release tags must match the app version', () => {
  const result = spawnSync(process.execPath, [join(root, 'scripts/check-release.mjs')], {
    env: { ...process.env, GITHUB_REF_TYPE: 'tag', GITHUB_REF_NAME: 'v0.0.0-invalid' },
    encoding: 'utf8',
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Release tag must match/);
});

for (const [name, response, expected] of [
  ['HTTP failure', '{ ok: false, status: 503 }', /HTTP 503/],
  ['corrupted model', '{ ok: true, arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer }', /checksum mismatch/],
]) {
  test(`asset preparation rejects ${name}`, async () => {
    const dir = await mkdtemp(join(tmpdir(), 'saywhen-asset-test-'));
    try {
      await mkdir(join(dir, 'scripts'));
      const script = join(dir, 'scripts/prepare-assets.mjs');
      await copyFile(join(root, 'scripts/prepare-assets.mjs'), script);
      const code = `globalThis.fetch = async () => (${response}); await import(${JSON.stringify(pathToFileURL(script).href)});`;
      const result = spawnSync(process.execPath, ['--input-type=module', '-e', code], { encoding: 'utf8' });
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, expected);
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });
}
