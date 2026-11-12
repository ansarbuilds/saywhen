#!/usr/bin/env python3
"""
Generate a cinema-grade 60.0s (44.1 kHz, stereo) soundtrack for saywhen launch film
AND export frame-accurate 30fps reactive envelopes to film/src/audioData.ts.

Pipeline:
1. Studio music bed (happy-beats-business-moves-vol-12-by-ende-dot-app.mp3)
   shaped with intro ramp, stereo sidechain ducking under Ava Neural VO, and tail fade.
2. Cleanly pre-processed Neural VO (12 cues in film/public/vo/01..12.mp3) via FFmpeg
   85 Hz high-pass + presence air shelf for studio broadcast clarity.
3. Precision SFX choreography from assets-src/sfx/ (impacts, glass ticks, keyboard bursts,
   confirmation bell, red security vault lock) + synthesized sub-drops (55 -> 31 Hz) &
   filtered air whoosh risers at scene cuts.
4. EBU R128 loudness mastering (-14 LUFS / -1.2 dBTP) via FFmpeg loudnorm.
5. Extracts 1800-frame AUDIO_RMS & AUDIO_PEAK into film/src/audioData.ts.
"""

from __future__ import annotations

import math
import os
import random
import shutil
import struct
import subprocess
import tempfile
import wave

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
FILM_DIR = os.path.dirname(SCRIPT_DIR)
VO_DIR = os.path.join(FILM_DIR, "public", "vo")
SFX_DIR = os.path.join(FILM_DIR, "assets-src", "sfx")
MUSIC_DIR = os.path.join(FILM_DIR, "assets-src", "music")
OUT_WAV = os.path.join(FILM_DIR, "public", "demo-soundtrack.wav")
OUT_AUDIODATA = os.path.join(FILM_DIR, "src", "audioData.ts")

SR = 44100
DURATION = 60.0
FPS = 30
TOTAL_SAMPLES = int(SR * DURATION)

# Exact frame offsets matching film/src/cinematic.tsx
VOICE_CUES = [
    ("01", 14),
    ("02", 120),
    ("03", 228),
    ("04", 318),
    ("05", 498),
    ("06", 604),
    ("07", 718),
    ("08", 916),
    ("09", 1130),
    ("10", 1284),
    ("11", 1498),
    ("12", 1682),
]


def find_ffmpeg() -> str:
    sys_ff = shutil.which("ffmpeg")
    if sys_ff:
        return sys_ff
    return "ffmpeg"


FFMPEG = find_ffmpeg()


def decode_audio_to_stereo(path: str, af: str | None = None) -> tuple[list[float], list[float]]:
    cmd = [FFMPEG, "-v", "error", "-i", path]
    if af:
        cmd.extend(["-af", af])
    cmd.extend(["-f", "s16le", "-acodec", "pcm_s16le", "-ar", str(SR), "-ac", "2", "pipe:1"])
    raw = subprocess.check_output(cmd)
    n = len(raw) // 4
    samples = struct.unpack(f"<{n * 2}h", raw)
    left = [samples[i * 2] / 32768.0 for i in range(n)]
    right = [samples[i * 2 + 1] / 32768.0 for i in range(n)]
    return left, right


def mix_sfx(
    dest_l: list[float],
    dest_r: list[float],
    sfx: tuple[list[float], list[float]],
    start_sec: float,
    gain: float = 0.5,
    pan: float = 0.0,
) -> None:
    sl, sr = sfx
    start_idx = int(start_sec * SR)
    lg = gain * (1.0 - max(0.0, pan))
    rg = gain * (1.0 + min(0.0, pan))
    for i in range(len(sl)):
        idx = start_idx + i
        if 0 <= idx < TOTAL_SAMPLES:
            dest_l[idx] += sl[i] * lg
            dest_r[idx] += sr[i] * rg


def add_sub_drop(
    dest_l: list[float],
    dest_r: list[float],
    start_sec: float,
    f0: float = 64.0,
    f1: float = 31.0,
    dur: float = 0.52,
    amp: float = 0.34,
) -> None:
    start_idx = int(start_sec * SR)
    n = int(dur * SR)
    phase = 0.0
    for i in range(n):
        idx = start_idx + i
        if not (0 <= idx < TOTAL_SAMPLES):
            continue
        p = i / max(1, n - 1)
        freq = f1 + (f0 - f1) * math.exp(-5.0 * p)
        phase += 2.0 * math.pi * freq / SR
        atk = min(1.0, p / 0.018)
        dec = math.exp(-4.2 * p)
        s = amp * atk * dec * (0.85 * math.sin(phase) + 0.15 * math.sin(phase * 2.0))
        dest_l[idx] += s
        dest_r[idx] += s


def add_whoosh_riser(
    dest_l: list[float],
    dest_r: list[float],
    peak_sec: float,
    rise_dur: float = 0.44,
    tail_dur: float = 0.18,
    amp: float = 0.22,
    pan_sweep: float = 0.35,
) -> None:
    rng = random.Random(int(peak_sec * 1000))
    total_dur = rise_dur + tail_dur
    n = int(total_dur * SR)
    start_idx = int((peak_sec - rise_dur) * SR)
    lp_l = 0.0
    lp_r = 0.0
    hp_l = 0.0
    hp_r = 0.0
    prev_l = 0.0
    prev_r = 0.0
    for i in range(n):
        idx = start_idx + i
        if not (0 <= idx < TOTAL_SAMPLES):
            continue
        t = i / SR
        if t < rise_dur:
            u = t / rise_dur
            env = (u ** 2.6) * amp
            cutoff = 0.06 + 0.42 * (u ** 2.0)
        else:
            u = (t - rise_dur) / max(0.01, tail_dur)
            env = math.exp(-7.0 * u) * amp * 0.85
            cutoff = 0.45 * (1.0 - 0.65 * u)

        nl = rng.uniform(-1.0, 1.0)
        nr = rng.uniform(-1.0, 1.0)
        # Gentle high-pass (~140 Hz) + sweeping low-pass filter for silky air riser
        hp_l = 0.96 * (hp_l + nl - prev_l)
        prev_l = nl
        hp_r = 0.96 * (hp_r + nr - prev_r)
        prev_r = nr
        lp_l += cutoff * (hp_l - lp_l)
        lp_r += cutoff * (hp_r - lp_r)

        pan = math.sin((i / n - 0.5) * math.pi) * pan_sweep
        dest_l[idx] += lp_l * env * (1.0 - pan)
        dest_r[idx] += lp_r * env * (1.0 + pan)


def write_audio_data_ts(wav_path: str, ts_path: str) -> None:
    with wave.open(wav_path, "rb") as w:
        assert w.getframerate() == SR
        nframes = w.getnframes()
        raw = w.readframes(nframes)
    samples = struct.unpack(f"<{nframes * 2}h", raw)
    spc = SR // FPS  # 1470 samples per frame
    total_frames = int(DURATION * FPS)
    rms_list = []
    peak_list = []
    for f in range(total_frames):
        s0 = f * spc
        s1 = min(nframes, s0 + spc)
        acc = 0.0
        pk = 0.0
        count = max(1, s1 - s0)
        for s in range(s0, s1):
            l_v = samples[s * 2] / 32768.0
            r_v = samples[s * 2 + 1] / 32768.0
            m = 0.5 * (abs(l_v) + abs(r_v))
            acc += 0.5 * (l_v * l_v + r_v * r_v)
            if m > pk:
                pk = m
        rms_list.append(round(math.sqrt(acc / count), 3))
        peak_list.append(round(pk, 3))

    content = (
        "// Auto-extracted 30fps audio reactive envelopes from demo-soundtrack.wav\n"
        f"export const AUDIO_RMS: number[] = {rms_list};\n"
        f"export const AUDIO_PEAK: number[] = {peak_list};\n"
    )
    with open(ts_path, "w", encoding="utf-8") as fp:
        fp.write(content)
    print(f"Wrote {len(rms_list)} reactive frames to {ts_path}")


def main() -> None:
    print("Decoding studio music track (happy-beats-business-moves-vol-12)...")
    music_path = os.path.join(MUSIC_DIR, "happy-beats-business-moves-vol-12-by-ende-dot-app.mp3")
    music_l, music_r = decode_audio_to_stereo(
        music_path,
        af="highpass=f=32,equalizer=f=240:t=q:w=1.2:g=-2.5,equalizer=f=3500:t=q:w=1.4:g=-2.0",
    )

    print("Decoding SFX assets...")
    sfx_files = [
        "impact/impactSoft_medium_001.ogg",
        "impact/impactSoft_medium_002.ogg",
        "impact/impactSoft_medium_003.ogg",
        "impact/impactSoft_medium_004.ogg",
        "impact/impactSoft_heavy_001.ogg",
        "impact/impactBell_heavy_000.ogg",
        "impact/impactBell_heavy_003.ogg",
        "impact/impactGlass_light_002.ogg",
        "impact/impactMetal_heavy_000.ogg",
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
    sfx_cache = {rel: decode_audio_to_stereo(os.path.join(SFX_DIR, rel)) for rel in sfx_files}
    keypresses = [
        decode_audio_to_stereo(os.path.join(SFX_DIR, "keyboard", f"keypress-{k:03d}.wav"))
        for k in range(1, 13)
    ]

    print("Decoding 12 Neural voiceover clips with studio presence EQ...")
    vo_l = [0.0] * TOTAL_SAMPLES
    vo_r = [0.0] * TOTAL_SAMPLES
    vo_env = [0.0] * TOTAL_SAMPLES
    vo_filter = "highpass=f=85,equalizer=f=3200:t=q:w=1.2:g=2.4,equalizer=f=7500:t=q:w=1.0:g=1.6"

    for cid, start_frame in VOICE_CUES:
        path = os.path.join(VO_DIR, f"{cid}.mp3")
        cl, cr = decode_audio_to_stereo(path, af=vo_filter)
        s0 = int((start_frame / FPS) * SR)
        for i in range(len(cl)):
            idx = s0 + i
            if 0 <= idx < TOTAL_SAMPLES:
                vl = cl[i] * 1.18
                vr = cr[i] * 1.18
                vo_l[idx] += vl
                vo_r[idx] += vr
                vo_env[idx] = max(vo_env[idx], abs(vl), abs(vr))

    # Smooth exponential sidechain envelope (attack 28ms, release 310ms)
    print("Mixing sidechain-ducked score, risers, sub-drops & foley...")
    duck_env = 0.0
    atk_coef = 1.0 - math.exp(-1.0 / (SR * 0.028))
    rel_coef = 1.0 - math.exp(-1.0 / (SR * 0.310))

    mix_l = [0.0] * TOTAL_SAMPLES
    mix_r = [0.0] * TOTAL_SAMPLES
    n_music = len(music_l)

    for i in range(TOTAL_SAMPLES):
        target = 1.0 if vo_env[i] > 0.022 else 0.0
        coef = atk_coef if target > duck_env else rel_coef
        duck_env += coef * (target - duck_env)

        ml = music_l[i % n_music]
        mr = music_r[i % n_music]
        t = i / SR

        fade_in = min(1.0, t / 0.45)
        fade_out = min(1.0, (DURATION - t) / 1.65) if t > DURATION - 1.65 else 1.0
        # Vibrant musical energy between sentences (0.40 -> 0.145 under VO)
        duck = 0.40 - 0.255 * duck_env

        mix_l[i] = ml * duck * fade_in * fade_out + vo_l[i]
        mix_r[i] = mr * duck * fade_in * fade_out + vo_r[i]

    # Scene-synced sub-bass impacts + glossy air risers at major film transitions
    for sec, amp in [
        (0.05, 0.32),
        (3.93, 0.35),
        (7.47, 0.26),
        (16.40, 0.32),
        (20.00, 0.36),
        (23.80, 0.28),
        (30.13, 0.34),
        (37.47, 0.30),
        (42.47, 0.38),
        (49.47, 0.30),
        (55.60, 0.40),
    ]:
        add_sub_drop(mix_l, mix_r, sec, amp=amp)

    for peak_sec, amp, pan in [
        (3.92, 0.24, -0.25),
        (16.38, 0.20, 0.25),
        (20.00, 0.28, 0.0),
        (23.78, 0.20, -0.20),
        (30.12, 0.24, 0.20),
        (42.45, 0.25, -0.25),
        (55.60, 0.32, 0.0),
    ]:
        add_whoosh_riser(mix_l, mix_r, peak_sec, amp=amp, pan_sweep=pan)

    # Tactile foley choreography synced directly to beat visual events
    sfx_events = [
        # Beat 1: Option+Space mechanical impact + prism shatter -> whip pan
        (0.06, "impact/impactBell_heavy_000.ogg", 0.34, 0.0),
        (0.92, "impact/impactGlass_light_002.ogg", 0.38, -0.15),
        (1.62, "impact/impactSoft_medium_001.ogg", 0.46, 0.15),
        # Beat 2: Sculpted ribbon -> Look. Talk. Sent.
        (3.93, "impact/impactSoft_medium_002.ogg", 0.48, 0.0),
        (5.55, "interface/bong_001.ogg", 0.36, 0.18),
        # Beat 3: Biometric face lock
        (7.47, "interface/switch_007.ogg", 0.36, -0.20),
        (8.60, "interface/click_003.ogg", 0.32, 0.18),
        # Beat 4: Speech capture live ticks
        (10.50, "interface/click_002.ogg", 0.34, 0.0),
        # Beat 5: Rack focus -> Check preview + countdown clicks
        (16.40, "impact/impactSoft_medium_003.ogg", 0.44, 0.0),
        (17.10, "interface/click_005.ogg", 0.30, 0.15),
        (17.70, "interface/click_005.ogg", 0.26, 0.15),
        (18.30, "interface/click_005.ogg", 0.24, 0.15),
        # Beat 6: Hyperspace burst + Terminal send confirmation
        (20.00, "impact/impactSoft_heavy_001.ogg", 0.42, 0.0),
        (22.10, "interface/bong_001.ogg", 0.42, 0.0),
        # Beat 7: Compiler laser scan + Obsidian/Light theme flip
        (23.80, "casino/card-slide-1.ogg", 0.38, -0.15),
        (26.20, "casino/card-place-1.ogg", 0.36, 0.15),
        (28.30, "interface/switch_007.ogg", 0.38, 0.0),
        # Beat 8: Four Send Modes sequential cards (Pause / Nod / Send it / Type only)
        (30.15, "casino/card-slide-1.ogg", 0.40, -0.35),
        (31.65, "casino/card-place-1.ogg", 0.42, -0.12),
        (33.20, "interface/click_005.ogg", 0.42, 0.12),
        (34.90, "interface/switch_007.ogg", 0.40, 0.35),
        # Beat 9: Always Cancelable strike
        (37.48, "interface/drop_001.ogg", 0.42, 0.0),
        (39.10, "ui/click2.ogg", 0.44, 0.0),
        # Beat 10: Destructive Command Vault Lock
        (42.48, "impact/impactMetal_heavy_000.ogg", 0.46, 0.0),
        (43.25, "interface/drop_002.ogg", 0.38, -0.15),
        (45.50, "interface/glitch_002.ogg", 0.20, 0.15),
        # Beat 11: Privacy grid sweep
        (49.48, "impact/impactSoft_medium_004.ogg", 0.36, 0.0),
        # Beat 12: Finale golden reveal
        (55.62, "impact/impactBell_heavy_003.ogg", 0.48, 0.0),
        (57.20, "interface/bong_001.ogg", 0.32, 0.0),
    ]
    for sec, rel, gain, pan in sfx_events:
        mix_sfx(mix_l, mix_r, sfx_cache[rel], sec, gain, pan)

    # Beat 6 fast mechanical terminal execution ticks
    for k in range(16):
        mix_sfx(mix_l, mix_r, keypresses[k % len(keypresses)], 20.12 + k * 0.075, 0.34, -0.1 + 0.2 * (k / 15.0))

    # Write temporary raw PCM mix -> EBU R128 loudness-mastered stereo WAV via FFmpeg
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tf:
        raw_wav = tf.name

    try:
        interleaved = bytearray(TOTAL_SAMPLES * 4)
        for i in range(TOTAL_SAMPLES):
            # Soft saturation ceiling before mastering
            lv = math.tanh(mix_l[i] * 0.92)
            rv = math.tanh(mix_r[i] * 0.92)
            il = max(-32767, min(32767, int(lv * 32767.0)))
            ir = max(-32767, min(32767, int(rv * 32767.0)))
            struct.pack_into("<hh", interleaved, i * 4, il, ir)

        with wave.open(raw_wav, "wb") as wf:
            wf.setnchannels(2)
            wf.setsampwidth(2)
            wf.setframerate(SR)
            wf.writeframes(bytes(interleaved))

        print("Mastering final soundtrack through FFmpeg EBU R128 (-14 LUFS)...")
        subprocess.run(
            [
                FFMPEG,
                "-y",
                "-v",
                "error",
                "-i",
                raw_wav,
                "-af",
                "loudnorm=I=-14:TP=-1.2:LRA=8",
                "-ar",
                str(SR),
                "-ac",
                "2",
                "-c:a",
                "pcm_s16le",
                OUT_WAV,
            ],
            check=True,
        )
    finally:
        if os.path.exists(raw_wav):
            os.remove(raw_wav)

    write_audio_data_ts(OUT_WAV, OUT_AUDIODATA)
    print(f"Done: {OUT_WAV}")


if __name__ == "__main__":
    main()
