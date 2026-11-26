# What Mohamed has to do

The app can be built and tested without any of these. None of them belong in the git repository. Add them in GitHub under Settings, Secrets and variables, Actions, or on your own machines.

## Apple signing and notarization

Create a Developer ID Application certificate, export it as a `.p12`, and base64-encode that file. Then add:

| Secret | What it is |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 of the Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password you set when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_PASSWORD` | App-specific password for that Apple ID |
| `APPLE_TEAM_ID` | 10-character team id |

The release workflow already turns signing on only when `APPLE_CERTIFICATE` and `APPLE_SIGNING_IDENTITY` are present. Without them the macOS build stays unsigned and Gatekeeper warns. After a signed build, check the downloaded app with `codesign --verify --deep --strict` and `spctl --assess --type execute`.

## Windows signing

The release workflow passes these through when they exist. Without them the Windows installer stays unsigned and SmartScreen warns.

| Secret | What it is |
| --- | --- |
| `WINDOWS_CERTIFICATE` | Base64 of the Authenticode `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for that `.pfx` |

A cloud signing service (Azure Trusted Signing, SSL.com, and similar) is a separate account. The workflow does not talk to one until you add the certificate secret above or replace the step with that service's action.

## Updates

saywhen does not yet publish updater files. Before the first build that should update itself:

1. On your machine, run `npx tauri signer generate -w ~/saywhen-updater.key`.
2. Put the public key in `src-tauri/tauri.conf.json` under the updater plugin, and set `bundle.createUpdaterArtifacts` to `true`.
3. Add the private key as `TAURI_SIGNING_PRIVATE_KEY`. If the key has a password, add `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
4. Do not commit the private key. If you lose it, installed copies cannot trust a new key without an app update that contains the new public key.

The GitHub release workflow publishes a prerelease. Install that exact build on a clean Mac and a clean Windows PC before calling it stable. The [launch checklist](LAUNCH_CHECKLIST.md) is the list of those checks. Most of the hands-on rows are still open.

## Charging money

The in-app license screen is a local trial plus a place to paste a key. Enforcement is off, so nobody is locked out.

The development key format is checked with a secret that is printed in the source on purpose (`saywhen-dev-license-not-for-production-v1`). Anyone can make a development key. Do not sell those.

To charge:

1. Create the store (Lemon Squeezy, Paddle, or Stripe) in your name.
2. Run a small server you control. It receives `{ "key": "...", "app": "saywhen" }` and returns `{ "valid": true, "tier": "pro", "expiresAtMs": 123 }`.
3. Put that server's HTTPS address in Settings, License, License server.
4. Turn enforcement to Required only when you want snippets, per-app profiles, history export, cloud clean-up, and non-English models to wait for Pro. Hands-free dictation stays available either way.
5. Keep the store's API keys on that server. Do not put them in this repository or in the desktop app.

## Still yours to measure

CPU and battery with the camera on, a real work session of unwanted sends and missed starts, and a labeled file if you want the talking-to-the-screen trainer to mean anything. The trainer's `--smoke` run is a toy. See [training/talking-to-screen/README.md](../training/talking-to-screen/README.md).
