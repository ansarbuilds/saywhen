# Launch checklist

Status: beta preparation complete locally; public launch sign-off is pending.

## Verified in this checkout

- Frontend production build passes and includes the camera model and MediaPipe WASM runtime.
- macOS Apple Silicon debug `.app` packaging succeeds. This is not a signed production installer test.
- 46 automated Rust tests pass, including model format validation and the model checksum check; 3 desktop/GPU tests are explicitly ignored by default.
- 3 Node release tests pass: wrong release tags, failed model requests and corrupt camera-model responses are rejected.
- Release metadata versions agree. CI and release packaging now run these checks before bundling.
- Camera asset preparation validates the pinned face model's SHA-256 and copies WASM from the installed npm version. Cached asset preparation passes.
- A fresh download was attempted but blocked by this session's DNS/network restrictions (`ENOTFOUND storage.googleapis.com`). It must pass on a clean CI runner before release.

## Required before publishing

Record the version, OS version, hardware and result for each applicable platform. Do not infer Windows or Intel Mac results from Apple Silicon.

| Gate | Apple Silicon | Intel Mac | Windows x64 |
| --- | --- | --- | --- |
| Clean checkout: `npm ci`, `npm test`, `npm run check:release`, `npm run build` | Local pass on 2026-11-12 (macOS 27.0.1, M4, existing checkout); CI pending | Pending CI | Pending CI |
| Locked Rust tests pass | Local pass on 2026-11-12 (46 passed, 3 ignored) | Pending | Pending |
| Release installer builds and opens on a fresh account | Pending | Pending | Pending |
| Camera, microphone and typing permissions: deny, retry, grant | Pending | Pending | Pending |
| Download Tiny, interrupt the download, retry, restart | Pending | Pending | Pending |
| Disconnect internet after setup: camera and dictation work | Pending | Pending | Pending |
| Type only, pause, nod and phrase modes behave as labeled | Pending | Pending | Pending |
| Esc, shake and speech cancel before injection | Pending | Pending | Pending |
| Switch app during preview: no delivery to the new app | Pending | Pending | Pending |
| Switch app after paste: no Enter in the new app | Pending | Pending | Pending |
| Risky commands are downgraded; no unintended execution | Pending | Pending | Pending |
| Hide window for 10 minutes: gate remains active | Pending | Pending | Pending |
| Sleep/wake, mic unplug/reconnect: no stale send | Pending | Pending | Pending |
| Full work session: record unwanted sends and missed starts | Pending | Pending | Pending |
| Signing and installer security prompts documented | Pending | Pending | Pending |

Use a disposable terminal with harmless commands for delivery tests. The destructive-command matcher is a heuristic, not a shell parser or a guarantee against dangerous commands. Test risky strings in an editor, never a live shell. Paste itself can execute multiline input in some terminals.

Run hardware-dependent tests explicitly from an interactive desktop, after downloading Tiny:

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml -- --ignored --test-threads=1
```

The GPU benchmark crashed inside Metal allocation in the restricted agent session. It remains a required hardware check; excluding it from unattended unit tests is not a fix or proof of runtime compatibility. The clipboard test temporarily replaces clipboard contents, so run it when the clipboard can be used for testing.

## Prepare the release

1. Complete the gates for every platform being advertised. Omit unverified platform installers from a public release, or label them explicitly experimental.
2. Choose an unused version and update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` and `src-tauri/tauri.conf.json` together. Run `npm run check:release`.
3. Configure the Apple signing and notarization secrets listed in `.github/workflows/release.yml`. Verify the final downloaded app with `codesign --verify --deep --strict` and `spctl --assess --type execute`. Windows signing still needs separate configuration or a clearly disclosed unsigned beta.
4. Push reviewed changes and a matching `v<version>` tag. The workflow creates a **draft prerelease**, not a public release. All intended matrix jobs must pass; a successful platform alone is not enough.
5. Download and test the actual installers. Add release notes with supported OS/hardware, signing status, first-run instructions, known limitations and the exact test evidence.
6. Check the README demo links and attach the referenced video assets. The README clip links to the project page at `https://ansarbuilds.dev/p/saywhen/`, so that page must be live before the repository goes public. Publish the draft only after the gates pass. Mark it stable only after the hardware evidence supports that claim.

## Remaining hardening

- Speech downloads use HTTPS, bounded retries, a temporary file and GGML format/size sanity checks. Since 2026-11-12 they are also pinned to one revision of the model repository and to SHA-256 digests. Both digests were checked against real downloads of Tiny and Base. The v0.2.0 installers predate this, so it ships with the next build.
- The current webview CSP is disabled. A restrictive policy needs testing with MediaPipe WASM, blob workers and Tauri IPC before enabling it.
- Review optional remote cleanup and API-key storage before advertising it for sensitive work.
- Real-user accuracy, CPU and battery results are still needed. Existing replay numbers are synthetic-corpus results.
