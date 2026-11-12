# Documentation visuals

These assets use the app's plum, amber and pale neutral palette. They are locally embedded PNGs and a GIF, with no external image-host dependency.

| File | Purpose |
| --- | --- |
| `brand-hero.png` | AI-generated silver-and-amber editorial brand artwork |
| `brand-features.png` | AI-generated intent, voice and control feature illustration |
| `readme-hero.png` | Project introduction and three-step concept |
| `workflow.gif` | 12-second illustrated pause-mode loop, approximately 1.1 MB |
| `workflow-still.png` | Non-animated alternative showing the check step |
| `intent-and-privacy.png` | Camera and microphone gate with explicit optional-path caveats |
| `send-styles.png` | Four send modes and shared policy constraints |

The GIF is an explanatory animation, not an app recording. Its timing must not be presented as measured transcription latency. Existing screenshots in `../screenshots/` are browser demo captures, not proof of native permission or injection behavior.

## The film in the main README

The player at the top of the main README is a GitHub attachment, not a file in this repository. GitHub only plays videos that were uploaded through its web editor, and the free limit is 10 MB. To replace it, encode the film under that size, drag the file into any issue or comment box on github.com (no need to submit), and paste the `https://github.com/user-attachments/assets/...` link it gives you on a line of its own in the README.

```bash
ffmpeg -i film/out/saywhen-demo.mp4 -c:v libx264 -preset slow -crf 26 -pix_fmt yuv420p -c:a aac -b:a 128k -movflags +faststart film/out/saywhen-demo-github.mp4
```

## Regenerate

Install Pillow in your chosen Python environment, then run from the repository root:

```bash
python3 scripts/render-doc-visuals.py
```

The two `brand-*.png` files are generated brand artwork, not product screenshots or technical diagrams. They are maintained separately and are not overwritten by the renderer.

The renderer uses Arial/Menlo on macOS or DejaVu Sans/Mono on Linux. Font metrics can differ slightly; inspect the resulting images before committing. Images are rendered from text and geometry, without camera or microphone data.

Keep alternate text and nearby prose in sync when changing a visual. Keep animation slow, avoid flashes and provide a static alternative. Do not replace product screenshots with unlabeled illustrative interfaces.
