#!/usr/bin/env python3
"""
Generate 4 studio-mastered 44.1 kHz stereo soundtracks for saywhen Instagram Reels (9:16 vertical):
1. reel-1-audio.wav (11.8s, 354 frames @ 30fps): VO 01, 03, 04, 06, 02
2. reel-2-audio.wav (10.0s, 300 frames @ 30fps): VO 07, 05, 02
3. reel-3-audio.wav (10.0s, 300 frames @ 30fps): VO 09, 10, 11, 12
4. reel-hero-audio.wav (15.0s, 450 frames @ 30fps): VO 01, 02, 03, 07, 11

Masters each track through FFmpeg EBU R128 (-14 LUFS / -1.2 dBTP) for Instagram Reels.
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
PUBLIC_DIR = os.path.join(FILM_DIR, "public")

SR = 44100
FPS = 30
LEAD_TRIM_THRESH = 10.0 ** (-36.0 / 20.0)
TAIL_KEEP_THRESH = 10.0 ** (-38.0 / 20.0)

FFMPEG = shutil.which("ffmpeg") or "ffmpeg"


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


def trim_vo_silence(cl: list[float], cr: list[float], cid: str) -> tuple[list[float], list[float]]:
    n = len(cl)
    if n == 0:
        return cl, cr
    first = 0
    while first < n and abs(cl[first]) <= LEAD_TRIM_THRESH and abs(cr[first]) <= LEAD_TRIM_THRESH:
        first += 1
    if first >= n:
        return cl, cr
    last = n - 1
    while last > first and abs(cl[last]) <= TAIL_KEEP_THRESH and abs(cr[last]) <= TAIL_KEEP_THRESH:
        last -= 1

    pre = int(0.012 * SR)
    post = int(0.024 * SR)
    s0 = max(0, first - pre)
    s1 = min(n, last + post + 1)
    tl = cl[s0:s1]
    tr = cr[s0:s1]

    if cid == "01":
        gap_thresh = 0.008
        min_gap_samples = int(0.35 * SR)
        keep_gap_samples = int(0.24 * SR)
        run_start = -1
        best = None
        for idx in range(len(tl)):
            if abs(tl[idx]) <= gap_thresh and abs(tr[idx]) <= gap_thresh:
                if run_start < 0:
                    run_start = idx
            else:
                if run_start >= 0 and (idx - run_start) >= min_gap_samples:
                    best = (run_start, idx)
                run_start = -1
        if best is not None:
            g0, g1 = best
            cut_start = g0 + keep_gap_samples // 2
            cut_end = g1 - (keep_gap_samples - keep_gap_samples // 2)
            if cut_end > cut_start:
                cf = min(int(0.008 * SR), keep_gap_samples // 4)
                head_l = tl[:cut_start]
                head_r = tr[:cut_start]
                tail_l = tl[cut_end:]
                tail_r = tr[cut_end:]
                for k in range(min(cf, len(head_l), len(tail_l))):
                    fade = k / max(1, cf)
                    head_l[-1 - k] *= fade
                    head_r[-1 - k] *= fade
                    tail_l[k] *= fade
                    tail_r[k] *= fade
                tl = head_l + tail_l
                tr = head_r + tail_r

    fade_in = min(pre, len(tl))
    for k in range(fade_in):
        g = k / max(1, fade_in)
        tl[k] *= g
        tr[k] *= g
    fade_out = min(post, len(tl))
    for k in range(fade_out):
        g = k / max(1, fade_out)
        tl[-1 - k] *= g
        tr[-1 - k] *= g
    return tl, tr


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
    total_samples = len(dest_l)
    lg = gain * (1.0 - max(0.0, pan))
    rg = gain * (1.0 + min(0.0, pan))
    for i in range(len(sl)):
        idx = start_idx + i
        if 0 <= idx < total_samples:
            dest_l[idx] += sl[i] * lg
            dest_r[idx] += sr[i] * rg


def add_sub_drop(
    dest_l: list[float],
    dest_r: list[float],
    start_sec: float,
    f0: float = 64.0,
    f1: float = 31.0,
    dur: float = 0.48,
    amp: float = 0.34,
) -> None:
    start_idx = int(start_sec * SR)
    n = int(dur * SR)
    total_samples = len(dest_l)
    phase = 0.0
    for i in range(n):
        idx = start_idx + i
        if not (0 <= idx < total_samples):
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
    rise_dur: float = 0.38,
    tail_dur: float = 0.16,
    amp: float = 0.22,
    pan_sweep: float = 0.3,
) -> None:
    rng = random.Random(int(peak_sec * 1000))
    total_dur = rise_dur + tail_dur
    n = int(total_dur * SR)
    start_idx = int((peak_sec - rise_dur) * SR)
    total_samples = len(dest_l)
    lp_l = 0.0
    lp_r = 0.0
    hp_l = 0.0
    hp_r = 0.0
    prev_l = 0.0
    prev_r = 0.0
    for i in range(n):
        idx = start_idx + i
        if not (0 <= idx < total_samples):
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
        hp_l = 0.96 * (hp_l + nl - prev_l)
        prev_l = nl
        hp_r = 0.96 * (hp_r + nr - prev_r)
        prev_r = nr
        lp_l += cutoff * (hp_l - lp_l)
        lp_r += cutoff * (hp_r - lp_r)
        pan = math.sin((i / n - 0.5) * math.pi) * pan_sweep
        dest_l[idx] += lp_l * env * (1.0 - pan)
        dest_r[idx] += lp_r * env * (1.0 + pan)


def build_reel_track(
    out_filename: str,
    duration_sec: float,
    music_l: list[float],
    music_r: list[float],
    music_offset_sec: float,
    vo_cache: dict[str, tuple[list[float], list[float]]],
    vo_cues: list[tuple[str, int]],
    sub_drops: list[tuple[float, float]],
    risers: list[tuple[float, float, float]],
    sfx_events: list[tuple[float, tuple[list[float], list[float]], float, float]],
) -> None:
    total_samples = int(duration_sec * SR)
    vo_l = [0.0] * total_samples
    vo_r = [0.0] * total_samples
    vo_env = [0.0] * total_samples

    print(f"\nBuilding {out_filename} ({duration_sec:.1f}s)...")
    for cid, start_frame in vo_cues:
        cl, cr = vo_cache[cid]
        dur_frames = math.ceil((len(cl) / SR) * FPS)
        print(f"  VO {cid}: start={start_frame} ({start_frame/FPS:.2f}s) frames={dur_frames} end={start_frame + dur_frames}")
        s0 = int((start_frame / FPS) * SR)
        for i in range(len(cl)):
            idx = s0 + i
            if 0 <= idx < total_samples:
                vl = cl[i] * 1.18
                vr = cr[i] * 1.18
                vo_l[idx] += vl
                vo_r[idx] += vr
                vo_env[idx] = max(vo_env[idx], abs(vl), abs(vr))

    duck_env = 0.0
    atk_coef = 1.0 - math.exp(-1.0 / (SR * 0.025))
    rel_coef = 1.0 - math.exp(-1.0 / (SR * 0.280))
    mix_l = [0.0] * total_samples
    mix_r = [0.0] * total_samples
    n_music = len(music_l)
    m_off = int(music_offset_sec * SR)

    for i in range(total_samples):
        target = 1.0 if vo_env[i] > 0.022 else 0.0
        coef = atk_coef if target > duck_env else rel_coef
        duck_env += coef * (target - duck_env)

        ml = music_l[(m_off + i) % n_music]
        mr = music_r[(m_off + i) % n_music]
        t = i / SR
        fade_in = min(1.0, t / 0.25)
        fade_out = min(1.0, (duration_sec - t) / 0.85) if t > duration_sec - 0.85 else 1.0
        duck = 0.42 - 0.26 * duck_env

        mix_l[i] = ml * duck * fade_in * fade_out + vo_l[i]
        mix_r[i] = mr * duck * fade_in * fade_out + vo_r[i]

    for sec, amp in sub_drops:
        add_sub_drop(mix_l, mix_r, sec, amp=amp)

    for peak_sec, amp, pan in risers:
        add_whoosh_riser(mix_l, mix_r, peak_sec, amp=amp, pan_sweep=pan)

    for sec, sfx_pair, gain, pan in sfx_events:
        mix_sfx(mix_l, mix_r, sfx_pair, sec, gain, pan)

    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tf:
        raw_wav = tf.name

    out_wav = os.path.join(PUBLIC_DIR, out_filename)
    try:
        interleaved = bytearray(total_samples * 4)
        for i in range(total_samples):
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
                out_wav,
            ],
            check=True,
        )
        print(f"  Mastered -> {out_wav}")
    finally:
        if os.path.exists(raw_wav):
            os.remove(raw_wav)


def main() -> None:
    music_path = os.path.join(MUSIC_DIR, "happy-beats-business-moves-vol-12-by-ende-dot-app.mp3")
    music_l, music_r = decode_audio_to_stereo(
        music_path,
        af="highpass=f=32,equalizer=f=240:t=q:w=1.2:g=-2.5,equalizer=f=3500:t=q:w=1.4:g=-2.0",
    )

    vo_filter = "highpass=f=85,equalizer=f=3200:t=q:w=1.2:g=2.4,equalizer=f=7500:t=q:w=1.0:g=1.6"
    vo_ids = ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12"]
    vo_cache = {}
    for cid in vo_ids:
        rl, rr = decode_audio_to_stereo(os.path.join(VO_DIR, f"{cid}.mp3"), af=vo_filter)
        vo_cache[cid] = trim_vo_silence(rl, rr, cid)

    sfx_files = [
        "impact/impactSoft_medium_001.ogg",
        "impact/impactSoft_medium_002.ogg",
        "impact/impactSoft_medium_003.ogg",
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
        "casino/card-slide-1.ogg",
        "casino/card-place-1.ogg",
        "ui/click2.ogg",
    ]
    sfx = {rel: decode_audio_to_stereo(os.path.join(SFX_DIR, rel)) for rel in sfx_files}
    keypresses = [
        decode_audio_to_stereo(os.path.join(SFX_DIR, "keyboard", f"keypress-{k:03d}.wav"))
        for k in range(1, 13)
    ]

    # =========================================================================
    # REEL 1: "Look. Talk. Sent." (10.0s = 300 frames)
    # 0.0s - 2.3s (f 0..69): Hook ("Voice input. Without a hotkey.")
    # 2.3s - 5.8s (f 69..174): Look + Talk ("It can see you looking at the screen." + "So you just talk. No button to hold.")
    # 5.8s - 8.0s (f 174..240): Preview -> Ghostty Terminal ("Then it types it in, and presses Enter.")
    # 8.0s - 10.0s (f 240..300): Finale ("Look. Talk. Sent.")
    # =========================================================================
    r1_sfx = [
        (0.05, sfx["impact/impactBell_heavy_000.ogg"], 0.35, 0.0),
        (0.65, sfx["impact/impactGlass_light_002.ogg"], 0.36, -0.15),
        (2.30, sfx["interface/switch_007.ogg"], 0.38, -0.18),
        (3.85, sfx["interface/click_002.ogg"], 0.34, 0.15),
        (5.80, sfx["impact/impactSoft_heavy_001.ogg"], 0.42, 0.0),
        (7.45, sfx["interface/bong_001.ogg"], 0.42, 0.0),
        (8.00, sfx["impact/impactBell_heavy_003.ogg"], 0.44, 0.0),
        (8.20, sfx["interface/click_002.ogg"], 0.28, -0.15),
        (8.85, sfx["interface/click_003.ogg"], 0.30, 0.0),
        (9.45, sfx["interface/bong_001.ogg"], 0.36, 0.15),
    ]
    for k in range(10):
        r1_sfx.append((6.05 + k * 0.07, keypresses[k % len(keypresses)], 0.32, -0.1 + 0.2 * (k / 9.0)))

    build_reel_track(
        out_filename="reel-1-audio.wav",
        duration_sec=11.8,
        music_l=music_l,
        music_r=music_r,
        music_offset_sec=0.0,
        vo_cache=vo_cache,
        vo_cues=[
            ("01", 5),     # 0.17s: "Voice input. Without a hotkey." (65f -> ends 70)
            ("03", 74),    # 2.47s: "It can see you looking at the screen." (50f -> ends 124)
            ("04", 126),   # 4.20s: "So you just talk. No button to hold." (81f -> ends 207)
            ("06", 209),   # 6.97s: "Then it types it in, and presses Enter." (67f -> ends 276)
            ("02", 278),   # 9.27s: "Look. Talk. Sent." (72f -> ends 350 -> 345f = 11.5s)
        ],
        sub_drops=[(0.04, 0.32), (2.40, 0.28), (6.70, 0.34), (8.90, 0.38)],
        risers=[(2.38, 0.22, -0.2), (6.68, 0.26, 0.2), (8.88, 0.30, 0.0)],
        sfx_events=r1_sfx,
    )

    # =========================================================================
    # REEL 2: "Speaks Developer" (10.0s = 300 frames)
    # 0.0s - 3.3s (f 0..99): Hook ("You speak like a person. It writes like a developer.")
    # 3.3s - 7.8s (f 99..234): 3 Live CLI Cleanup Transformations ("It writes the real command, and lets you check.")
    # 7.8s - 10.0s (f 234..300): 34/34 Score + Finale ("Look. Talk. Sent.")
    # =========================================================================
    r2_sfx = [
        (0.05, sfx["impact/impactSoft_medium_001.ogg"], 0.42, 0.0),
        (1.10, sfx["casino/card-slide-1.ogg"], 0.38, -0.20),
        (2.40, sfx["interface/switch_007.ogg"], 0.40, 0.15),
        (3.40, sfx["casino/card-place-1.ogg"], 0.38, -0.15),
        (4.55, sfx["interface/click_005.ogg"], 0.36, 0.15),
        (5.60, sfx["casino/card-slide-1.ogg"], 0.38, -0.12),
        (6.65, sfx["interface/bong_001.ogg"], 0.40, 0.12),
        (7.90, sfx["impact/impactBell_heavy_003.ogg"], 0.44, 0.0),
        (8.15, sfx["interface/click_002.ogg"], 0.28, -0.15),
        (8.80, sfx["interface/click_003.ogg"], 0.30, 0.0),
        (9.40, sfx["interface/bong_001.ogg"], 0.36, 0.15),
    ]
    build_reel_track(
        out_filename="reel-2-audio.wav",
        duration_sec=10.0,
        music_l=music_l,
        music_r=music_r,
        music_offset_sec=12.0,
        vo_cache=vo_cache,
        vo_cues=[
            ("07", 5),     # 0.17s: "You speak like a person. It writes like a developer." (101f -> ends 106)
            ("05", 112),   # 3.73s: "It writes the real command, and lets you check." (81f -> ends 193)
            ("02", 222),   # 7.40s: "Look. Talk. Sent." (72f -> ends 294)
        ],
        sub_drops=[(0.04, 0.32), (3.30, 0.30), (5.50, 0.28), (7.90, 0.38)],
        risers=[(3.28, 0.22, -0.2), (7.88, 0.30, 0.0)],
        sfx_events=r2_sfx,
    )

    # =========================================================================
    # REEL 3: "Safe & Private by Design" (10.0s = 300 frames)
    # 0.0s - 2.6s (f 0..78): Cancel Anytime ("And you can always stop it.")
    # 2.6s - 5.6s (f 78..168): Destructive Guard ("Risky commands never press Enter for you.")
    # 5.6s - 7.9s (f 168..237): 100% On-Device Privacy ("Nothing leaves this computer.")
    # 7.9s - 10.0s (f 237..300): Finale ("Look. Talk. Sent.")
    # =========================================================================
    r3_sfx = [
        (0.05, sfx["interface/drop_001.ogg"], 0.40, 0.0),
        (1.35, sfx["ui/click2.ogg"], 0.44, 0.0),
        (1.38, sfx["impact/impactGlass_light_002.ogg"], 0.34, 0.0),
        (2.60, sfx["impact/impactMetal_heavy_000.ogg"], 0.46, 0.0),
        (3.65, sfx["interface/drop_002.ogg"], 0.40, -0.15),
        (5.60, sfx["impact/impactSoft_medium_003.ogg"], 0.38, 0.0),
        (6.40, sfx["interface/switch_007.ogg"], 0.36, 0.15),
        (7.90, sfx["impact/impactBell_heavy_003.ogg"], 0.44, 0.0),
        (8.15, sfx["interface/click_002.ogg"], 0.28, -0.15),
        (8.80, sfx["interface/click_003.ogg"], 0.30, 0.0),
        (9.40, sfx["interface/bong_001.ogg"], 0.36, 0.15),
    ]
    build_reel_track(
        out_filename="reel-3-audio.wav",
        duration_sec=10.0,
        music_l=music_l,
        music_r=music_r,
        music_offset_sec=24.0,
        vo_cache=vo_cache,
        vo_cues=[
            ("09", 6),     # 0.20s: "And you can always stop it." (45f -> ends 51)
            ("10", 78),    # 2.60s: "Risky commands never press Enter for you." (74f -> ends 152)
            ("11", 168),   # 5.60s: "Nothing leaves this computer." (46f -> ends 214)
            ("12", 222),   # 7.40s: "Look. Talk. Sent." (72f -> ends 294)
        ],
        sub_drops=[(0.04, 0.32), (2.60, 0.38), (5.60, 0.32), (7.90, 0.38)],
        risers=[(2.58, 0.25, -0.25), (5.58, 0.22, 0.2), (7.88, 0.30, 0.0)],
        sfx_events=r3_sfx,
    )

    # =========================================================================
    # REEL HERO (Flagship 15.0s = 450 frames): All-in-one launch cut
    # f 0..70   (0.00 - 2.33s): Beat 1 Hotkey Tax & 3D Keycap Shatter (VO 01 @ f5)
    # f 70..158 (2.33 - 5.27s): Beat 2 Halftone Iris + 3D Parametric Ribbon "Look. Talk. Sent." (VO 02 @ f74)
    # f 158..236(5.27 - 7.87s): Beat 3 3D 478-Point Face Mesh + UI Window (VO 03 @ f160)
    # f 236..350(7.87 - 11.67s):Beat 4 3D Acoustic Terrain + Wet-Ink Cleanup + Ghostty Warp (VO 07 @ f238)
    # f 350..450(11.67 - 15.0s):Beat 5 Vertical Slot Reel ("Runs On Your Computer.") + Fibonacci Vortex Finale (VO 11 @ f354)
    # =========================================================================
    rh_sfx = [
        (0.06, sfx["impact/impactSoft_medium_001.ogg"], 0.38, -0.15),
        (0.55, sfx["casino/card-slide-1.ogg"], 0.35, 0.12),
        (1.00, sfx["interface/drop_002.ogg"], 0.36, -0.12),
        (1.93, sfx["impact/impactGlass_light_002.ogg"], 0.44, 0.0),
        (2.33, sfx["impact/impactBell_heavy_000.ogg"], 0.44, 0.0),
        (2.50, sfx["interface/click_002.ogg"], 0.32, -0.15),
        (3.18, sfx["interface/click_003.ogg"], 0.34, 0.0),
        (3.92, sfx["interface/bong_001.ogg"], 0.40, 0.15),
        (5.27, sfx["interface/switch_007.ogg"], 0.40, -0.18),
        (6.10, sfx["interface/bong_001.ogg"], 0.36, 0.10),
        (7.87, sfx["casino/card-place-1.ogg"], 0.38, -0.10),
        (9.35, sfx["interface/click_005.ogg"], 0.38, 0.0),
        (10.10, sfx["impact/impactSoft_heavy_001.ogg"], 0.44, 0.0),
        (11.10, sfx["interface/bong_001.ogg"], 0.36, 0.12),
        # Slot reel clicks locking onto "Runs On Your Computer." at 12.75s (f=382)
        (11.80, sfx["ui/click2.ogg"], 0.24, -0.15),
        (11.98, sfx["ui/click2.ogg"], 0.26, 0.10),
        (12.16, sfx["ui/click2.ogg"], 0.28, -0.05),
        (12.36, sfx["ui/click2.ogg"], 0.30, 0.12),
        (12.75, sfx["impact/impactBell_heavy_003.ogg"], 0.48, 0.0),
        (13.35, sfx["impact/impactGlass_light_002.ogg"], 0.32, 0.0),
    ]
    for k in range(8):
        rh_sfx.append((10.22 + k * 0.065, keypresses[k % len(keypresses)], 0.30, -0.15 + 0.3 * (k / 7.0)))

    build_reel_track(
        out_filename="reel-hero-audio.wav",
        duration_sec=15.0,
        music_l=music_l,
        music_r=music_r,
        music_offset_sec=4.0,
        vo_cache=vo_cache,
        vo_cues=[
            ("01", 5),     # 0.17s: "Voice input. Without a hotkey." (65f -> ends 70)
            ("02", 74),    # 2.47s: "Look. Talk. Sent." (72f -> ends 146)
            ("03", 160),   # 5.33s: "It can see you looking at the screen." (50f -> ends 210)
            ("07", 238),   # 7.93s: "You speak like a person. It writes like a developer." (101f -> ends 339)
            ("11", 354),   # 11.80s: "Nothing leaves this computer." (46f -> ends 400)
        ],
        sub_drops=[
            (0.04, 0.32),
            (2.33, 0.40),
            (5.27, 0.30),
            (7.87, 0.32),
            (10.10, 0.36),
            (12.75, 0.40),
            (13.30, 0.34),
        ],
        risers=[
            (2.31, 0.28, -0.2),
            (5.25, 0.22, 0.18),
            (7.85, 0.24, -0.15),
            (10.08, 0.26, 0.22),
            (12.73, 0.30, 0.0),
        ],
        sfx_events=rh_sfx,
    )


if __name__ == "__main__":
    main()
