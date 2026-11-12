# What has been measured

These numbers come from prototype sessions on an Apple Silicon Mac. The films and animations in this repository are illustrations, and their timing is not a measurement. The numbers below are.

| Part | What we measured | Result |
| --- | --- | --- |
| Face tracking (MediaPipe) | Time per camera frame | 12 to 15 ms |
| "Facing the screen" check | Telling facing from turned away | 100% on our test sessions |
| "Mouth still" check | Ignoring a closed or still mouth | 96.2% rejected correctly |
| Start and end of speech | 56 test clips (generated speech plus simulated camera traces, not live users) | 0% false starts (was 36.4%), 0% cut-offs (was 52.9%) |
| Developer word clean-up | 34 spoken commands | 34 of 34 exact (was 44.1%) |
| Speech to text (Whisper tiny.en, GPU) | 1.5 s of audio | 87 to 473 ms |
| Finding the front app (macOS) | Median of 20 calls | 1 to 2 microseconds |

The start-and-end-of-speech clips are generated speech with simulated camera traces, not live users. Real-user accuracy, CPU and battery use are not measured yet. The [launch checklist](LAUNCH_CHECKLIST.md) lists what has been checked on which hardware.

Run the replay benchmark yourself:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin replay -- recordings
```
