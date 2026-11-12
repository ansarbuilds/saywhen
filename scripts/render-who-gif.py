#!/usr/bin/env python3
"""Build the Who it's for loop: home, office, then the 2025 work-setting bar.

Words are drawn here, not baked into the source illustrations. The survey
figures match the Stack Overflow 2025 work question and the JetBrains 2025
line used in the README. Timing is a design choice, not a measurement.
"""

from __future__ import annotations

import math
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
HOME = ROOT / "docs/visuals/home-day.jpg"
OFFICE = ROOT / "docs/visuals/home-and-office.jpg"
OUT_GIF = ROOT / "docs/visuals/who-its-for.gif"
OUT_PNG = ROOT / "docs/visuals/who-its-for.jpg"
FRAMES = Path("/tmp/saywhen-who-frames")

W, H = 960, 540
FPS = 10
PLUM = (34, 6, 28)
AMBER = (251, 179, 3)
SILVER = (223, 229, 236)
CREAM = (236, 228, 214)
MUTED = (179, 168, 180)
HYBRID = (228, 192, 122)
OFFICE_SEG = (109, 90, 104)

SERIF = "/System/Library/Fonts/Supplemental/Georgia.ttf"
SERIF_BOLD = "/System/Library/Fonts/Supplemental/Georgia Bold.ttf"
SANS = "/System/Library/Fonts/Supplemental/Arial.ttf"
SANS_BOLD = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"

SEGMENTS = (
    (0.324, AMBER, "32.4%", "Fully remote"),
    (0.371, HYBRID, "37.1%", "Hybrid"),
    (0.126, SILVER, "12.6%", "Free to choose"),
    (0.179, OFFICE_SEG, "17.9%", "In an office"),
)


def font(path: str, size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(path, size)


def ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 1 - (1 - t) ** 3


def cover(im: Image.Image, zoom: float, focus: tuple[float, float]) -> Image.Image:
    im = im.convert("RGB")
    scale = max((W * zoom) / im.width, (H * zoom) / im.height)
    resized = im.resize((max(W, int(im.width * scale)), max(H, int(im.height * scale))), Image.Resampling.LANCZOS)
    cx = int(resized.width * focus[0])
    cy = int(resized.height * focus[1])
    left = max(0, min(resized.width - W, cx - W // 2))
    top = max(0, min(resized.height - H, cy - H // 2))
    return resized.crop((left, top, left + W, top + H))


def scrim() -> Image.Image:
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    px = layer.load()
    start = int(H * 0.38)
    for y in range(start, H):
        t = (y - start) / (H - start)
        alpha = int(210 * (t ** 1.35))
        for x in range(0, W, 2):
            px[x, y] = (*PLUM, alpha)
            if x + 1 < W:
                px[x + 1, y] = (*PLUM, alpha)
    return layer


def text_size(draw: ImageDraw.ImageDraw, text: str, face: ImageFont.FreeTypeFont) -> tuple[int, int]:
    box = draw.textbbox((0, 0), text, font=face)
    return box[2] - box[0], box[3] - box[1]


def paint_text(base: Image.Image, lines: list[tuple[str, ImageFont.FreeTypeFont, tuple[int, int, int], int]], alpha: float, rise: int) -> None:
    if alpha <= 0:
        return
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    for text, face, color, y in lines:
        tw, _ = text_size(draw, text, face)
        x = (W - tw) // 2
        yy = y - int((1 - alpha) * rise)
        draw.text((x + 2, yy + 3), text, font=face, fill=(*PLUM, int(160 * alpha)))
        draw.text((x, yy), text, font=face, fill=(*color, int(255 * alpha)))
    base.alpha_composite(layer)


def scene_frame(photo: Image.Image, zoom: float, focus: tuple[float, float], lines: list, alpha: float) -> Image.Image:
    frame = cover(photo, zoom, focus).convert("RGBA")
    frame.alpha_composite(SCRIM)
    paint_text(frame, lines, alpha, 18)
    return frame


def stats_frame(progress: float, alpha: float) -> Image.Image:
    frame = Image.new("RGBA", (W, H), (*PLUM, 255))
    draw = ImageDraw.Draw(frame)
    kicker = font(SANS_BOLD, 18)
    title = font(SERIF, 54)
    source = font(SANS, 18)
    label_num = font(SANS_BOLD, 22)
    label_name = font(SANS, 16)
    lines = [
        ("STACK OVERFLOW  2025", kicker, AMBER, 78),
        ("Where developers worked", title, CREAM, 118),
    ]
    paint_text(frame, lines, alpha, 12)

    bar_w, bar_h = 760, 22
    bar_x = (W - bar_w) // 2
    bar_y = 250
    shown = ease(progress)
    draw.rounded_rectangle((bar_x, bar_y, bar_x + bar_w, bar_y + bar_h), radius=11, fill=(58, 32, 52, 255))
    x = bar_x
    for share, color, _, _ in SEGMENTS:
        width = bar_w * share * shown
        if width > 1:
            draw.rectangle((x, bar_y, x + width, bar_y + bar_h), fill=color)
        x += bar_w * share * shown

    if progress > 0.55:
        label_alpha = ease((progress - 0.55) / 0.45)
        cursor = bar_x
        for share, _, num, name in SEGMENTS:
            cx = cursor + (bar_w * share) / 2
            cursor += bar_w * share
            for text, face, color, dy in (
                (num, label_num, CREAM, 292),
                (name, label_name, MUTED, 322),
            ):
                tw, _ = text_size(draw, text, face)
                overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
                ImageDraw.Draw(overlay).text((cx - tw / 2, dy), text, font=face, fill=(*color, int(255 * label_alpha * alpha)))
                frame.alpha_composite(overlay)

    foot = "Not a measure of who wants voice.  US fully remote: 45%."
    foot2 = "JetBrains 2025:  62% regularly use an AI coding assistant."
    paint_text(
        frame,
        [
            (foot, source, MUTED, 430),
            (foot2, source, SILVER, 462),
        ],
        alpha * ease(max(0, (progress - 0.72) / 0.28)),
        8,
    )
    return frame


def blend(a: Image.Image, b: Image.Image, t: float) -> Image.Image:
    return Image.blend(a.convert("RGBA"), b.convert("RGBA"), ease(t))


SCRIM = scrim()


def main() -> None:
    home = Image.open(HOME)
    office_sheet = Image.open(OFFICE)
    office = office_sheet.crop((office_sheet.width // 2, 0, office_sheet.width, office_sheet.height))

    home_lines = [
        ("HOME DAY", font(SANS_BOLD, 20), AMBER, 268),
        ("Look. Talk.", font(SERIF, 72), CREAM, 304),
        ("Hands off the keyboard.", font(SANS, 26), SILVER, 400),
    ]
    office_lines = [
        ("OFFICE DAY", font(SANS_BOLD, 20), AMBER, 268),
        ("Stay quiet.", font(SERIF, 72), CREAM, 304),
        ("Type only. The shortcut is still there.", font(SANS, 24), SILVER, 400),
    ]

    home_a = scene_frame(home, 1.0, (0.42, 0.48), home_lines, 1)
    home_b = scene_frame(home, 1.08, (0.40, 0.46), home_lines, 1)
    office_a = scene_frame(office, 1.0, (0.48, 0.58), office_lines, 1)
    office_b = scene_frame(office, 1.07, (0.46, 0.56), office_lines, 1)
    card_in = stats_frame(0.02, 1)

    FRAMES.mkdir(parents=True, exist_ok=True)
    for old in FRAMES.glob("*.png"):
        old.unlink()

    sequence: list[Image.Image] = []

    def add_move(start: Image.Image, end: Image.Image, n: int, text_in: int = 8) -> None:
        for i in range(n):
            t = i / max(1, n - 1)
            frame = blend(start, end, t)
            if i < text_in:
                # The pre-rendered scenes already include text. Fade the whole
                # frame up from a dimmed copy so the first beat does not pop.
                dim = Image.blend(Image.new("RGBA", (W, H), (*PLUM, 255)), frame, ease((i + 1) / text_in))
                sequence.append(dim.convert("RGB"))
            else:
                sequence.append(frame.convert("RGB"))

    def add_cross(a: Image.Image, b: Image.Image, n: int) -> None:
        for i in range(n):
            sequence.append(blend(a, b, (i + 1) / n).convert("RGB"))

    add_move(home_a, home_b, 28, text_in=8)
    add_cross(home_b, office_a, 8)
    add_move(office_a, office_b, 26, text_in=0)
    add_cross(office_b, card_in, 8)
    for i in range(36):
        sequence.append(stats_frame((i + 1) / 18 if i < 18 else 1, 1).convert("RGB"))

    for i, frame in enumerate(sequence):
        frame.save(FRAMES / f"frame_{i:04d}.png", optimize=True)

    sequence[20].convert("RGB").save(OUT_PNG, "JPEG", quality=82, optimize=True)
    palette = FRAMES / "palette.png"
    shrink = "fps=6,scale=640:-1:flags=lanczos"
    subprocess.run(
        [
            "ffmpeg", "-y", "-framerate", str(FPS), "-i", str(FRAMES / "frame_%04d.png"),
            "-vf", f"{shrink},palettegen=max_colors=48:stats_mode=diff",
            str(palette),
        ],
        check=True,
        capture_output=True,
    )
    subprocess.run(
        [
            "ffmpeg", "-y", "-framerate", str(FPS), "-i", str(FRAMES / "frame_%04d.png"),
            "-i", str(palette),
            "-lavfi", f"{shrink}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle",
            "-loop", "0", str(OUT_GIF),
        ],
        check=True,
        capture_output=True,
    )
    print(f"{len(sequence)} frames, {OUT_GIF.stat().st_size / 1024:.0f} KB, {len(sequence) / FPS:.1f}s")


if __name__ == "__main__":
    main()
