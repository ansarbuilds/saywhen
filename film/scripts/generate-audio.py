#!/usr/bin/env python3
"""
Builds film/public/demo-soundtrack.wav (60.0s, 44.1 kHz, 16-bit stereo)
by mixing:
1. Real studio music track from brag (happy-beats-business-moves-vol-12-by-ende-dot-app.mp3, CC BY 4.0 Sascha Ende)
   with automatic sidechain ducking under voiceover.
2. 12 Microsoft Azure Neural (en-US-AvaNeural) voiceover reads in film/public/vo/01.mp3..12.mp3.
3. Real CC0 sound effects from brag (Kenney.nl / OpenGameArt):
   - 32 mechanical keypress WAVs (keypress-001..032.wav) for ticker & terminal injection
   - impactSoft_medium_001..004.ogg for whip pans & iris transitions
   - impactBell_heavy_000 / 003.ogg for major reveals & end card lockup
   - interface/click_002..005.ogg, switch_007.ogg, bong_001.ogg, drop_001.ogg, casino/card-slide-1.ogg
"""

import math
import os
import random
import struct
import subprocess
import tempfile
import wave

SR = 44100
DURATION = 60.0
TOTAL_SAMPLES = int(SR * DURATION)
FPS = 30.0

FILM_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VO_DIR = os.path.join(FILM_DIR, "public", "vo")
SFX_DIR = os.path.join(FILM_DIR, "assets-src", "sfx")
MUSIC_DIR = os.path.join(FILM_DIR, "assets-src", "music")
OUT_WAV = os.path.join(FILM_DIR, "public", "demo-soundtrack.wav")

VOICE_CUES = [
    ("01", 14),
    ("02", 118),
    ("03", 230),
    ("04", 318),
    ("05", 500),
    ("06", 604),
    ("07", 720),
    ("08", 990),
    ("09", 1132),
    ("10", 1288),
    ("11", 1536),
    ("12", 1688),
]


def decode_audio_to_stereo(src_path: str) -> list[tuple[float, float]]:
    """Decode any audio file (.mp3, .ogg, .wav) to 44.1kHz stereo floats [-1, 1] using npx remotion ffmpeg."""
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
        tmp_path = tmp.name
    try:
        subprocess.run(
            [
                "npx",
                "remotion",
                "ffmpeg",
                "-y",
                "-i",
                src_path,
                "-ac",
                "2",
                "-ar",
                str(SR),
                "-f",
                "wav",
                tmp_path,
            ],
            cwd=FILM_DIR,
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        with wave.open(tmp_path, "rb") as wf:
            n_frames = wf.getnframes()
            raw = wf.readframes(n_frames)
            vals = struct.unpack(f"<{n_frames * 2}h", raw)
            return [(vals[i * 2] / 32768.0, vals[i * 2 + 1] / 32768.0) for i in range(n_frames)]
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)


def mix_stereo(
    buf_l: list[float],
    buf_r: list[float],
    clip: list[tuple[float, float]],
    start_sec: float,
    gain_l: float = 1.0,
    gain_r: float = 1.0,
) -> None:
    idx0 = int(start_sec * SR)
    for i, (sl, sr) in enumerate(clip):
        idx = idx0 + i
        if 0 <= idx < TOTAL_SAMPLES:
            buf_l[idx] += sl * gain_l
            buf_r[idx] += sr * gain_r


def main() -> None:
    print("Decoding studio music track (happy-beats-business-moves-vol-12)...")
    music_path = os.path.join(MUSIC_DIR, "happy-beats-business-moves-vol-12-by-ende-dot-app.mp3")
    music_samples = decode_audio_to_stereo(music_path)

    print("Decoding SFX library from brag...")
    sfx_cache: dict[str, list[tuple[float, float]]] = {}
    sfx_files = [
        "impact/impactSoft_medium_001.ogg",
        "impact/impactSoft_medium_002.ogg",
        "impact/impactSoft_medium_003.ogg",
        "impact/impactSoft_medium_004.ogg",
        "impact/impactBell_heavy_000.ogg",
        "impact/impactBell_heavy_003.ogg",
        "impact/impactBell_heavy_004.ogg",
        "interface/bong_001.ogg",
        "interface/click_002.ogg",
        "interface/click_003.ogg",
        "interface/click_005.ogg",
        "interface/drop_001.ogg",
        "interface/drop_002.ogg",
        "interface/switch_007.ogg",
        "interface/glitch_002.ogg",
        "casino/card-slide-1.ogg",
        "casino/card-place-1.ogg",
        "ui/click2.ogg",
    ]
    for rel in sfx_files:
        sfx_cache[rel] = decode_audio_to_stereo(os.path.join(SFX_DIR, rel))

    keypress_clips = []
    for k in range(1, 33):
        kp_path = os.path.join(SFX_DIR, "keyboard", f"keypress-{k:03d}.wav")
        if os.path.exists(kp_path):
            keypress_clips.append(decode_audio_to_stereo(kp_path))

    print("Decoding 12 Ava neural voiceover clips...")
    vo_l = [0.0] * TOTAL_SAMPLES
    vo_r = [0.0] * TOTAL_SAMPLES
    vo_env = [0.0] * TOTAL_SAMPLES

    for cid, start_frame in VOICE_CUES:
        mp3_path = os.path.join(VO_DIR, f"{cid}.mp3")
        clip = decode_audio_to_stereo(mp3_path)
        start_idx = int((start_frame / FPS) * SR)
        for i, (sl, sr) in enumerate(clip):
            idx = start_idx + i
            if 0 <= idx < TOTAL_SAMPLES:
                vo_l[idx] += sl * 1.12
                vo_r[idx] += sr * 1.12
                vo_env[idx] = max(vo_env[idx], abs(sl), abs(sr))

    # Smooth sidechain envelope (attack 35ms, release 320ms)
    smooth_env = [0.0] * TOTAL_SAMPLES
    cur = 0.0
    att = math.exp(-1.0 / (0.035 * SR))
    rel = math.exp(-1.0 / (0.320 * SR))
    for i in range(TOTAL_SAMPLES):
        target = 1.0 if vo_env[i] > 0.025 else 0.0
        if target > cur:
            cur = att * cur + (1.0 - att) * target
        else:
            cur = rel * cur + (1.0 - rel) * target
        smooth_env[i] = cur

    # Lay down the music track with warm cinematic sub-bass reinforcement and VO ducking
    mix_l = [0.0] * TOTAL_SAMPLES
    mix_r = [0.0] * TOTAL_SAMPLES

    for i in range(TOTAL_SAMPLES):
        t = i / SR
        ml, mr = music_samples[i % len(music_samples)]
        # Fade in first 0.6s, fade out last 1.8s
        fade_in = min(1.0, t / 0.6)
        fade_out = min(1.0, (DURATION - t) / 1.8) if t > DURATION - 1.8 else 1.0
        # Duck from 0.34 down to 0.14 when voiceover is active
        duck = 0.34 - 0.20 * smooth_env[i]
        # Warm analog sub-octave pulse on 110 BPM grid
        sub = 0.04 * math.sin(2.0 * math.pi * 55.0 * t) * (0.5 + 0.5 * math.cos(2.0 * math.pi * (109.96 / 60.0) * t))
        mix_l[i] = ml * duck * fade_in * fade_out + sub * fade_out
        mix_r[i] = mr * duck * fade_in * fade_out + sub * fade_out

    # Layer SFX synced to exact scene transitions and interactions
    rng = random.Random(42)

    # 1. Scene transitions & whip pans
    transitions = [
        (0.05, "impact/impactBell_heavy_000.ogg", 0.45),
        (1.60, "impact/impactSoft_medium_001.ogg", 0.55),
        (3.90, "impact/impactSoft_medium_002.ogg", 0.58),
        (7.45, "impact/impactSoft_medium_003.ogg", 0.55),
        (10.45, "impact/impactSoft_medium_004.ogg", 0.58),
        (16.38, "impact/impactSoft_medium_001.ogg", 0.52),
        (20.05, "impact/impactBell_heavy_004.ogg", 0.52),  # Pixel burst send!
        (23.78, "impact/impactSoft_medium_002.ogg", 0.55),
        (32.70, "impact/impactSoft_medium_003.ogg", 0.55),
        (37.45, "impact/impactSoft_medium_004.ogg", 0.55),
        (42.65, "impact/impactSoft_medium_001.ogg", 0.55),
        (50.90, "impact/impactSoft_medium_002.ogg", 0.55),
        (55.85, "impact/impactBell_heavy_003.ogg", 0.55),  # End card lockup
    ]
    for sec, name, vol in transitions:
        mix_stereo(mix_l, mix_r, sfx_cache[name], sec, vol, vol)

    # 2. Look / Talk / Sent triptych hits (Beat 2: frames 118, 146, 174)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/drop_001.ogg"], 118 / FPS, 0.50, 0.50)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/drop_002.ogg"], 146 / FPS, 0.52, 0.52)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/bong_001.ogg"], 174 / FPS, 0.58, 0.58)

    # 3. 3D card slides & UI switches (Beat 3, Beat 5, Beat 7, Beat 8)
    card_times = [2.1, 7.8, 16.8, 24.4, 26.8, 29.2, 33.1, 34.4, 35.7]
    for ct in card_times:
        mix_stereo(mix_l, mix_r, sfx_cache["casino/card-slide-1.ogg"], ct, 0.34, 0.34)

    ui_clicks = [0.6, 1.0, 1.4, 8.4, 17.5, 18.5, 19.5, 33.6, 35.0, 36.3, 38.6, 44.2, 57.5]
    for uc in ui_clicks:
        mix_stereo(mix_l, mix_r, sfx_cache["interface/click_003.ogg"], uc, 0.42, 0.42)

    # Cancel override glitch + switch (Beat 9: f1160 = 38.67s)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/glitch_002.ogg"], 38.65, 0.38, 0.38)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/switch_007.ogg"], 38.75, 0.48, 0.48)

    # Destructive guard shield lock (Beat 10: f1335 = 44.5s)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/bong_001.ogg"], 44.50, 0.55, 0.55)

    # 4. Mechanical keyboard keypresses during:
    # - Beat 4 Caret-tracking ticker (11.2s to 15.4s)
    t_cur = 11.20
    while t_cur < 15.35:
        kp = rng.choice(keypress_clips)
        pan = rng.uniform(0.85, 1.0)
        mix_stereo(mix_l, mix_r, kp, t_cur, 0.24 * pan, 0.24 * (1.85 - pan))
        t_cur += rng.uniform(0.055, 0.085)

    # - Beat 6 Terminal command injection burst (20.3s to 21.3s)
    t_cur = 20.30
    while t_cur < 21.25:
        kp = rng.choice(keypress_clips)
        mix_stereo(mix_l, mix_r, kp, t_cur, 0.28, 0.28)
        t_cur += 0.032

    # - Beat 11 Slot reel ticks (51.3s to 54.4s)
    reel_ticks = [51.35, 51.55, 51.72, 51.88, 52.04, 52.20, 52.38, 52.58, 52.82, 53.15, 53.55]
    for rt in reel_ticks:
        mix_stereo(mix_l, mix_r, sfx_cache["interface/click_002.ogg"], rt, 0.36, 0.36)
    mix_stereo(mix_l, mix_r, sfx_cache["interface/bong_001.ogg"], 53.65, 0.55, 0.55)

    # Combine voiceover + soft saturation limiter
    out_bytes = bytearray(TOTAL_SAMPLES * 4)
    for i in range(TOTAL_SAMPLES):
        l = mix_l[i] + vo_l[i]
        r = mix_r[i] + vo_r[i]
        # Soft limiter
        l = math.tanh(l * 0.92) * 0.94
        r = math.tanh(r * 0.92) * 0.94
        li = max(-32767, min(32767, int(l * 32767)))
        ri = max(-32767, min(32767, int(r * 32767)))
        struct.pack_into("<hh", out_bytes, i * 4, li, ri)

    with wave.open(OUT_WAV, "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(SR)
        wf.writeframes(out_bytes)

    print(f"Wrote {OUT_WAV} ({os.path.getsize(OUT_WAV):,} bytes)")


if __name__ == "__main__":
    main()
