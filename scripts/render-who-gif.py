#!/usr/bin/env python3
"""Build the Who it's for loop.

The illustrations stay whole. Words sit in a band under the picture, then a
still card shows the 2025 Stack Overflow work split. Holds are exact copies
so the GIF stays sharp. Timing is a design choice, not a measurement.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
HOME = ROOT / "docs/visuals/home-day.jpg"
SPLIT = ROOT / "docs/visuals/home-and-office.jpg"
OUT_GIF = ROOT / "docs/visuals/who-its-for.gif"
OUT_JPG = ROOT / "docs/visuals/who-its-for.jpg"
WORK = Path("/tmp/saywhen-who-v2")

W = 960
PIC_H = 540
BAND = 112
H = PIC_H + BAND

PLUM = (34, 6, 28)
AMBER = (251, 179, 3)
SILVER = (223, 229, 236)
CREAM = (243, 236, 226)
MUTED = (186, 174, 186)
HYBRID = (228, 192, 122)
FLEX = (223, 229, 236)
OFFICE = (122, 104, 118)

SERIF = "/System/Library/Fonts/Supplemental/Georgia.ttf"
SANS = "/System/Library/Fonts/Supplemental/Arial.ttf"
SANS_BOLD = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"

SEGMENTS = (
    (0.324, AMBER, "32.4%", "Fully remote"),
    (0.371, HYBRID, "37.1%", "Hybrid"),
    (0.126, FLEX, "12.6%", "Free to choose"),
    (0.179, OFFICE, "17.9%", "In an office"),
)


def face(path: str, size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(path, size)


def picture(path: Path) -> Image.Image:
    im = Image.open(path).convert("RGB")
    return im.resize((W, PIC_H), Image.Resampling.LANCZOS)


def text_width(draw: ImageDraw.ImageDraw, text: str, fnt: ImageFont.FreeTypeFont) -> int:
    box = draw.textbbox((0, 0), text, font=fnt)
    return box[2] - box[0]


def split_card(photo: Image.Image) -> Image.Image:
    card = Image.new("RGB", (W, H), PLUM)
    card.paste(photo, (0, 0))
    draw = ImageDraw.Draw(card)
    draw.rectangle((0, PIC_H, W, PIC_H + 3), fill=AMBER)
    kicker = face(SANS_BOLD, 16)
    headline = face(SERIF, 32)
    draw.text((36, PIC_H + 24), "HOME", font=kicker, fill=AMBER)
    draw.text((36, PIC_H + 52), "Look. Talk.", font=headline, fill=CREAM)
    right = W // 2 + 28
    draw.text((right, PIC_H + 24), "OFFICE", font=kicker, fill=SILVER)
    draw.text((right, PIC_H + 52), "Stay quiet.", font=headline, fill=CREAM)
    draw.line((W // 2, PIC_H + 22, W // 2, H - 22), fill=(90, 62, 84), width=1)
    return card


def chart(progress: float) -> Image.Image:
    card = Image.new("RGB", (W, H), PLUM)
    draw = ImageDraw.Draw(card)
    kicker = face(SANS_BOLD, 18)
    title = face(SERIF, 46)
    num_face = face(SANS_BOLD, 22)
    name_face = face(SANS, 15)
    foot = face(SANS, 16)

    label = "STACK OVERFLOW  2025"
    draw.text(((W - text_width(draw, label, kicker)) // 2, 150), label, font=kicker, fill=AMBER)
    heading = "Where developers worked"
    draw.text(((W - text_width(draw, heading, title)) // 2, 190), heading, font=title, fill=CREAM)

    bar_w, bar_h = 760, 20
    bar_x = (W - bar_w) // 2
    bar_y = 300
    draw.rounded_rectangle((bar_x, bar_y, bar_x + bar_w, bar_y + bar_h), radius=10, fill=(58, 36, 52))
    shown = max(0.0, min(1.0, progress))
    x = bar_x
    for share, color, _, _ in SEGMENTS:
        width = bar_w * share * shown
        if width > 0.5:
            draw.rectangle((x, bar_y, x + width, bar_y + bar_h), fill=color)
        x += width

    if progress >= 1:
        col = bar_w / 4
        for i, (color, num, name) in enumerate((item[1], item[2], item[3]) for item in SEGMENTS):
            cx = bar_x + col * i + col / 2
            num_w = text_width(draw, num, num_face)
            name_w = text_width(draw, name, name_face)
            draw.rectangle((cx - 16, 342, cx + 16, 348), fill=color)
            draw.text((cx - num_w / 2, 360), num, font=num_face, fill=CREAM)
            draw.text((cx - name_w / 2, 390), name, font=name_face, fill=MUTED)
        note = "Not a measure of who wants voice.   US fully remote: 45%."
        note2 = "JetBrains 2025:  62% regularly use an AI coding assistant."
        draw.text(((W - text_width(draw, note, foot)) // 2, 470), note, font=foot, fill=MUTED)
        draw.text(((W - text_width(draw, note2, foot)) // 2, 498), note2, font=foot, fill=SILVER)
    return card


def main() -> None:
    WORK.mkdir(parents=True, exist_ok=True)
    home = _with_photo(picture(HOME), "HOME DAY", "Look. Talk.")
    split = split_card(picture(SPLIT))
    home.save(OUT_JPG, "JPEG", quality=86, optimize=True)

    frames = [("home.png", home, 2.4), ("split.png", split, 2.6)]
    for i, step in enumerate((0.18, 0.36, 0.55, 0.74, 0.9, 1, 1)):
        frames.append((f"bar_{i}.png", chart(step), 0.12 if i < 5 else 2.2))

    for name, image, _duration in frames:
        image.save(WORK / name)

    listing = WORK / "list.txt"
    lines = []
    for name, _image, duration in frames:
        lines.append(f"file '{name}'")
        lines.append(f"duration {duration}")
    lines.append(f"file '{frames[-1][0]}'")
    listing.write_text("\n".join(lines) + "\n")

    palette = WORK / "palette.png"
    subprocess.run(
        ["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(listing),
         "-vf", "palettegen=max_colors=128:stats_mode=diff", str(palette)],
        cwd=WORK, check=True, capture_output=True,
    )
    subprocess.run(
        ["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(listing),
         "-i", str(palette),
         "-lavfi", "paletteuse=dither=none",
         "-loop", "0", str(OUT_GIF)],
        cwd=WORK, check=True, capture_output=True,
    )
    print(f"{OUT_GIF.stat().st_size / 1024:.0f} KB")


def _with_photo(photo: Image.Image, title: str, line: str) -> Image.Image:
    card = Image.new("RGB", (W, H), PLUM)
    card.paste(photo, (0, 0))
    draw = ImageDraw.Draw(card)
    draw.rectangle((0, PIC_H, W, PIC_H + 3), fill=AMBER)
    draw.text((40, PIC_H + 22), title, font=face(SANS_BOLD, 18), fill=AMBER)
    draw.text((40, PIC_H + 50), line, font=face(SERIF, 42), fill=CREAM)
    return card


if __name__ == "__main__":
    main()
