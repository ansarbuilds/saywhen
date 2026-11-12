# Documentation visuals

These assets use the app's plum, amber and pale neutral palette. They are locally embedded PNGs and a GIF, with no external image-host dependency.

| File | Purpose |
| --- | --- |
| `readme-hero.png` | Project introduction and three-step concept |
| `workflow.gif` | 12-second illustrated pause-mode loop, approximately 1.1 MB |
| `workflow-still.png` | Non-animated alternative showing the check step |
| `intent-and-privacy.png` | Camera and microphone gate with explicit optional-path caveats |
| `send-styles.png` | Four send modes and shared policy constraints |

The GIF is an explanatory animation, not an app recording. Its timing must not be presented as measured transcription latency. Existing screenshots in `../screenshots/` are browser demo captures, not proof of native permission or injection behavior.

## Regenerate

Install Pillow in your chosen Python environment, then run from the repository root:

```bash
python3 scripts/render-doc-visuals.py
```

The renderer uses Arial/Menlo on macOS or DejaVu Sans/Mono on Linux. Font metrics can differ slightly; inspect the resulting images before committing. Images are rendered from text and geometry, without camera or microphone data.

Keep alternate text and nearby prose in sync when changing a visual. Keep animation slow, avoid flashes and provide a static alternative. Do not replace product screenshots with unlabeled illustrative interfaces.
