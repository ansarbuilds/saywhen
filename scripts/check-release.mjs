import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const pkg = JSON.parse(read('package.json'));
const lock = JSON.parse(read('package-lock.json'));
const config = JSON.parse(read('src-tauri/tauri.conf.json'));
const cargoLock = read('src-tauri/Cargo.lock').match(/\[\[package\]\]\nname = "saywhen"\nversion = "([^"]+)"/)?.[1];
const cargo = read('src-tauri/Cargo.toml').match(/^version = "([^"]+)"/m)?.[1];
for (const version of [lock.version, lock.packages[''].version, config.version, cargo, cargoLock]) {
  assert.equal(version, pkg.version, 'Release versions must agree across npm, Cargo and Tauri');
}
if (process.env.GITHUB_REF_TYPE === 'tag') {
  assert.equal(process.env.GITHUB_REF_NAME, `v${pkg.version}`, 'Release tag must match the app version');
}
console.log(`Release metadata verified: v${pkg.version}`);
