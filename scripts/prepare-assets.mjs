// Ship camera assets with the app so installed builds need no vision network calls.
import { cp, mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const modelDir = join(root, 'public/models');
const modelPath = join(modelDir, 'face_landmarker.task');
const sha256 = '64184e229b263107bc2b804c6625db1341ff2bb731874b0bcc2fe6544e0bc9ff';
const source = 'https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task';
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

await mkdir(modelDir, { recursive: true });
const cached = await readFile(modelPath).catch(() => null);
if (!cached || digest(cached) !== sha256) {
  console.log('Downloading the pinned MediaPipe face model (about 4 MB)...');
  const response = await fetch(source, { signal: AbortSignal.timeout(120_000) });
  if (!response.ok) throw new Error(`Face model download failed: HTTP ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  if (digest(bytes) !== sha256) throw new Error('Face model checksum mismatch. Build stopped.');
  const temporary = `${modelPath}.${process.pid}.part`;
  try {
    await writeFile(temporary, bytes);
    await rename(temporary, modelPath);
  } finally {
    await rm(temporary, { force: true });
  }
}
// Use the WASM runtime from the exact npm lockfile version, never a separate CDN version.
await cp(join(root, 'node_modules/@mediapipe/tasks-vision/wasm'), join(root, 'public/wasm'), { recursive: true });
console.log('Camera model verified and WASM runtime prepared.');
