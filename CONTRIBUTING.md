# Contributing to saywhen

Thanks for wanting to help. Bug reports, tuning data and small fixes are all welcome.

## Before you start

- For anything bigger than a small fix, open an issue first so we can agree on the approach.
- Read [AGENTS.md](AGENTS.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). The principles there are not optional: local-first, private, always cancelable, fail loudly, measure everything.

## Set up

You need Node.js 20 or newer, Rust and CMake (see the README). The Rust version is pinned in `rust-toolchain.toml`, and rustup installs it for you. Please be kind and patient with each other: see the [Code of Conduct](CODE_OF_CONDUCT.md).

```bash
npm install
```

```bash
npx tauri dev
```

## Before you open a pull request

```bash
npm run build
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

If you changed when saywhen starts or stops listening, also run the replay benchmark and paste the before and after numbers into the pull request:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin replay -- --generate-corpus recordings
```

## Rules of the road

- **Privacy first.** Never write camera frames to disk, send them anywhere or log them. Audio is only kept when the user opts in.
- **Never commit** model files (`*.bin`, `*.gguf`, `*.onnx`), recordings or `settings.local.json`.
- **Numbers, not vibes.** Claims about accuracy, speed, CPU or battery need a measurement. Say what you verified and what you assumed.
- **No em dashes** in docs or code comments. CI checks this.
- **Plain language** in anything a user reads.
- Keep pull requests small and focused on one thing.

## Reporting a bug

Use the bug report template. The **Problems** panel on the Home screen has a copy button for the error message. Please do not attach recordings of your voice or screenshots that show private text.
