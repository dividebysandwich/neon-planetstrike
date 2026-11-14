#!/usr/bin/env python3
"""Synthesizes the planetstrike game's sound effects (assets/sounds/*.ogg).

- footstep_1..3: a soft rubber sole on a metal grating: a low thud, a tiny metallic click
  and a faint rattle of the grating a moment later (each variant a little different).
- laser_fire: the classic "pew": an FM tone diving from 2.4 kHz to 300 Hz over a low punch,
  a bright transient and a little noise tick, a short slap echo.
- laser_fire_red: Red's blasters: a lower, buzzier zap: a ring-modulated square diving from
  1.7 kHz to 160 Hz with a growl, over a low punch, a short slap echo.
- hit_player: being tagged: a buzzy saw/square zap falling with vibrato over a low
  power-down tone and a deep thump.
- player_hit: someone else hit: an electric crack and a buzzing zap falling over a thud.
- tagged_out: eliminated: a long power-down whine, electric crackle and a low boom.
- hit_target: a team target hit, heard across the arena: a deep boom, a metallic ping, a
  rising three-note bell chime played twice over a two-tone alarm, and a shimmering tail.
- grenade_bounce: the grenade bouncing: a hard clack and a short electronic chirp.
- grenade_throw: a grenade thrown: the latch let go, a whoosh and a rising chirp.
- grenade_arm: the grenade activating and jumping up: a rising charge-up whine ending in a pop.
- grenade_burst: one burst of the hovering grenade: three rapid tiny high zaps.
- respawn: teleporting in: a rising arpeggio over a rising filtered-noise sweep.
- deny: can't fire: a short double buzz.
- empty: out of shots: a dry mechanical double click with a faint dying whine.
- reload: refilling at a station: a rising electric charge hum with a ticking pulse.
- reload_done: the blaster full again: a bright two-note confirmation chirp.
- score_bleep: you hit an enemy or a team target: an arcade "ba-ding", two quick square-wave
  notes up a fifth.
- score_tag: you tagged an enemy out: a fast four-note square-wave arpeggio up to a held top.
- pickup: taking a grenade: a quick sparkly upward blip with a soft metallic latch.
- mega_win: the megatarget hit (Megatarget mode), heard by everyone: a sub-bass impact, a
  rising noise sweep into a big detuned-saw brass fanfare (a triad climbing an octave, then a
  held major chord with a fifth on top), bells over it, and a long shimmering tail.
- power_tick: the ticking of a player who may enter the megatarget's room: a sharp clock tick
  with a tiny high ping.
- power_up: both secondary targets hit: a rising filtered sweep resolving to a bright chord.
- turret_fire: a turret's shot: a hard, metallic, lower zap with a mechanical knock.
- turret_down: a turret switched off: a falling power-down whine with sputtering crackle.
- shield_zap: a player walking into a forcefield: a violent electric crackle and a buzzing
  discharge.
- mirror_ping: a bolt bouncing off a mirror: a bright glassy ping bending up, a tiny
  zap and a short shimmering ring.
- jump_pad: a jump pad throwing someone up: a soft low thump, a rising whoosh of filtered
  noise and a sine chirp sweeping up two octaves.
- gate_hum: a laser gate's buzz while it's on (a seamless one-second loop): a harsh
  electric hum with a fast flutter and a thin high whine.
- charge_loop: the charge gun charging (a seamless one-second loop, its pitch raised as it
  charges): a bright whine with a fast flutter over a low buzz.
- charge_full: the charge gun fully charged: a quick rising two-note chime.
- beam_fire: the charge gun's beam: a heavy crack, a long bright zap sweeping down from
  3 kHz, a sub-bass punch and a hissing tail.
- barrier_up: a barrier going up: a deep electric swell rising into a glassy shimmer.
- barrier_hit: a bolt stopped by a barrier: a dull electric thud with a short crackle.
- barrier_down: a barrier going down: a falling fizzle.
- piece_slide: a turret box's puzzle piece sliding in or out: a short grating swish ending in
  a light plastic clack.
- piece_stuck: a puzzle piece sliding into another one: the swish cut short by a dull knock and
  a buzz.
- puzzle_solved: a turret box's puzzle solved: a latch snapping home and a quick rising chime.
- box_open: a turret box's lid opening: a latch clunk, a pneumatic hiss and the leaves sliding.
- turret_extend: a turret rising out of its box: a servo whine climbing over a rattle, locking
  with a clunk.
- turret_retract: a turret folding back into its box: the servo whine falling, a soft clunk.
- box_close: a turret box's lid closing: a short hiss and a heavy shutting thud.
- event_blackout: a blackout: a heavy relay clunk, the mains hum dying away and a deep
  sub-bass drop.
- event_overdrive: an overdrive: a fast rising arpeggio over a swelling noise sweep, ending in
  a bright chord.
- event_double: double points: a short brassy two-chord fanfare.
- shield_hum: a forcefield's hum (a seamless two-second loop): a mains-like drone with
  slowly beating partials and faint crackle.

Planetstrike (loops are built from components with a whole number of cycles over the loop and
circular filters/noise, so they repeat seamlessly):
- engine_light_tank (2 s loop): a mid turbine whine, a 48 Hz growl and the tracks' rumble.
- engine_heavy_tank (2.5 s loop): deeper and slower: a 36 Hz throb, low turbine, heavy rumble.
- engine_bus (2 s loop): wheeled: a 90 Hz motor hum, an electric whine, soft tyre roar.
- engine_vtol (2 s loop): a fan/jet whoosh pulsing at 14 Hz (rotor), a turbine whine, a throb.
- tracks (1.5 s loop): track links clattering over the sprockets over a grinding rattle.
- lock_tone: a lock-on held: two short high beeps (repeated by the game while it locks).
- lock_warning: being locked on: a fast urgent two-tone warble.
- flares: an aircraft's flares: a volley of popping charges and a fizzing hiss.
- Weapons (the "weapons (layered)" section: every shot a crack, a body, a sub and its mechanism,
  saturated, transient-shaped and set in a room; one sound per weapon):
  - rifle_fire, ember_fire (Red: low, slow, boomy), volt_fire (Blue: a tight electric snap),
    hive_fire (Green: thin, fast, chittering), scatter_fire (a blast, then the pump),
    lance_crack (a supersonic crack and a long tail), striker_fire, skyhook_fire, pistol_fire.
  - medgun_loop, repair_loop (2 s loops): the heal beam's warm pulsing hum, the repair beam's arc.
  - pounder_fire, mauler_fire, burster_fire: a MAX's twin chainguns, cannons and flak tubes.
  - cannon_light, cannon_heavy, rail_fire, autocannon_fire, flak_fire, rocket_fire,
    missile_fire, air_cannon, chaingun, turret_wall_fire, spitfire_fire: vehicles and turrets.
  - tail_small / tail_medium / tail_heavy: a shot heard from far off (dull, soft, rolling),
    played instead of its own sound beyond its range.
- Impacts: impact_body_1..2, impact_shield, impact_armor_1..2 (a vehicle), impact_ground_1..3,
  impact_metal_1..2 (walls and decks), ricochet_1..3.
- Explosions: explosion_small / _medium / _large (by the blast's size), explosion_wreck (a
  vehicle: a cook-off and plates raining down), airburst (flak, in the open air).
- bullet_whiz_1..3, whiz_heavy: near misses (a bolt's whistle, a shell's tearing whoosh).
- reload_rifle / _scatter / _lance / _launcher / _pistol / _heavy, reload_done; casing_1..2,
  casing_shell: the guns' mechanisms.
- vehicle_spawn: a vehicle materialising: a rising sweep and ring cloud, a thump and chord.
- rearm_loop (2 s loop): a charging hum, a geared servo whine, ratchet ticks, an electric fizz.
- deploy / undeploy: the bus's hydraulic legs and its spawn systems powering up / down.
- cloak_on: a shimmer of rings and hiss sliding down. warp: a warp gate's rising suck, a pop.
- warpgate_hum (3 s loop): a deep resonant drone with slowly beating partials.
- forcefield_buzz (1.5 s loop): a fluttering 100 Hz electric hum with sparse crackle.
- forcefield_deny: a thunk, a sizzle and a short buzz.
- terminal_open / terminal_select: UI beeps. hack_start: rising data warble over an alarm.
  hack_tick: a soft blip each second of a hack. hack_stopped: falling notes, a power-down.
- base_captured / base_lost / continent_captured: brass stingers (triumphant, ominous minor,
  and a big fanfare). alert: a two-pulse klaxon.
- heal, fall_thud, hurt, death, spawn, hitmarker, kill_confirm: player feedback.

Each sound is scaled to a loudness (dBFS) suited to the mix: its whole RMS, or (the weapons,
marked "peak" in `SOUNDS`) the RMS of its loudest 100 ms, so that a long tail takes nothing from
the crack; peaks softly limited under full scale. Mono, 44.1 kHz, Ogg Vorbis.

    python3 tools/gen_sfx.py [name ...]      (needs numpy and ffmpeg with libvorbis)

Without arguments every sound is written, else only those named (e.g. `laser_fire` or
`laser_fire.ogg`). Every sound has its own random seed, so the output is identical either way.
"""

import os
import subprocess
import sys

import numpy as np

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
OUT = os.path.join(ROOT, "assets", "sounds")
RATE = 44100
EXT = ".ogg"

rng = np.random.default_rng(0)


# ---------------------------------------------------------------- helpers

def seconds(n):
    return np.arange(int(n * RATE)) / RATE


def noise(t, lo, hi, order=2.0):
    """Band-limited noise of `t` seconds between `lo` and `hi` Hz, RMS 1."""
    n = int(t * RATE)
    f = np.fft.rfftfreq(n, 1.0 / RATE)
    f[0] = 1e-3
    mag = 1.0 / np.sqrt(1.0 + (lo / f) ** (2 * order)) / np.sqrt(1.0 + (f / hi) ** (2 * order))
    spec = mag * np.exp(2j * np.pi * rng.random(len(f)))
    spec[0] = 0.0
    x = np.fft.irfft(spec, n)
    return x / np.sqrt(np.mean(x * x))


def sweep_noise(t, hi_from, hi_to, lo=20.0):
    """Noise whose upper cutoff glides from `hi_from` to `hi_to` Hz (one-pole filters)."""
    x = rng.standard_normal(int(t * RATE))
    hi = np.geomspace(hi_from, hi_to, len(x))
    a = np.exp(-2 * np.pi * hi / RATE)
    y = np.empty_like(x)
    s1 = s2 = 0.0
    for i, (v, k) in enumerate(zip(x, a)):
        s1 = (1 - k) * v + k * s1
        s2 = (1 - k) * s1 + k * s2
        y[i] = s2
    y -= np.convolve(y, np.ones(int(RATE / lo)) / int(RATE / lo), "same")
    return y / np.sqrt(np.mean(y * y))


def env(t, attack, decay, hold=0.0):
    """Envelope: rises over `attack`, holds `hold` s, then decays with time constant `decay`."""
    x = seconds(t)
    rise = 1.0 - np.exp(-x / max(attack, 1e-5))
    fall = np.where(x < hold, 1.0, np.exp(-(x - hold) / decay))
    return rise * fall


def fades(x, fade_in=0.002, fade_out=0.02):
    n_in, n_out = int(fade_in * RATE), int(fade_out * RATE)
    x = x.copy()
    x[:n_in] *= np.linspace(0, 1, n_in)
    x[len(x) - n_out:] *= np.linspace(1, 0, n_out)
    return x


def at(total, start, x):
    """`x` placed `start` s into a silence of `total` s (cut at the end)."""
    out = np.zeros(int(total * RATE))
    i = int(start * RATE)
    out[i:i + len(x)] += x[:max(0, len(out) - i)]
    return out


def phase(f):
    """Phase (in cycles) of a tone whose frequency follows the array `f` (Hz)."""
    return np.cumsum(f) / RATE


def thump(t, hz, decay, drop=0.5):
    """A low thud: a sine falling in pitch by `drop` (fraction), decaying."""
    x = seconds(t)
    f = hz * (1 - drop * (1 - np.exp(-x / decay)))
    return np.sin(2 * np.pi * phase(f)) * env(t, 0.001, decay)


def click(t, lo, hi, decay):
    return noise(t, lo, hi, 3.0) * env(t, 0.0002, decay)


def ring(t, freqs, decays, gains=None):
    """Damped sine partials (metal ringing)."""
    x = seconds(t)
    gains = gains or [1.0] * len(freqs)
    return sum(g * np.sin(2 * np.pi * f * x + rng.random() * 6.28) * np.exp(-x / d)
               for f, d, g in zip(freqs, decays, gains))


def saw(f):
    p = phase(f)
    return 2.0 * (p - np.floor(p + 0.5))


def square(f, duty=0.5):
    p = phase(f) % 1.0
    return np.where(p < duty, 1.0, -1.0)


def lowpass(x, hi, order=2.0):
    f = np.fft.rfftfreq(len(x), 1.0 / RATE)
    return np.fft.irfft(np.fft.rfft(x) / np.sqrt(1.0 + (f / hi) ** (2 * order)), len(x))


def highpass(x, lo, order=2.0):
    f = np.fft.rfftfreq(len(x), 1.0 / RATE)
    f[0] = 1e-3
    return np.fft.irfft(np.fft.rfft(x) / np.sqrt(1.0 + (lo / f) ** (2 * order)), len(x))


def echoes(x, delays, gains, hi):
    """`x` with its echoes `delays` s later, `gains` as loud, dulled above `hi` Hz."""
    t = len(x) / RATE
    dull = lowpass(x, hi)
    out = x.copy()
    for d, g in zip(delays, gains):
        out += g * at(t, d, dull)
    return out


def norm(x):
    return x / np.max(np.abs(x))


# ---------------------------------------------------------------- dynamics, space, layers

def smooth(x, tau):
    """`x` through a one-pole low-pass of time constant `tau` s (an envelope follower's
    smoothing; padded, so nothing wraps round)."""
    n = len(x)
    pad = int(min(6 * tau, 2.0) * RATE) + 1
    f = np.fft.rfftfreq(n + pad, 1.0 / RATE)
    y = np.fft.irfft(np.fft.rfft(np.concatenate([x, np.zeros(pad)])) / (1.0 + 2j * np.pi * f * tau), n + pad)
    return y[:n]


def saturate(x, drive, bias=0.0):
    """A tanh waveshaper (`drive`: how hard, 1 hardly at all; `bias`: asymmetry, for even
    harmonics), at twice the sample rate against aliasing. The peak is kept."""
    n = len(x)
    peak = np.max(np.abs(x)) + 1e-12
    spec = np.fft.rfft(x / peak)
    up = np.fft.irfft(np.concatenate([spec, np.zeros(n - len(spec) + 1)]), 2 * n) * 2
    y = np.tanh(drive * (up + bias)) - np.tanh(drive * bias)
    y = np.fft.irfft(np.fft.rfft(y)[:len(spec)], n) / 2
    return y * peak / (np.max(np.abs(y)) + 1e-12)


def bands(x, split_hz):
    """`x` split in three (low, mid, high) at the two frequencies of `split_hz`; they add up
    to `x`."""
    low = lowpass(x, split_hz[0], 4.0)
    high = highpass(x, split_hz[1], 4.0)
    return low, x - low - high, high


def transient(x, attack_gain, sustain_gain, split_hz=(200, 2500)):
    """A three-band transient shaper: each band's attacks raised by `attack_gain` (0: as they
    are, 1: about twice as loud) and what follows them by `sustain_gain` (negative: tighter)."""
    out = np.zeros(len(x))
    for band, (fast, slow) in zip(bands(x, split_hz), ((0.004, 0.06), (0.0015, 0.03), (0.0005, 0.015))):
        a = np.abs(band)
        f, s = np.maximum(smooth(a, fast), 0), np.maximum(smooth(a, slow), 0)
        k = (f - s) / (f + s + 1e-9)
        out += band * (1.0 + attack_gain * np.clip(k, 0, 1) + sustain_gain * np.clip(-k, 0, 1))
    return out


def compress(x, thresh_db, ratio, attack=0.003, release=0.08, makeup=0.0):
    """A compressor: levels over `thresh_db` (dB under the peak) reduced by `ratio`, coming on
    over `attack` s and letting go over `release` s; `makeup` dB added after."""
    peak = np.max(np.abs(x)) + 1e-12
    a = np.abs(x) / peak
    level = np.maximum(np.maximum(smooth(a, attack), smooth(a, release)), 1e-6)
    over = np.maximum(20 * np.log10(level) - thresh_db, 0.0)
    return x * 10 ** ((-over * (1 - 1 / ratio) + makeup) / 20)


def sub(t, hz, decay, drop=0.3, attack=0.004):
    """A sub-bass layer: a pure sine sagging in pitch by `drop`, swelling in over `attack` s
    (no click of its own: the crack is another layer's)."""
    x = seconds(t)
    f = hz * (1 - drop * (1 - np.exp(-x / (decay * 1.5))))
    return np.sin(2 * np.pi * phase(f)) * env(t, attack, decay)


def delayed(x, t):
    """`x` starting `t` s later (as long as it was: its end is cut)."""
    return np.concatenate([np.zeros(int(t * RATE)), x])[:len(x)]


def _comb(x, delay, gain, damp):
    """A feedback comb filter (`delay` samples, `damp`: how much duller each time round)."""
    y = x.copy()
    for i in range(delay + 1, len(y), delay):
        j = min(i + delay, len(y))
        y[i:j] += gain * ((1 - damp) * y[i - delay:j - delay] + damp * y[i - delay - 1:j - delay - 1])
    return y


def _allpass(x, delay, gain):
    y = -gain * x
    y[delay:] += x[:-delay]
    for i in range(delay, len(y), delay):
        j = min(i + delay, len(y))
        y[i:j] += gain * y[i - delay:j - delay]
    return y


def reverb(x, size=1.0, decay=0.6, damp=0.35, mix=0.2, predelay=0.012):
    """`x` in a space: four damped combs and two all-passes (`size`: the delays' scale, 1 a
    hall; `decay`: s to fall 60 dB; `damp`: 0 bright, 1 dull; `mix`: how much of it;
    `predelay` s before it starts). The length stays, so leave room for the tail."""
    wet = np.zeros(len(x))
    for d in (0.0297, 0.0371, 0.0411, 0.0437):
        n = max(int(d * size * RATE), 8)
        wet += _comb(x, n, 10 ** (-3.0 * n / RATE / decay), damp)
    for d in (0.005, 0.0017):
        wet = _allpass(wet, max(int(d * size * RATE), 4), 0.6)
    wet = highpass(delayed(wet, predelay), 120)
    return x + mix * wet * (np.sqrt(np.mean(x * x)) / (np.sqrt(np.mean(wet * wet)) + 1e-12))


def slapback(x, delay, gain, hi=2500):
    """One hard echo off a wall `delay` s later, `gain` as loud, dulled above `hi` Hz."""
    return x + gain * delayed(lowpass(x, hi), delay)


def mech(T, kind):
    """A gun's mechanism, peak about 1: `bolt` (a bolt slamming home), `servo` (a short motor
    whir), `belt` (links rattling through), `tink` (a spent case ringing), `latch` (a catch
    snapping), `slide` (a part racked back and forth), `clunk` (a heavy breech)."""
    if kind == "bolt":
        y = (click(T, 900, 7000, 0.004) + thump(T, 210, 0.012, drop=0.4) * 0.7
             + ring(T, [1650, 2730, 4100], [0.012, 0.008, 0.005], [0.5, 0.35, 0.2]) * 0.6)
    elif kind == "servo":
        x = seconds(T)
        f = 300 + 500 * np.sin(np.pi * np.clip(x / T, 0, 1)) ** 2
        y = (saw(f) * 0.5 + square(f * 3.02, 0.4) * 0.2) * np.sin(np.pi * np.clip(x / T, 0, 1)) ** 0.5
        y = lowpass(y, 3500) * (0.8 + 0.2 * np.sin(2 * np.pi * 47 * x))
    elif kind == "belt":
        y = crackle(T, 220, 1200, 6000) * 0.8 + ring(T, [2300, 3700], [0.004, 0.003], [0.3, 0.2]) * 0.3
        y = y * env(T, 0.002, T * 0.5)
    elif kind == "tink":
        k = 0.9 + 0.2 * rng.random()
        y = ring(T, [3900 * k, 6350 * k, 9100 * k], [0.03, 0.02, 0.012], [1.0, 0.6, 0.35]) * env(T, 0.0003, 0.05)
        y = y + click(T, 4000, 12000, 0.0008) * 0.5
    elif kind == "latch":
        y = click(T, 1800, 9000, 0.0015) + ring(T, [2600, 3900], [0.008, 0.005], [0.5, 0.3])
    elif kind == "slide":
        h = T / 2
        back = click(h, 700, 5000, 0.006) + noise(h, 1500, 6000) * env(h, 0.01, h * 0.3) * 0.25
        fwd = click(h, 900, 7000, 0.004) + thump(h, 180, 0.015, drop=0.4) * 0.6
        y = at(T, 0.0, back * 0.8) + at(T, h, fwd)
    elif kind == "clunk":
        y = (thump(T, 120, 0.035, drop=0.5) + click(T, 400, 4000, 0.008) * 0.8
             + ring(T, [540, 930, 1480, 2300], [0.05, 0.035, 0.025, 0.015], [0.5, 0.4, 0.3, 0.2]) * 0.5)
    else:
        raise ValueError(kind)
    return y / (np.max(np.abs(y)) + 1e-12)


def far(x, tail=1.2, hi=900):
    """`x` as heard from far off: the highs gone (above `hi` Hz), the attack softened, `tail`
    s of rolling reverberation after it."""
    y = np.concatenate([x, np.zeros(int(tail * RATE))])
    y = lowpass(y, hi, 1.5)
    y = smooth(y, 0.002) + 0.5 * smooth(y, 0.012)
    y = reverb(y, size=2.2, decay=tail * 0.9, damp=0.6, mix=0.9, predelay=0.03)
    return slapback(y, 0.19, 0.3, hi * 0.7)


def window_rms(x, win=0.1):
    """The RMS level of the loudest `win` s of `x`."""
    n = min(int(win * RATE), len(x))
    c = np.concatenate([[0.0], np.cumsum(x * x)])
    return np.sqrt(np.max(c[n:] - c[:-n]) / n)


CEILING = 10 ** (-2.0 / 20)
KNEE = 0.6


def soft_limit(x):
    """Peaks over `KNEE` bent over towards `CEILING` (-2 dBFS: the encoder overshoots a
    little); under it nothing is changed."""
    a = np.abs(x)
    over = KNEE + (CEILING - KNEE) * np.tanh((a - KNEE) / (CEILING - KNEE))
    return np.sign(x) * np.where(a > KNEE, over, a)


def write(name, x, level_db, loop=False, mode="rms"):
    """Writes `x`, as loud as `level_db` dBFS. `mode` "rms": the whole sound's RMS, the peaks
    softly limited under full scale (loops, tones, the older sounds). "peak": the RMS of its
    loudest 100 ms, with a soft-knee limiter at -2 dBFS (one-shots: a long tail doesn't push
    the crack into the limiter then). (`loop`: played round and round, so no fades at the
    ends.)"""
    x = x - np.mean(x) if loop else fades(x - np.mean(x))
    target = 10 ** (level_db / 20)
    if mode == "peak":
        limit = lambda g: soft_limit(x * g)
        measure = window_rms
    else:
        limit = lambda g: 0.95 * np.tanh(x * g / 0.95)
        measure = lambda y: np.sqrt(np.mean(y ** 2))
    lo, hi = 0.0, 1e3 / np.max(np.abs(x))
    for _ in range(60):
        g = (lo + hi) / 2
        lo, hi = (g, hi) if measure(limit(g)) < target else (lo, g)
    x = limit((lo + hi) / 2)
    if not np.all(np.isfinite(x)):
        sys.exit(f"{name}: not finite")
    pcm = np.round(np.clip(x, -1, 1) * 32767).astype("<i2").tobytes()
    path = os.path.join(OUT, name + EXT)
    codec = ["-c:a", "libvorbis", "-q:a", "6"] if EXT == ".ogg" else ["-compression_level", "12"]
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "s16le", "-ar", str(RATE), "-ac", "1", "-i", "-",
         *codec, path],
        input=pcm, check=True)
    peak = 20 * np.log10(np.max(np.abs(x)))
    rms = 20 * np.log10(np.sqrt(np.mean(x ** 2)))
    loud = 20 * np.log10(window_rms(x))
    print(f"{path}  {len(x) / RATE:.3f} s  {mode} {level_db:.1f} dBFS  rms {rms:.1f}  "
          f"loudest 100 ms {loud:.1f}  peak {peak:.1f} dBFS")


# ---------------------------------------------------------------- sounds

def footstep(variant):
    T = 0.15
    v = variant - 1
    body = thump(T, 85 + 12 * v, 0.022, drop=0.35)
    scuff = noise(T, 120, 900) * env(T, 0.003, 0.018) * 0.25
    base = 2350 + 310 * v
    ping = ring(T, [base, base * 1.53, base * 2.21, base * 3.08],
                [0.012, 0.009, 0.006, 0.004], [1.0, 0.7, 0.45, 0.25])
    tick = click(T, 2500, 9000, 0.0015) * 0.6
    metal = (ping * 0.12 + tick * 0.15) * env(T, 0.0005, 0.03)
    rattle = at(T, 0.028 + 0.009 * v,
                ring(0.06, [base * 1.17, base * 1.9], [0.006, 0.004]) * 0.05
                + click(0.06, 3000, 8000, 0.001) * 0.05)
    return body + scuff + metal + rattle


def laser_fire():
    T = 0.25
    x = seconds(T)
    f = 300 + (2400 - 300) * np.exp(-x / 0.045)
    index = 1.6 * np.exp(-x / 0.05) + 0.25
    mod = np.sin(2 * np.pi * phase(f * 0.5))
    carrier = np.sin(2 * np.pi * phase(f) + index * mod)
    tone = (carrier + 0.25 * np.sin(4 * np.pi * phase(f))) * env(T, 0.0008, 0.07, hold=0.02)
    transient = click(T, 3000, 10000, 0.003) * 0.35
    tick = noise(T, 800, 5000) * env(T, 0.0005, 0.012) * 0.12
    # The low end: a short punch under the pew.
    punch = thump(T, 95, 0.045, drop=0.45) * 0.9 + lowpass(noise(T, 30, 250), 160) * env(T, 0.001, 0.03) * 0.3
    y = tone + transient + tick + punch
    return echoes(lowpass(y, 9000), [0.045], [0.18], 2500)


def laser_fire_red():
    T = 0.27
    x = seconds(T)
    f = 160 + (1700 - 160) * np.exp(-x / 0.05)
    buzz = square(f, 0.35) * (0.6 + 0.4 * np.sin(2 * np.pi * phase(f * 0.5)))
    growl = saw(f * 0.5) * (0.5 + 0.5 * np.sin(2 * np.pi * 55 * x))
    tone = lowpass(buzz * 0.7 + growl * 0.4, 5000) * env(T, 0.0008, 0.07, hold=0.025)
    transient = click(T, 2000, 8000, 0.003) * 0.3
    punch = thump(T, 85, 0.05, drop=0.45) * 1.0 + lowpass(noise(T, 30, 250), 160) * env(T, 0.001, 0.03) * 0.3
    return echoes(tone + transient + punch, [0.05], [0.18], 2200)


def hit_player():
    T = 0.6
    x = seconds(T)
    vib = 1 + 0.06 * np.sin(2 * np.pi * 19 * x)
    f = (150 + 750 * np.exp(-x / 0.16)) * vib
    zap = 0.6 * saw(f) + 0.4 * square(f * 1.005, 0.4)
    zap = lowpass(zap, 3800) * env(T, 0.002, 0.16, hold=0.06)
    fl = 220 * np.exp(-x / 0.35) + 45
    down = np.sin(2 * np.pi * phase(fl)) * env(T, 0.01, 0.25, hold=0.1) * 0.8
    sizzle = noise(T, 1500, 7000) * env(T, 0.001, 0.05) * 0.15
    body = thump(T, 70, 0.14, drop=0.4) * 1.1
    return zap + down + sizzle + body


def player_hit():
    T = 0.4
    x = seconds(T)
    crack = click(T, 2500, 11000, 0.003) * 0.9 + noise(T, 1500, 8000) * env(T, 0.0005, 0.02) * 0.3
    f = (220 + 1300 * np.exp(-x / 0.05)) * (1 + 0.08 * np.sin(2 * np.pi * 31 * x))
    zap = lowpass(0.6 * saw(f) + 0.4 * square(f * 1.01, 0.3), 4500) * env(T, 0.001, 0.07, hold=0.03)
    thud = thump(T, 120, 0.06, drop=0.5) + thump(T, 65, 0.1, drop=0.35) * 0.9
    return crack + 0.7 * zap + 0.8 * thud


def tagged_out():
    T = 1.2
    x = seconds(T)
    vib = 1 + (0.01 + 0.06 * x / T) * np.sin(2 * np.pi * (9 + 6 * x) * x)
    f = 1500 * np.exp(-x / 0.33) * vib + 45
    whine = np.sin(2 * np.pi * phase(f)) + 0.35 * lowpass(saw(f), 3000)
    whine *= env(T, 0.005, 0.35, hold=0.5)
    # crackle: sparse filtered impulses, thinning out
    imp = np.zeros(len(x))
    density = 900 * np.exp(-x / 0.35) + 30
    hits = rng.random(len(x)) < density / RATE
    imp[hits] = rng.standard_normal(hits.sum())
    crackle = highpass(lowpass(imp, 6500), 1200)
    crackle = crackle / np.max(np.abs(crackle)) * env(T, 0.001, 0.4, hold=0.15) * 0.5
    boom = at(T, 0.02, thump(1.0, 70, 0.32, drop=0.45)) * 1.1
    rumble = lowpass(noise(T, 25, 400), 180) * env(T, 0.005, 0.3) * 0.35
    zap = click(T, 2000, 9000, 0.01) * 0.4
    return 0.7 * whine + crackle + boom + rumble + zap


def bell(t, hz, decay):
    return ring(t, [hz, hz * 2.76, hz * 5.40, hz * 2.0],
                [decay, decay * 0.45, decay * 0.2, decay * 0.7],
                [1.0, 0.35, 0.15, 0.25])


def hit_target():
    T = 1.3
    notes = [(0.0, 1046.5), (0.07, 1318.5), (0.14, 1568.0), (0.42, 1318.5), (0.49, 1568.0), (0.56, 2093.0)]
    chime = sum(at(T, s, bell(T, hz, 0.22) * env(T, 0.001, 1.0)) * (1.0 + 0.15 * (i % 3))
                for i, (s, hz) in enumerate(notes))
    ping = (ring(T, [3520, 5280, 7390], [0.03, 0.02, 0.012], [1.0, 0.5, 0.3])
            + click(T, 4000, 12000, 0.002)) * 0.4
    x = seconds(T)
    # Two-tone alarm under the chime.
    alarm_f = np.where((x * 7.0) % 1.0 < 0.5, 660.0, 880.0)
    alarm = lowpass(square(alarm_f, 0.5), 2500) * env(T, 0.01, 0.35, hold=0.5) * 0.18
    boom = thump(T, 62, 0.28, drop=0.4) * 1.3 + lowpass(noise(T, 25, 300), 140) * env(T, 0.003, 0.2) * 0.4
    shimmer = noise(T, 6000, 12000) * (0.6 + 0.4 * np.sin(2 * np.pi * 23 * x))
    shimmer *= at(T, 0.14, env(T, 0.03, 0.3)) * 0.12
    sparkle = at(T, 0.56, bell(T, 3136, 0.25)) * 0.3
    return echoes(chime + ping + alarm + boom + shimmer + sparkle, [0.09, 0.18], [0.25, 0.1], 5000)


def grenade_bounce():
    T = 0.2
    clack = (click(T, 1500, 8000, 0.004) * 0.7
             + ring(T, [1150, 2930, 4410], [0.02, 0.012, 0.007], [0.5, 0.4, 0.25])
             + thump(T, 190, 0.015, drop=0.3) * 0.6)
    x = seconds(0.07)
    f = 1500 + 1700 * x / 0.07
    chirp = (np.sin(2 * np.pi * phase(f)) + 0.2 * square(f)) * env(0.07, 0.003, 0.03, hold=0.02)
    return clack + at(T, 0.03, lowpass(chirp, 7000) * 0.3)


def grenade_throw():
    T = 0.45
    x = seconds(T)
    latch = click(T, 1200, 6000, 0.004) * 0.6 + ring(T, [1900, 3100], [0.015, 0.01]) * 0.25
    whoosh = sweep_noise(T, 600, 4000) * np.sin(np.pi * np.clip(x / 0.38, 0, 1)) ** 2 * 0.5
    f = 500 + 1600 * np.clip((x - 0.03) / 0.25, 0, 1) ** 1.5
    chirp = at(T, 0.03, (np.sin(2 * np.pi * phase(f)) * env(T, 0.005, 0.09, hold=0.12))[:int((T - 0.03) * RATE)]) * 0.3
    return latch + highpass(whoosh, 300) + chirp


def grenade_arm():
    T = 0.6
    x = seconds(T)
    rise = np.clip(x / 0.5, 0, 1)
    f = 200 * (2500 / 200) ** rise
    trem = 0.75 + 0.25 * np.sin(2 * np.pi * phase(12 + 50 * rise))
    whine = (np.sin(2 * np.pi * phase(f)) + 0.3 * lowpass(saw(f * 0.5), 5000)) * trem
    whine *= (0.15 + 0.85 * rise ** 1.5) * np.where(x < 0.5, 1.0, np.exp(-(x - 0.5) / 0.008))
    pop = at(T, 0.5, thump(0.1, 140, 0.03, drop=0.5) * 1.2
             + click(0.1, 800, 9000, 0.006) * 0.8
             + noise(0.1, 300, 3000) * env(0.1, 0.001, 0.02) * 0.3)
    return 0.55 * whine + pop


def grenade_burst():
    T = 0.15
    out = np.zeros(int(T * RATE))
    for i, s in enumerate([0.0, 0.042, 0.088]):
        d = 0.04
        x = seconds(d)
        hz = 4200 - 300 * i
        f = 1300 + (hz - 1300) * np.exp(-x / 0.011)
        z = np.sin(2 * np.pi * phase(f) + 0.8 * np.sin(2 * np.pi * phase(f * 0.5)))
        z = z * env(d, 0.0005, 0.011) + click(d, 5000, 12000, 0.001) * 0.25
        out += at(T, s, z * (1.0 - 0.15 * i))
    return out


def respawn():
    T = 0.8
    notes = [523.3, 659.3, 784.0, 1046.5, 1318.5]
    arp = np.zeros(int(T * RATE))
    for i, hz in enumerate(notes):
        d = T - 0.07 * i
        x = seconds(d)
        f = hz * (1 + 0.006 * np.sin(2 * np.pi * 6 * x))
        p = phase(f)
        tri = 2 * np.abs(2 * (p - np.floor(p + 0.5))) - 1
        tone = (0.7 * tri + 0.3 * np.sin(2 * np.pi * 2 * p)) * env(d, 0.004, 0.16 + 0.03 * i)
        arp += at(T, 0.07 * i, tone)
    x = seconds(T)
    sweep = highpass(sweep_noise(T, 300, 10000), 250)
    sweep *= np.sin(np.pi * np.clip(x / 0.7, 0, 1)) ** 2 * (0.7 + 0.3 * np.sin(2 * np.pi * 31 * x))
    return echoes(0.6 * arp + 0.2 * sweep, [0.08, 0.16], [0.3, 0.12], 4000)


def deny():
    T = 0.2
    x = seconds(T)
    tone = square(np.full(len(x), 140.0)) + square(np.full(len(x), 147.0), 0.3)
    gate = ((x > 0.0) & (x < 0.075)) | ((x > 0.1) & (x < 0.175))
    gate = np.convolve(gate.astype(float), np.ones(88) / 88, "same")  # 2 ms soft edges
    return lowpass(tone, 2200) * gate


def empty():
    T = 0.22
    out = np.zeros(int(T * RATE))
    for start, g in ((0.0, 1.0), (0.055, 0.7)):
        c = click(0.05, 1800, 7000, 0.004) + 0.5 * ring(0.05, [2300, 3900], [0.01, 0.006])
        out += g * at(T, start, c)
    x = seconds(T)
    f = 900 * np.exp(-x * 9)
    whine = np.sin(2 * np.pi * phase(f)) * env(T, 0.003, 0.05) * 0.15
    return out + whine


def reload():
    T = 0.9
    x = seconds(T)
    f = 110 * (1 + 2.2 * (x / T) ** 1.4)
    hum = 0.6 * saw(f) + 0.4 * square(f * 2.01, 0.3)
    hum = lowpass(hum, 1800) * (0.4 + 0.6 * x / T)
    tick = (np.sin(2 * np.pi * 14 * x * (1 + x)) > 0.6).astype(float)
    tick = np.convolve(tick, np.ones(66) / 66, "same")
    zap = highpass(noise(T, 3000, 9000), 2500) * tick * 0.25
    return fades(hum * 0.7 + zap, 0.02, 0.08)


def pickup():
    T = 0.45
    x = seconds(T)
    f = 600 * np.exp(x * 5.5)
    blip = np.sin(2 * np.pi * phase(f)) * env(T, 0.002, 0.07)
    sparkle = highpass(noise(T, 4000, 12000), 3500) * env(T, 0.001, 0.12)
    sparkle *= (np.sin(2 * np.pi * 37 * x) > 0).astype(float)
    latch = at(T, 0.11, click(0.08, 900, 5000, 0.006) + 0.6 * ring(0.08, [1700, 2600], [0.03, 0.02]))
    return 0.7 * blip + 0.2 * sparkle + 0.5 * latch


def chip(notes, step, last, duty=0.25):
    """8-bit style notes: (Hz) each `step` s long, the last held `last` s."""
    T = step * (len(notes) - 1) + last + 0.12
    out = np.zeros(int(T * RATE))
    for i, hz in enumerate(notes):
        d = last if i == len(notes) - 1 else step
        x = seconds(d)
        f = hz * (1 + 0.01 * np.exp(-x / 0.01))
        tone = (square(np.full(len(x), 0.0) + f, duty) * 0.7 + np.sin(2 * np.pi * phase(f)) * 0.3)
        out += at(T, step * i, tone * env(d, 0.002, d * 0.8, hold=d * 0.4))
    return echoes(lowpass(out, 9000), [0.07], [0.2], 6000)


def score_bleep():
    return chip([1318.5, 1975.5], 0.055, 0.11)


def score_tag():
    return chip([1046.5, 1318.5, 1568.0, 2093.0], 0.05, 0.2)


def supersaw(hz, T, voices=7, spread=0.012):
    """Detuned saws around `hz` (a big analog brass/pad tone)."""
    x = seconds(T)
    out = np.zeros(len(x))
    for k in range(voices):
        d = 1.0 + spread * (k - (voices - 1) / 2) / ((voices - 1) / 2)
        f = np.full(len(x), hz * d) * (1 + 0.003 * np.sin(2 * np.pi * (4.5 + k * 0.3) * x))
        p = phase(f) + rng.random()
        out += 2.0 * (p - np.floor(p + 0.5))
    return out / voices


def brass(hz, T, attack, decay, hold, bright):
    """A supersaw note through a filter that opens with the note (brassy swell)."""
    tone = supersaw(hz, T) + 0.5 * supersaw(hz / 2, T, 3, 0.006)
    # Filter sweep approximated by mixing a dull and a bright copy along the envelope.
    e = env(T, attack, decay, hold)
    dull = lowpass(tone, hz * 2.0)
    shine = lowpass(tone, hz * bright)
    return (dull * (1 - e) + shine * e) * e


def mega_win():
    T = 4.6
    x = seconds(T)
    # Impact: a sub drop and a burst of noise.
    boom = thump(T, 55, 0.6, drop=0.55) * 1.6 + lowpass(noise(T, 20, 400), 180) * env(T, 0.002, 0.35) * 0.8
    crack = click(T, 1500, 12000, 0.015) * 0.6
    # A rising sweep into the fanfare.
    sweep = sweep_noise(0.55, 300, 9000) * env(0.55, 0.4, 0.05, hold=0.5)
    sweep = at(T, 0.0, sweep) * 0.35
    # Fanfare: C E G (rising), then C' held as a big chord (C E G C' G').
    fan = np.zeros(len(x))
    for start, hz, dur in [(0.30, 261.6, 0.18), (0.48, 329.6, 0.18), (0.66, 392.0, 0.18)]:
        fan += at(T, start, brass(hz, dur + 0.25, 0.01, 0.12, dur * 0.6, 6.0)) * 0.9
    chord = [261.6, 329.6, 392.0, 523.3, 784.0]
    for i, hz in enumerate(chord):
        fan += at(T, 0.86, brass(hz, T - 0.86, 0.04, 1.4, 0.9, 7.0)) * (0.85 if i < 4 else 0.55)
    # Sub pedal under the chord.
    pedal = at(T, 0.86, np.sin(2 * np.pi * 65.4 * seconds(T - 0.86)) * env(T - 0.86, 0.05, 1.6, 0.8)) * 0.7
    # Bells and sparkle.
    bells = sum(at(T, 0.86 + 0.09 * i, bell(T, hz, 0.9)) for i, hz in enumerate([1046.5, 1568.0, 2093.0, 3136.0])) * 0.22
    shimmer = noise(T, 5000, 14000) * (0.5 + 0.5 * np.sin(2 * np.pi * 17 * x))
    shimmer *= at(T, 0.86, env(T, 0.2, 1.4, hold=0.4)) * 0.1
    mix = boom + crack + sweep + fan + pedal + bells + shimmer
    return echoes(mix, [0.13, 0.27, 0.41], [0.35, 0.2, 0.1], 4000)


def power_tick():
    T = 0.12
    tick = click(T, 2500, 9000, 0.0025) * 1.2
    knock = ring(T, [1800, 2950], [0.012, 0.008], [1.0, 0.6]) * 0.8
    ping = np.sin(2 * np.pi * 3520 * seconds(T)) * env(T, 0.0005, 0.025) * 0.35
    return tick + knock + ping


def power_up():
    T = 1.1
    x = seconds(T)
    f = 180 * (1 + 5 * (1 - np.exp(-x / 0.25)))
    rise = lowpass(saw(f) * env(T, 0.02, 0.25, hold=0.45), 3500) * 0.5
    sweep = sweep_noise(T, 400, 10000) * env(T, 0.3, 0.2, hold=0.4) * 0.25
    chord = sum(at(T, 0.45, np.sin(2 * np.pi * phase(np.full(int((T - 0.45) * RATE), hz))) * env(T - 0.45, 0.01, 0.3))
                for hz in [1046.5, 1318.5, 1568.0, 2093.0]) * 0.3
    return echoes(rise + sweep + chord, [0.08], [0.25], 6000)


def turret_fire():
    T = 0.32
    x = seconds(T)
    f = 900 * np.exp(-x / 0.06) + 140
    zap = (square(f, 0.3) * 0.6 + saw(f * 1.5) * 0.4) * env(T, 0.001, 0.07)
    zap = lowpass(zap, 4500)
    knock = thump(T, 120, 0.05, drop=0.6) * 0.9 + click(T, 300, 3000, 0.006) * 0.6
    clank = ring(T, [930, 1410, 2350], [0.04, 0.03, 0.02], [0.6, 0.4, 0.3]) * 0.4
    return echoes(zap + knock + clank, [0.06], [0.2], 3000)


def turret_down():
    T = 1.2
    x = seconds(T)
    f = 1400 * np.exp(-x / 0.35) + 60
    whine = (saw(f) * 0.5 + np.sin(2 * np.pi * phase(f * 0.5)) * 0.5) * env(T, 0.005, 0.5, hold=0.3)
    whine = lowpass(whine, 3000)
    sputter = noise(T, 1500, 8000) * ((rng.random(len(x)) < 0.002).astype(float).cumsum() % 2)
    sputter = lowpass(sputter * env(T, 0.01, 0.4), 6000) * 0.25
    clunk = at(T, 0.85, thump(0.3, 90, 0.08) + click(0.3, 200, 2000, 0.01) * 0.5) * 0.9
    return whine + sputter + clunk


def shield_zap():
    T = 0.8
    x = seconds(T)
    crackle = noise(T, 1500, 14000) * (rng.random(len(x)) < 0.02).astype(float)
    crackle = lowpass(crackle, 9000) * env(T, 0.001, 0.25) * 3.0
    buzz_f = 110 + 30 * np.sin(2 * np.pi * 13 * x)
    buzz = (square(buzz_f, 0.5) * 0.6 + saw(buzz_f * 2.01) * 0.4) * env(T, 0.002, 0.3, hold=0.1)
    buzz = lowpass(buzz, 4000)
    snap = click(T, 800, 12000, 0.004) * 1.5 + thump(T, 70, 0.12) * 0.8
    return crackle + buzz * 0.7 + snap


def shield_hum():
    T = 2.0
    x = seconds(T)
    # Every partial a whole number of cycles in 2 s (multiples of 0.5 Hz): a seamless loop.
    drone = (np.sin(2 * np.pi * 60 * x) + 0.6 * np.sin(2 * np.pi * 120.5 * x)
             + 0.35 * np.sin(2 * np.pi * 180 * x) + 0.25 * np.sin(2 * np.pi * 240.5 * x)
             + 0.15 * np.sin(2 * np.pi * 420 * x))
    drone *= 0.8 + 0.2 * np.sin(2 * np.pi * 1.5 * x)
    # Noise from noise() is periodic over its length: the crackle loops too.
    hiss = noise(T, 2000, 9000) * (0.5 + 0.5 * np.sin(2 * np.pi * 3.0 * x)) ** 4 * 0.12
    return drone * 0.5 + hiss


def mirror_ping():
    T = 0.45
    x = seconds(T)
    f = 2600 * (1 + 0.25 * (1 - np.exp(-x / 0.03)))
    ping = np.sin(2 * np.pi * phase(f)) * env(T, 0.001, 0.12) * 0.6
    glass = ring(T, [3900, 5230, 7110], [0.09, 0.06, 0.04], [0.5, 0.35, 0.25]) * 0.5
    zap = lowpass(square(1800 * np.exp(-x / 0.02) + 400, 0.3) * env(T, 0.0005, 0.02), 6000) * 0.3
    return echoes(ping + glass + zap, [0.05], [0.25], 7000)


def jump_pad():
    T = 0.9
    x = seconds(T)
    thud = thump(T, 70, 0.12) * 0.9
    whoosh = sweep_noise(T, 300, 6000) * env(T, 0.05, 0.45, hold=0.2) * 0.6
    f = 220 * 4 ** np.clip(x / 0.5, 0, 1)
    chirp = np.sin(2 * np.pi * phase(f)) * env(T, 0.01, 0.35, hold=0.25) * 0.35
    return echoes(thud + whoosh + chirp, [0.07], [0.2], 5000)


def gate_hum():
    T = 1.0
    x = seconds(T)
    # Every partial a whole number of cycles in 1 s: a seamless loop.
    buzz = square(np.full(len(x), 100.0), 0.3) * 0.5 + saw(np.full(len(x), 200.0)) * 0.3
    buzz = lowpass(buzz, 2500) * (0.75 + 0.25 * np.sin(2 * np.pi * 25 * x))
    whine = np.sin(2 * np.pi * 3200 * x) * 0.05 * (0.6 + 0.4 * np.sin(2 * np.pi * 3 * x))
    return buzz + whine


def charge_loop():
    T = 1.0
    x = seconds(T)
    # Every partial a whole number of cycles in 1 s: a seamless loop.
    whine = (np.sin(2 * np.pi * 660 * x) + 0.5 * np.sin(2 * np.pi * 1320 * x)
             + 0.25 * np.sin(2 * np.pi * 1985 * x)) * (0.7 + 0.3 * np.sin(2 * np.pi * 16 * x))
    buzz = lowpass(saw(np.full(len(x), 110.0)), 1500) * 0.3
    return whine * 0.4 + buzz


def charge_full():
    T = 0.4
    a = np.sin(2 * np.pi * 1318.5 * seconds(T)) * env(T, 0.002, 0.08)
    b = at(T, 0.07, np.sin(2 * np.pi * 1975.5 * seconds(T - 0.07)) * env(T - 0.07, 0.002, 0.18))
    return echoes((a + b) * 0.6, [0.06], [0.25], 8000)


def beam_fire():
    T = 1.0
    x = seconds(T)
    f = 3000 * np.exp(-x / 0.12) + 120
    zap = (saw(f) * 0.5 + square(f * 0.5, 0.4) * 0.3) * env(T, 0.001, 0.35, hold=0.05)
    zap = lowpass(zap, 7000)
    crack = click(T, 1500, 14000, 0.004) * 1.6
    punch = thump(T, 50, 0.18) * 1.2
    hiss = noise(T, 3000, 12000) * env(T, 0.01, 0.5) * 0.25
    return echoes(zap + crack + punch + hiss, [0.09, 0.2], [0.3, 0.15], 5000)


def lance_fire():
    T = 0.9
    x = seconds(T)
    # A sharp crack and a high, glassy whine falling away (the prism lance).
    crack = click(T, 2500, 15000, 0.002) * 1.8
    f = 5200 * np.exp(-x / 0.06) + 900
    whine = np.sin(2 * np.pi * phase(f)) * env(T, 0.0005, 0.18, hold=0.01) * 0.5
    glass = ring(T, [2900, 4350, 6100], [0.25, 0.18, 0.12], [0.35, 0.25, 0.18]) * 0.4
    punch = thump(T, 70, 0.08, drop=0.5) * 0.9
    return echoes(crack + whine + glass + punch, [0.11, 0.26], [0.3, 0.15], 6000)


def sensor_pulse():
    T = 1.2
    x = seconds(T)
    # A sonar ping: a pure tone with a soft attack and a long, echoing tail.
    f = 1320 * (1 - 0.04 * (1 - np.exp(-x / 0.3)))
    ping = np.sin(2 * np.pi * phase(f)) * env(T, 0.004, 0.22) * 0.7
    under = np.sin(2 * np.pi * phase(f * 0.5)) * env(T, 0.01, 0.15) * 0.25
    return echoes(ping + under, [0.18, 0.36, 0.54], [0.4, 0.2, 0.1], 5000)


def event_storm():
    T = 2.2
    x = seconds(T)
    # Glass shimmering up: a cloud of bright rings sweeping upwards, over a rising whoosh.
    out = np.zeros_like(x)
    for k in range(9):
        start = 0.08 * k
        hz = 1400 * 2 ** (k / 6)
        out += at(T, start, ring(T - start, [hz, hz * 1.5, hz * 2.1], [0.5, 0.35, 0.25],
                                 [0.4, 0.25, 0.15])) * 0.35
    whoosh = sweep_noise(T, 400, 9000) * env(T, 0.4, 0.6, hold=0.6) * 0.35
    return echoes(out + whoosh, [0.12, 0.27], [0.35, 0.18], 7000)


def event_lowgrav():
    T = 2.4
    x = seconds(T)
    # A slow, wobbling rise: two detuned tones gliding up an octave, a soft airy swell.
    f = 110 * 2 ** np.clip(x / 1.6, 0, 1)
    wob = 1 + 0.03 * np.sin(2 * np.pi * 3.0 * x)
    tones = (np.sin(2 * np.pi * phase(f * wob)) + 0.6 * np.sin(2 * np.pi * phase(f * 1.503))) \
        * env(T, 0.3, 0.6, hold=1.1) * 0.5
    air = sweep_noise(T, 300, 3000) * env(T, 0.6, 0.5, hold=0.8) * 0.2
    return echoes(tones + air, [0.2, 0.4], [0.3, 0.15], 4000)


def headshot():
    T = 0.5
    x = seconds(T)
    # Two bright ascending pings over a hard knock.
    pings = sum(
        at(T, start, np.sin(2 * np.pi * hz * seconds(T - start)) * env(T - start, 0.001, 0.09))
        for hz, start in ((1760, 0.0), (2637, 0.06))
    ) * 0.5
    knock = thump(T, 140, 0.05, drop=0.6) * 0.9 + click(T, 2000, 9000, 0.002) * 0.6
    return pings + knock


def barrier_up():
    T = 0.7
    x = seconds(T)
    f = 90 * (1 + 3 * (1 - np.exp(-x / 0.15)))
    swell = lowpass(saw(f) * env(T, 0.04, 0.3, hold=0.2), 2500) * 0.6
    shimmer = ring(T, [2100, 3150, 4200], [0.25, 0.2, 0.15], [0.3, 0.2, 0.15]) * env(T, 0.15, 0.4)
    return echoes(swell + shimmer * 0.6, [0.07], [0.2], 6000)


def barrier_hit():
    T = 0.3
    x = seconds(T)
    thud = thump(T, 110, 0.06) * 1.0
    crackle = noise(T, 2000, 10000) * (rng.random(len(x)) < 0.04).astype(float)
    crackle = lowpass(crackle, 8000) * env(T, 0.001, 0.08) * 2.0
    buzz = lowpass(square(np.full(len(x), 160.0), 0.4), 2500) * env(T, 0.001, 0.07) * 0.4
    return thud + crackle + buzz


def barrier_down():
    T = 0.6
    x = seconds(T)
    f = 900 * np.exp(-x / 0.2) + 80
    fizz = lowpass(saw(f) * env(T, 0.005, 0.3), 3000) * 0.5
    hiss = noise(T, 1500, 9000) * env(T, 0.005, 0.25) * 0.3
    return fizz + hiss


def slide_swish(T, length):
    """A piece's swish along its slot: band noise with a rattle, `length` s long."""
    x = seconds(T)
    body = noise(T, 700, 4500) * env(T, 0.008, length * 0.5, hold=length * 0.4)
    rattle = 0.6 + 0.4 * np.sin(2 * np.pi * 55 * x)
    body = body * rattle * (x < length + 0.03)
    return lowpass(body, 5000) * 0.35


def piece_slide():
    T = 0.22
    swish = slide_swish(T, 0.12)
    clack = at(T, 0.12, click(0.1, 1500, 7000, 0.004) * 1.0 + ring(0.1, [2400, 3700], [0.012, 0.008], [0.5, 0.3]))
    return swish + clack


def piece_stuck():
    T = 0.3
    swish = slide_swish(T, 0.07)
    knock = at(T, 0.07, thump(0.23, 160, 0.04, drop=0.4) * 1.1 + click(0.23, 300, 2500, 0.006) * 0.8)
    n = int(0.12 * RATE)
    buzz = at(T, 0.075, lowpass(square(np.full(n, 110.0), 0.35), 1800) * env(0.12, 0.002, 0.04)) * 0.35
    return swish + knock + buzz


def puzzle_solved():
    T = 0.75
    latch = click(T, 1200, 8000, 0.003) * 1.2 + thump(T, 220, 0.03, drop=0.3) * 0.7
    notes = [(0.05, 1046.5), (0.11, 1318.5), (0.17, 1568.0), (0.23, 2093.0)]
    chime = sum(at(T, t0, (np.sin(2 * np.pi * hz * seconds(T - t0)) * 0.7
                           + square(np.full(int((T - t0) * RATE), hz), 0.25) * 0.15)
                   * env(T - t0, 0.002, 0.18)) for t0, hz in notes) * 0.45
    return echoes(latch + lowpass(chime, 8000), [0.08], [0.25], 6000)


def box_open():
    T = 0.55
    x = seconds(T)
    clunk = thump(T, 95, 0.06, drop=0.5) * 1.0 + click(T, 400, 4000, 0.008) * 0.7
    hiss = noise(T, 2500, 11000) * env(T, 0.01, 0.12, hold=0.05) * 0.35
    slide = at(T, 0.06, noise(0.35, 500, 2500) * env(0.35, 0.03, 0.12, hold=0.12)) * 0.3
    stop = at(T, 0.33, click(0.2, 600, 5000, 0.006) * 0.7 + ring(0.2, [1300, 2100], [0.03, 0.02], [0.4, 0.25]))
    return clunk + hiss + slide + stop


def servo(T, f0, f1, tau):
    """A servo motor's whine gliding from `f0` to `f1` Hz with a geared rattle."""
    x = seconds(T)
    f = f1 + (f0 - f1) * np.exp(-x / tau)
    whine = saw(f) * 0.5 + np.sin(2 * np.pi * phase(f * 2)) * 0.3
    gears = 0.7 + 0.3 * np.sign(np.sin(2 * np.pi * phase(f / 6)))
    return lowpass(whine * gears, 3500)


def turret_extend():
    T = 0.85
    run = 0.6
    motor = servo(T, 180, 520, 0.25) * env(T, 0.03, 0.05, hold=run - 0.05) * 0.5
    rattle = noise(T, 800, 5000) * (rng.random(int(T * RATE)) < 0.01).astype(float)
    rattle = lowpass(rattle, 5000) * env(T, 0.02, 0.1, hold=run - 0.1) * 1.2
    lock = at(T, run, thump(T - run, 130, 0.05, drop=0.5) * 0.9 + click(T - run, 800, 6000, 0.004) * 0.8
              + ring(T - run, [1700, 2600], [0.04, 0.03], [0.4, 0.3]))
    return echoes(motor + rattle + lock, [0.06], [0.15], 4000)


def turret_retract():
    T = 0.8
    run = 0.58
    motor = servo(T, 520, 150, 0.3) * env(T, 0.02, 0.05, hold=run - 0.05) * 0.5
    rattle = noise(T, 800, 5000) * (rng.random(int(T * RATE)) < 0.01).astype(float)
    rattle = lowpass(rattle, 5000) * env(T, 0.02, 0.1, hold=run - 0.1) * 1.2
    clunk = at(T, run, thump(T - run, 100, 0.06, drop=0.5) * 0.9 + click(T - run, 300, 3000, 0.006) * 0.5)
    return motor + rattle + clunk


def box_close():
    T = 0.5
    hiss = noise(T, 2500, 10000) * env(T, 0.02, 0.06, hold=0.12) * 0.25
    slide = noise(T, 500, 2500) * env(T, 0.02, 0.06, hold=0.2) * 0.3
    shut = at(T, 0.24, thump(0.26, 75, 0.08, drop=0.5) * 1.3 + click(0.26, 200, 3000, 0.01) * 0.8
              + ring(0.26, [700, 1150], [0.05, 0.03], [0.3, 0.2]))
    return hiss + slide + shut

def event_blackout():
    T = 1.6
    x = seconds(T)
    clunk = thump(T, 60, 0.15) * 1.3 + click(T, 200, 3000, 0.012) * 0.8
    f = 120 * np.exp(-x / 0.5) + 25
    hum = (np.sin(2 * np.pi * phase(f)) + 0.4 * np.sin(2 * np.pi * phase(f * 2))) * env(T, 0.005, 0.9)
    drop = np.sin(2 * np.pi * phase(55 * np.exp(-x / 0.6) + 20)) * env(T, 0.02, 1.1) * 0.8
    return echoes(clunk + hum * 0.5 + drop, [0.15], [0.25], 2000)


def event_overdrive():
    T = 1.3
    notes = [523.25, 659.25, 783.99, 1046.5, 1318.5, 1568.0]
    arp = sum(at(T, 0.07 * i, square(np.full(int((T - 0.07 * i) * RATE), hz), 0.3)
                 * env(T - 0.07 * i, 0.002, 0.12)) for i, hz in enumerate(notes)) * 0.25
    sweep = sweep_noise(T, 500, 9000) * env(T, 0.3, 0.3, hold=0.2) * 0.2
    chord = sum(at(T, 0.45, np.sin(2 * np.pi * phase(np.full(int((T - 0.45) * RATE), hz)))
                   * env(T - 0.45, 0.01, 0.6)) for hz in [1046.5, 1318.5, 1568.0, 2093.0]) * 0.25
    return echoes(lowpass(arp, 7000) + sweep + chord, [0.09], [0.25], 7000)


def event_double():
    T = 1.2
    def chord(t0, hzs, dur):
        return sum(at(T, t0, brass(hz, T - t0, 0.02, dur, 0.05, 6.0)) for hz in hzs)
    fan = chord(0.0, [392.0, 493.88, 587.33], 0.2) + chord(0.22, [523.25, 659.25, 783.99], 0.7)
    return echoes(lowpass(fan * 0.3, 6000), [0.1, 0.22], [0.3, 0.15], 6000)


# ---------------------------------------------------------------- planetstrike: loop helpers

def cyc(T, hz):
    """`hz` rounded to a whole number of cycles in `T` s (so a loop of `T` s is seamless)."""
    return round(hz * T) / T


def const(T, hz):
    return np.full(int(T * RATE), cyc(T, hz))


def loop_at(T, start, x):
    """`x` placed `start` s into a silence of `T` s, wrapping round the end (for loops)."""
    out = np.zeros(int(T * RATE))
    idx = (int(start * RATE) + np.arange(len(x))) % len(out)
    np.add.at(out, idx, x)
    return out


def loop_echoes(x, delays, gains, hi):
    """`echoes` for a loop: the echoes wrap round to the start."""
    dull = lowpass(x, hi)
    out = x.copy()
    for d, g in zip(delays, gains):
        out += g * np.roll(dull, int(d * RATE))
    return out


def crackle(T, density, lo, hi):
    """Sparse random impulses (`density` per s) band-filtered between `lo` and `hi` Hz, peak 1.
    The filters are circular, so this loops over `T` s too."""
    n = int(T * RATE)
    imp = np.zeros(n)
    hits = rng.random(n) < density / RATE
    imp[hits] = rng.standard_normal(hits.sum())
    y = highpass(lowpass(imp, hi), lo)
    return y / (np.max(np.abs(y)) + 1e-12)


def pulses(T, rate, sharp):
    """A pulse train at `rate` Hz (a whole number of cycles in `T`), 0..1, `sharp`er peaks."""
    x = seconds(T)
    return (0.5 + 0.5 * np.sin(2 * np.pi * cyc(T, rate) * x)) ** sharp


# ---------------------------------------------------------------- planetstrike: vehicle engines (loops)

def engine_light_tank():
    T = 2.0
    x = seconds(T)
    # A mid turbine whine with a slow wobble, a growling engine and the tracks' rumble.
    wob = 1 + 0.004 * np.sin(2 * np.pi * 1.5 * x)
    tf = cyc(T, 1180) * wob
    turbine = (np.sin(2 * np.pi * phase(tf)) + 0.4 * np.sin(2 * np.pi * phase(tf * 2))
               + 0.5 * np.sin(2 * np.pi * phase(tf * 0.5)))
    turbine *= 0.85 + 0.15 * np.sin(2 * np.pi * 3.0 * x)
    ef = const(T, 48)
    engine = lowpass(saw(ef) + 0.5 * square(ef * 1.5, 0.3), 520) * (0.7 + 0.3 * np.sin(2 * np.pi * 24 * x))
    rumble = lowpass(noise(T, 25, 400), 200) * (0.75 + 0.25 * np.sin(2 * np.pi * 9 * x))
    clatter = noise(T, 900, 4500) * pulses(T, 12, 8)
    return turbine * 0.1 + engine * 0.5 + rumble * 0.45 + clatter * 0.1


def engine_heavy_tank():
    T = 2.5
    x = seconds(T)
    # Deeper and slower: a low diesel-like throb, a lower turbine and a heavy rumble.
    throb = 0.8 + 0.2 * np.sin(2 * np.pi * 0.8 * x)
    ef = const(T, 36)
    engine = lowpass(saw(ef) + 0.6 * square(ef * 2, 0.35) + 0.4 * saw(ef * 0.5), 380)
    engine *= (0.6 + 0.4 * pulses(T, 14, 2)) * throb
    tf = cyc(T, 720) * (1 + 0.003 * np.sin(2 * np.pi * 1.2 * x))
    turbine = np.sin(2 * np.pi * phase(tf)) + 0.35 * np.sin(2 * np.pi * phase(tf * 1.5))
    sub = np.sin(2 * np.pi * cyc(T, 36) * x) * throb
    rumble = lowpass(noise(T, 20, 300), 140) * (0.7 + 0.3 * np.sin(2 * np.pi * 6 * x))
    clatter = noise(T, 600, 3000) * pulses(T, 8, 10)
    return engine * 0.55 + sub * 0.35 + turbine * 0.06 + rumble * 0.55 + clatter * 0.12


def engine_bus():
    T = 2.0
    x = seconds(T)
    # Wheeled and electric: a motor hum, a bright electric whine, soft tyre roar.
    hum = (np.sin(2 * np.pi * cyc(T, 90) * x) + 0.5 * np.sin(2 * np.pi * cyc(T, 180) * x)
           + 0.3 * np.sin(2 * np.pi * cyc(T, 270) * x)) * (0.85 + 0.15 * np.sin(2 * np.pi * 0.5 * x))
    buzz = lowpass(square(const(T, 90), 0.3), 900) * 0.3
    wf = cyc(T, 1650) * (1 + 0.003 * np.sin(2 * np.pi * 3.0 * x))
    whine = np.sin(2 * np.pi * phase(wf)) + 0.4 * np.sin(2 * np.pi * phase(wf * 1.5))
    whine *= 0.8 + 0.2 * np.sin(2 * np.pi * 1.0 * x)
    tyres = noise(T, 150, 1500) * (0.85 + 0.15 * np.sin(2 * np.pi * 7.0 * x))
    road = lowpass(noise(T, 25, 250), 150)
    return hum * 0.35 + buzz + whine * 0.07 + tyres * 0.15 + road * 0.35


def engine_vtol():
    T = 2.0
    x = seconds(T)
    # A jet/fan whoosh pulsing with the rotor blades, a turbine whine and a low throb.
    pulse = 0.55 + 0.45 * pulses(T, 14, 3)
    jet = highpass(noise(T, 250, 7000), 300) * pulse
    air = noise(T, 1500, 9000) * (0.8 + 0.2 * pulses(T, 14, 1))
    low = lowpass(noise(T, 30, 250), 150) * pulse
    tf = cyc(T, 2900) * (1 + 0.002 * np.sin(2 * np.pi * 2.0 * x))
    whine = np.sin(2 * np.pi * phase(tf)) + 0.5 * np.sin(2 * np.pi * phase(tf * 1.5))
    throb = np.sin(2 * np.pi * cyc(T, 70) * x) * pulses(T, 14, 4)
    return jet * 0.3 + air * 0.1 + low * 0.45 + whine * 0.05 + throb * 0.35


def tracks():
    T = 1.5
    # Track links clattering over the sprockets: metallic clicks at 10 Hz, an accent every
    # other one, over a grinding rattle.
    out = np.zeros(int(T * RATE))
    for i in range(15):
        k = 1 + 0.08 * rng.standard_normal()
        d = 0.06
        c = (ring(d, [1250 * k, 2700 * k, 4100 * k], [0.008, 0.006, 0.004], [0.6, 0.4, 0.25])
             + click(d, 1500, 7000, 0.002) * 0.7 + thump(d, 150, 0.012, drop=0.4) * 0.6)
        g = (1.0 if i % 2 == 0 else 0.6) * (0.8 + 0.2 * rng.random())
        out += loop_at(T, i / 10 + 0.004 * rng.standard_normal(), c * g)
    grind = noise(T, 300, 2500) * (0.6 + 0.4 * pulses(T, 10, 2)) * 0.12
    rumble = lowpass(noise(T, 30, 300), 160) * 0.25
    return out * 0.6 + grind + rumble


# ---------------------------------------------------------------- planetstrike: lock-ons

def lock_tone():
    T = 0.3
    out = np.zeros(len(seconds(T)))
    for t0 in (0.0, 0.12):
        d = 0.07
        xx = seconds(d)
        tone = np.sin(2 * np.pi * 2200 * xx) * 0.8 + np.sin(2 * np.pi * 4400 * xx) * 0.15
        gate = np.clip(xx / 0.004, 0, 1) * np.clip((d - xx) / 0.008, 0, 1)
        out += at(T, t0, tone * gate)
    return out


def lock_warning():
    T = 0.6
    out = np.zeros(len(seconds(T)))
    for k in range(6):
        d = 0.09
        xx = seconds(d)
        f = 1400 if k % 2 == 0 else 1050
        tone = square(np.full(len(xx), float(f)), 0.5) * 0.5 + np.sin(2 * np.pi * f * xx) * 0.5
        gate = np.clip(xx / 0.004, 0, 1) * np.clip((d - xx) / 0.006, 0, 1)
        out += at(T, k * 0.1, lowpass(tone, 6000) * gate)
    return out


def flares():
    T = 1.5
    out = np.zeros(len(seconds(T)))
    for k in range(6):
        t0 = k * 0.11
        d = 0.25
        pop = click(d, 1500, 9000, 0.004) * 0.9 + thump(d, 200, 0.02, drop=0.5) * 0.6
        out += at(T, t0, pop)
    fizz = noise(T, 2500, 11000) * env(T, 0.05, 0.6, hold=0.3) * 0.35
    return echoes(out + fizz, [0.15, 0.3], [0.25, 0.1], 4000)


def cloak():
    # A rising shimmer: filtered noise swept up under a glassy chord, fading out.
    T = 0.7
    x = seconds(T)
    swish = sweep_noise(T, 1500, 9000) * env(T, 0.08, 0.35, hold=0.15) * 0.5
    f = 600 + 900 * (x / T)
    glass = (np.sin(2 * np.pi * f * x) + 0.6 * np.sin(2 * np.pi * f * 1.5 * x)
             + 0.3 * np.sin(2 * np.pi * f * 2.01 * x)) * env(T, 0.05, 0.3, hold=0.2) * 0.35
    return echoes(swish + glass, [0.09, 0.2], [0.3, 0.12], 6000)


def jet_thrust():
    # A short roaring hiss of thrust (played over and over while the jets fire).
    T = 0.28
    roar = noise(T, 120, 2200) * env(T, 0.02, 0.2, hold=0.08) * 0.8
    hiss = noise(T, 2500, 9000) * env(T, 0.01, 0.12, hold=0.05) * 0.35
    return roar + hiss


def chute_open():
    # A canopy snapping open: a whoosh and a fabric snap.
    T = 0.7
    whoosh = sweep_noise(T, 800, 4000) * env(T, 0.05, 0.3, hold=0.1) * 0.5
    snap = at(T, 0.18, click(0.08, 300, 3000, 0.01) * 0.9 + thump(0.08, 140, 0.03) * 0.6)
    return whoosh + snap


def silo_warning():
    # A falling two-tone alarm, twice: a supply running dry.
    T = 1.0
    out = np.zeros(len(seconds(T)))
    for k in range(4):
        d = 0.2
        xx = seconds(d)
        f = 880.0 if k % 2 == 0 else 587.0
        tone = square(np.full(len(xx), f), 0.5) * 0.35 + np.sin(2 * np.pi * f * xx) * 0.65
        out += at(T, k * 0.24, lowpass(tone, 5000) * env(d, 0.005, 0.12, hold=0.05))
    return out


def kit_down():
    # A device set down: a thud, two latches clacking, a short power-up chirp.
    T = 0.6
    out = thump(T, 90, 0.05, drop=0.5) * 0.9 + noise(T, 200, 2500) * env(T, 0.001, 0.03) * 0.4
    for t0 in (0.12, 0.2):
        out += at(T, t0, click(0.06, 1500, 7000, 0.004) * 0.7)
    d = 0.18
    xx = seconds(d)
    chirp = np.sin(2 * np.pi * (700 + 2200 * xx / d) * xx) * env(d, 0.01, 0.08, hold=0.05)
    out += at(T, 0.32, chirp * 0.4)
    return out


def hack_done():
    # Data chatter, then a two-note confirmation.
    T = 0.7
    out = np.zeros(len(seconds(T)))
    rng = np.random.default_rng(7)
    for k in range(10):
        d = 0.025
        xx = seconds(d)
        f = float(rng.choice([1800, 2400, 3000, 3600]))
        out += at(T, k * 0.028, square(np.full(len(xx), f), 0.5) * 0.25 * env(d, 0.002, 0.012))
    for t0, f in ((0.33, 1320), (0.45, 1980)):
        d = 0.16
        xx = seconds(d)
        tone = np.sin(2 * np.pi * f * xx) * 0.7 + np.sin(2 * np.pi * 2 * f * xx) * 0.15
        out += at(T, t0, tone * env(d, 0.004, 0.09, hold=0.03))
    return lowpass(out, 9000)




# ---------------------------------------------------------------- planetstrike: world and base


def vehicle_spawn():
    T = 1.9
    x = seconds(T)
    # A vehicle materialising on the pad: a sub swell and a rising noise sweep, a cloud of
    # glassy rings climbing, and a solid thump with a bright chord as it locks in.
    rise = np.clip(x / 1.1, 0, 1)
    sub = np.sin(2 * np.pi * phase(40 * 2 ** rise)) * (rise ** 2) * (x < 1.12) * 0.8
    sweep = sweep_noise(T, 200, 9000) * env(T, 0.6, 0.06, hold=1.05) * 0.3
    cloud = np.zeros(len(x))
    for k in range(12):
        t0 = 0.08 * k
        hz = 700 * 2 ** (k / 5)
        cloud += at(T, t0, ring(T - t0, [hz, hz * 1.5, hz * 2.01], [0.25, 0.18, 0.12], [0.4, 0.25, 0.15])) * 0.25
    land = at(T, 1.1, thump(T - 1.1, 70, 0.15, drop=0.5) * 1.3 + click(T - 1.1, 500, 8000, 0.006) * 0.8)
    chord = sum(at(T, 1.1, np.sin(2 * np.pi * hz * seconds(T - 1.1)) * env(T - 1.1, 0.005, 0.3))
                for hz in [784.0, 1174.7, 1568.0, 2349.3]) * 0.18
    return echoes(sub + sweep + cloud + land + chord, [0.09, 0.2], [0.3, 0.12], 6000)


def hydraulics(T, length):
    """A hydraulic ram running for `length` s: a pumping hiss and a low strained hum."""
    x = seconds(T)
    gate = env(T, 0.03, 0.06, hold=length)
    hiss = noise(T, 1500, 7000) * (0.7 + 0.3 * np.sin(2 * np.pi * 9 * x)) * 0.3
    hum = lowpass(saw(np.full(len(x), 75.0) * (1 + 0.15 * x / T)), 600) * 0.4
    return (hiss + hum) * gate


def deploy():
    T = 2.2
    x = seconds(T)
    # The bus deploying: a clunk, hydraulic legs extending, a heavy locking thud, then the
    # spawn systems powering up (a rising whine into a bright chord).
    clunk = thump(0.3, 110, 0.05) * 1.0 + click(0.3, 300, 4000, 0.008) * 0.6
    legs = hydraulics(0.8, 0.65)
    lock = thump(0.4, 70, 0.1, drop=0.5) * 1.3 + click(0.4, 300, 3000, 0.01) * 0.7 + ring(0.4, [520, 870], [0.06, 0.04], [0.3, 0.2])
    p = np.clip((x - 0.95) / 0.8, 0, 1)
    f = 140 * 6 ** p
    power = lowpass(saw(f) * 0.5 + np.sin(2 * np.pi * phase(f * 2)) * 0.4, 4000) * (p > 0) * (0.2 + 0.8 * p) * np.where(x < 1.75, 1.0, np.exp(-(x - 1.75) / 0.05)) * 0.35
    chord = sum(at(T, 1.75, np.sin(2 * np.pi * hz * seconds(T - 1.75)) * env(T - 1.75, 0.005, 0.2))
                for hz in [523.3, 784.0, 1046.5, 1568.0]) * 0.22
    return echoes(at(T, 0.0, clunk) + at(T, 0.08, legs) + at(T, 0.85, lock) + power + chord, [0.09], [0.2], 5000)


def undeploy():
    T = 1.8
    x = seconds(T)
    # Powering down (a falling whine), the legs retracting with a hiss and a final clunk.
    f = 900 * np.exp(-x / 0.25) + 60
    down = lowpass(saw(f) * 0.5 + np.sin(2 * np.pi * phase(f * 0.5)) * 0.4, 3000) * env(T, 0.005, 0.25, hold=0.15) * 0.4
    legs = hydraulics(0.75, 0.6)
    clunk = thump(0.5, 85, 0.08, drop=0.5) * 1.2 + click(0.5, 250, 3000, 0.01) * 0.6
    return down + at(T, 0.45, legs) + at(T, 1.25, clunk)


def cloak_on():
    T = 1.0
    x = seconds(T)
    # Fading out of sight: a cloud of glassy rings sliding down, a falling hiss, a phasey wobble.
    cloud = np.zeros(len(x))
    for k in range(10):
        t0 = 0.05 * k
        hz = 4200 * 2 ** (-k / 4)
        cloud += at(T, t0, ring(T - t0, [hz, hz * 1.5], [0.18, 0.12], [0.4, 0.2])) * 0.3
    hiss = sweep_noise(T, 10000, 600, lo=300) * env(T, 0.02, 0.3, hold=0.15) * (0.7 + 0.3 * np.sin(2 * np.pi * 11 * x)) * 0.35
    f = 600 * np.exp(-x / 0.3) + 80
    tone = np.sin(2 * np.pi * phase(f) + 0.6 * np.sin(2 * np.pi * 7 * x)) * env(T, 0.01, 0.3, hold=0.1) * 0.2
    return echoes(cloud + hiss + tone, [0.08, 0.17], [0.3, 0.15], 6000)


def rearm_loop():
    T = 2.0
    x = seconds(T)
    # Rearming on a pad: a charging hum, a geared servo whine gliding up and down, ratcheting
    # ticks and a pulsing electric fizz.
    hum = (lowpass(saw(const(T, 110)), 1200) * 0.4 + np.sin(2 * np.pi * 220 * x) * 0.3) * (0.8 + 0.2 * np.sin(2 * np.pi * x))
    sf = cyc(T, 700) * (1 + 0.12 * np.sin(2 * np.pi * 0.5 * x))
    whine = np.sin(2 * np.pi * phase(sf)) + 0.4 * np.sin(2 * np.pi * phase(sf * 1.5))
    gears = 0.7 + 0.3 * np.sign(np.sin(2 * np.pi * 24 * x))
    whine = lowpass(whine * gears, 4000) * 0.12
    ticks = sum(loop_at(T, i * 0.25, click(0.05, 1500, 7000, 0.002) + ring(0.05, [2100, 3300], [0.01, 0.007], [0.3, 0.2]))
                for i in range(8)) * 0.15
    fizz = noise(T, 2500, 8000) * pulses(T, 8, 6) * 0.08
    return hum + whine + ticks + fizz


def warp():
    T = 1.6
    x = seconds(T)
    # A warp gate: a whoosh sucked upwards with a rising FM tone, a pop as it snaps through
    # and a falling shimmer on the far side.
    rise = np.clip(x / 0.85, 0, 1)
    gate = np.where(x < 0.85, rise ** 1.5, np.exp(-(x - 0.85) / 0.02))
    suck = sweep_noise(T, 300, 11000) * gate * 0.5
    f = 150 * 12 ** rise
    tone = np.sin(2 * np.pi * phase(f) + 2.0 * np.sin(2 * np.pi * phase(f * 0.501))) * gate * 0.3
    pop = at(T, 0.85, thump(0.5, 90, 0.08, drop=0.6) * 1.3 + click(0.5, 800, 12000, 0.006) * 1.0)
    tail = at(T, 0.85, sweep_noise(T - 0.85, 9000, 500, lo=300) * env(T - 0.85, 0.005, 0.25) * 0.35)
    chime = at(T, 0.86, bell(T - 0.86, 1568.0, 0.3) + bell(T - 0.86, 2349.3, 0.25) * 0.6) * 0.15
    return echoes(suck + tone + pop + tail + chime, [0.11, 0.25], [0.3, 0.15], 5000)


def warpgate_hum():
    T = 3.0
    x = seconds(T)
    # A deep, resonant drone with slowly beating partials (multiples of 1/3 Hz) and a
    # breathing resonance; seamless over 3 s.
    breathe = 0.7 + 0.3 * np.sin(2 * np.pi * x / 3.0)
    drone = (np.sin(2 * np.pi * 41 * x) + 0.7 * np.sin(2 * np.pi * (247 / 3) * x)
             + 0.4 * np.sin(2 * np.pi * 123 * x) + 0.3 * np.sin(2 * np.pi * (494 / 3) * x)
             + 0.15 * np.sin(2 * np.pi * (740 / 3) * x)) * breathe
    res = lowpass(noise(T, 60, 500), 300) * (0.6 + 0.4 * np.sin(2 * np.pi * 2 * x / 3.0 + 1.0)) * 0.3
    halo = (np.sin(2 * np.pi * 660 * x) + 0.6 * np.sin(2 * np.pi * (2971 / 3) * x)) * (0.5 + 0.5 * np.sin(2 * np.pi * x)) * 0.05
    return drone * 0.5 + res + halo


def forcefield_buzz():
    T = 1.5
    x = seconds(T)
    # A forcefield's electric hum: a buzzy 100 Hz drone with a flutter, a thin whine and
    # occasional crackles, every component a whole number of cycles in 1.5 s.
    buzz = lowpass(square(const(T, 100), 0.3) * 0.5 + saw(const(T, 200)) * 0.3, 2500)
    buzz *= 0.75 + 0.25 * np.sin(2 * np.pi * 30 * x)
    whine = np.sin(2 * np.pi * 3000 * x) * 0.04 * (0.6 + 0.4 * np.sin(2 * np.pi * 2 * x))
    sparks = crackle(T, 25, 2000, 9000) * 0.3
    return buzz + whine + sparks


def forcefield_deny():
    T = 0.4
    x = seconds(T)
    # Blocked by a forcefield: a heavy thunk, a sizzling zap and a short buzz.
    thunk = thump(T, 100, 0.05, drop=0.5) * 1.2 + click(T, 300, 4000, 0.005) * 0.6
    zap = crackle(T, 3000, 1500, 10000) * env(T, 0.001, 0.05) * 0.8
    bf = 150 + 40 * np.exp(-x / 0.05)
    buzz = lowpass(square(bf, 0.4) * 0.6 + saw(bf * 2.01) * 0.4, 3000) * env(T, 0.002, 0.08, hold=0.05) * 0.5
    return thunk + zap + buzz


def beep(T, hz, length, duty=0.25):
    """A UI beep: a sine with a little square for edge, `length` s."""
    p = np.full(int(T * RATE), float(hz))
    tone = np.sin(2 * np.pi * phase(p)) * 0.75 + square(p, duty) * 0.12
    return tone * env(T, 0.002, length * 0.5, hold=length * 0.5)


def terminal_open():
    T = 0.4
    click_ = click(T, 1500, 8000, 0.002) * 0.6
    tones = at(T, 0.01, beep(0.15, 880, 0.05)) + at(T, 0.075, beep(0.2, 1318.5, 0.07))
    return echoes(click_ + lowpass(tones, 8000), [0.06], [0.2], 6000)


def terminal_select():
    T = 0.12
    return lowpass(beep(T, 1760, 0.03), 9000) + click(T, 2000, 9000, 0.0015) * 0.3


def hack_start():
    T = 1.1
    x = seconds(T)
    # Starting a hack: a warbling stream of data bleeps climbing in pitch over a rising
    # two-tone alarm.
    out = np.zeros(len(x))
    step = 0.05
    for i in range(18):
        base = 500 * 2 ** (i / 12)
        hz = base * [1.0, 1.5, 2.0, 1.25, 1.75][rng.integers(5)]
        d = step * 1.2
        out += at(T, step * i, beep(d, hz, step * 0.7, 0.3))
    alarm_f = np.where((x * 8.0) % 1.0 < 0.5, 620.0, 830.0) * (1 + 0.4 * x / T)
    alarm = lowpass(saw(alarm_f), 2500) * env(T, 0.02, 0.15, hold=0.85) * 0.15
    return echoes(lowpass(out, 7000) * 0.5 + alarm, [0.07], [0.2], 5000)


def hack_tick():
    T = 0.1
    x = seconds(T)
    # A soft, short blip (heard every second while a hack runs).
    f = 1200 * (1 + 0.15 * np.exp(-x / 0.008))
    return np.sin(2 * np.pi * phase(f)) * env(T, 0.002, 0.025) + click(T, 3000, 8000, 0.001) * 0.1


def hack_stopped():
    T = 0.8
    x = seconds(T)
    # A hack cancelled: three falling notes over a power-down whine.
    notes = sum(at(T, 0.08 * i, beep(0.2, hz, 0.07, 0.4)) for i, hz in enumerate([1046.5, 784.0, 523.3]))
    f = 700 * np.exp(-x / 0.18) + 70
    down = lowpass(saw(f), 2500) * env(T, 0.005, 0.2, hold=0.1) * 0.25
    return echoes(lowpass(notes, 6000) * 0.5 + down, [0.08], [0.2], 4000)


def base_captured():
    T = 2.1
    x = seconds(T)
    # A triumphant stinger: a short G chord leaping to a held bright C chord, bells and a boom.
    def chord(t0, hzs, decay, hold):
        return sum(at(T, t0, brass(hz, T - t0, 0.015, decay, hold, 6.0)) for hz in hzs)
    fan = chord(0.0, [392.0, 493.88, 587.33], 0.08, 0.1) + chord(0.16, [523.25, 659.25, 783.99, 1046.5], 0.7, 0.45)
    pedal = at(T, 0.16, np.sin(2 * np.pi * 65.4 * seconds(T - 0.16)) * env(T - 0.16, 0.03, 0.7, 0.3)) * 0.5
    boom = at(T, 0.16, thump(T - 0.16, 60, 0.25, drop=0.4)) * 0.8
    bells = sum(at(T, 0.16 + 0.08 * i, bell(T, hz, 0.5)) for i, hz in enumerate([1568.0, 2093.0, 2637.0])) * 0.15
    shimmer = noise(T, 5000, 13000) * at(T, 0.16, env(T, 0.1, 0.5, hold=0.2)) * (0.5 + 0.5 * np.sin(2 * np.pi * 17 * x)) * 0.06
    return echoes(fan * 0.35 + pedal + boom + bells + shimmer, [0.12, 0.26], [0.3, 0.15], 5000)


def base_lost():
    T = 2.1
    x = seconds(T)
    # An ominous stinger: a low minor chord sinking a semitone to a dark diminished one,
    # a sub boom and a growling rumble.
    def chord(t0, hzs, decay, hold):
        return sum(at(T, t0, brass(hz, T - t0, 0.03, decay, hold, 3.0)) for hz in hzs)
    fan = chord(0.0, [164.81, 196.0, 246.94], 0.12, 0.2) + chord(0.32, [155.56, 185.0, 220.0, 311.13], 0.8, 0.5)
    sub = at(T, 0.32, np.sin(2 * np.pi * phase(np.full(int((T - 0.32) * RATE), 38.9))) * env(T - 0.32, 0.05, 0.8, 0.3)) * 0.8
    boom = thump(T, 55, 0.35, drop=0.4) * 1.0 + at(T, 0.32, thump(T - 0.32, 45, 0.4, drop=0.4)) * 0.9
    rumble = lowpass(noise(T, 20, 300), 120) * env(T, 0.1, 0.8, hold=0.3) * 0.4
    toll = at(T, 0.32, bell(T - 0.32, 466.2, 0.6)) * 0.12
    return echoes(fan * 0.45 + sub + boom + rumble + toll, [0.18, 0.4], [0.3, 0.15], 2500)


def continent_captured():
    T = 3.6
    x = seconds(T)
    # The big fanfare: an impact and sweep, a climbing triad of brass chords, a held major
    # chord with a fifth on top, a sub pedal, bells and a long shimmer.
    boom = thump(T, 55, 0.5, drop=0.55) * 1.5 + lowpass(noise(T, 20, 400), 180) * env(T, 0.002, 0.3) * 0.7
    crack = click(T, 1500, 12000, 0.012) * 0.5
    sweep = at(T, 0.0, sweep_noise(0.5, 300, 9000) * env(0.5, 0.35, 0.05, hold=0.45)) * 0.3
    fan = np.zeros(len(x))
    steps = [(0.28, [261.6, 329.6, 392.0]), (0.46, [293.7, 370.0, 440.0]), (0.64, [329.6, 415.3, 493.9])]
    for start, hzs in steps:
        for hz in hzs:
            fan += at(T, start, brass(hz, 0.4, 0.01, 0.1, 0.1, 6.0)) * 0.6
    for i, hz in enumerate([349.2, 440.0, 523.3, 698.5, 1046.5]):
        fan += at(T, 0.84, brass(hz, T - 0.84, 0.03, 1.1, 0.8, 7.0)) * (0.7 if i < 4 else 0.45)
    pedal = at(T, 0.84, np.sin(2 * np.pi * 87.3 * seconds(T - 0.84)) * env(T - 0.84, 0.05, 1.2, 0.7)) * 0.6
    bells = sum(at(T, 0.84 + 0.09 * i, bell(T, hz, 0.8)) for i, hz in enumerate([1396.9, 1760.0, 2093.0, 2793.8])) * 0.18
    shimmer = noise(T, 5000, 14000) * (0.5 + 0.5 * np.sin(2 * np.pi * 17 * x)) * at(T, 0.84, env(T, 0.2, 1.1, hold=0.4)) * 0.08
    return echoes(boom + crack + sweep + fan * 0.6 + pedal + bells + shimmer, [0.13, 0.27, 0.41], [0.35, 0.2, 0.1], 4000)


def alert():
    T = 1.25
    x = seconds(T)
    # A base-under-attack klaxon: two whooping pulses of a buzzy rising horn.
    out = np.zeros(len(x))
    for t0 in (0.0, 0.55):
        d = 0.45
        xx = seconds(d)
        f = 360 + 200 * (xx / d) ** 0.7
        horn = saw(f) * 0.6 + square(f * 1.005, 0.4) * 0.4 + saw(f * 1.5) * 0.25
        gate = np.clip(xx / 0.015, 0, 1) * np.clip((d - xx) / 0.04, 0, 1)
        out += at(T, t0, lowpass(horn, 2200) * gate)
    return echoes(out, [0.1, 0.22], [0.25, 0.1], 2000)


# ---------------------------------------------------------------- planetstrike: player

def heal():
    T = 0.6
    x = seconds(T)
    # A soft healing chime: three rising sine notes with a light sparkle.
    notes = sum(at(T, 0.06 * i, np.sin(2 * np.pi * hz * seconds(T - 0.06 * i)) * env(T - 0.06 * i, 0.004, 0.15))
                for i, hz in enumerate([987.8, 1318.5, 1760.0]))
    sparkle = highpass(noise(T, 5000, 12000), 4000) * env(T, 0.02, 0.15) * (np.sin(2 * np.pi * 29 * x) > 0) * 0.15
    return echoes(notes * 0.5 + sparkle, [0.08], [0.25], 7000)


def fall_thud():
    T = 0.4
    # A body landing hard: a deep thud, the scuff of the impact and kit rattling.
    thud = thump(T, 60, 0.07, drop=0.5) * 1.3 + thump(T, 120, 0.03, drop=0.5) * 0.6
    scuff = lowpass(noise(T, 80, 2000), 900) * env(T, 0.002, 0.04) * 0.6
    rattle = at(T, 0.02, crackle(0.15, 300, 2000, 7000) * env(0.15, 0.002, 0.04) * 0.3)
    gear = at(T, 0.035, ring(0.1, [1900, 2800], [0.015, 0.01], [0.3, 0.2]) + click(0.1, 1500, 6000, 0.003) * 0.3)
    return thud + scuff + rattle + gear * 0.5


def hurt():
    T = 0.3
    x = seconds(T)
    # Taking damage: a punchy thud and a short buzzing zap falling.
    thud = thump(T, 110, 0.04, drop=0.5) * 1.0 + thump(T, 65, 0.07, drop=0.35) * 0.8
    f = (180 + 900 * np.exp(-x / 0.04)) * (1 + 0.06 * np.sin(2 * np.pi * 27 * x))
    zap = lowpass(saw(f) * 0.6 + square(f * 1.01, 0.35) * 0.4, 4000) * env(T, 0.001, 0.05, hold=0.02) * 0.6
    crack = click(T, 2000, 9000, 0.002) * 0.6
    return thud + zap + crack


def death():
    T = 1.5
    x = seconds(T)
    # Killed: a hit, a long falling whine with widening vibrato, sputtering static and a
    # low boom, ending in a dying fizz.
    hit = click(T, 1500, 10000, 0.004) * 1.0 + thump(T, 90, 0.06, drop=0.5) * 0.9
    vib = 1 + (0.01 + 0.08 * x / T) * np.sin(2 * np.pi * (8 + 5 * x) * x)
    f = 1100 * np.exp(-x / 0.4) * vib + 40
    whine = (np.sin(2 * np.pi * phase(f)) + 0.4 * lowpass(saw(f), 2500)) * env(T, 0.005, 0.4, hold=0.5) * 0.6
    static = crackle(T, 1200, 1200, 7000) * env(T, 0.005, 0.4, hold=0.2) * 0.5
    boom = at(T, 0.03, thump(1.0, 60, 0.35, drop=0.45)) * 1.0
    fizz = at(T, 0.7, noise(0.6, 2000, 7000) * env(0.6, 0.02, 0.15) * 0.1)
    return hit + whine + static + boom + fizz


def spawn():
    T = 1.0
    x = seconds(T)
    # Respawning: a rising shimmer sweep, a pentatonic arpeggio climbing and a soft landing.
    sweep = highpass(sweep_noise(T, 400, 11000), 300) * np.sin(np.pi * np.clip(x / 0.75, 0, 1)) ** 2 * 0.25
    notes = [587.3, 784.0, 880.0, 1174.7, 1568.0]
    arp = sum(at(T, 0.06 * i, (np.sin(2 * np.pi * hz * seconds(T - 0.06 * i))
                               + 0.25 * np.sin(4 * np.pi * hz * seconds(T - 0.06 * i))) * env(T - 0.06 * i, 0.003, 0.14 + 0.03 * i))
              for i, hz in enumerate(notes)) * 0.4
    f = 60 * 2 ** np.clip(x / 0.35, 0, 1)
    swell = np.sin(2 * np.pi * phase(f)) * env(T, 0.15, 0.08, hold=0.25) * 0.5
    land = at(T, 0.38, thump(0.3, 90, 0.06, drop=0.4) * 0.7)
    return echoes(sweep + arp + swell + land, [0.08, 0.16], [0.3, 0.12], 5000)


def hitmarker():
    T = 0.08
    # A crisp tick when your shot connects.
    tick = click(T, 3000, 12000, 0.0012) * 1.0
    ping = np.sin(2 * np.pi * 3300 * seconds(T)) * env(T, 0.0003, 0.012) * 0.6
    return tick + ping


def kill_confirm():
    T = 0.4
    # A kill confirmed: two quick square-ish tones a fourth apart.
    a = beep(0.12, 1318.5, 0.06, 0.25)
    b = beep(0.25, 1760.0, 0.12, 0.25)
    out = at(T, 0.0, a) + at(T, 0.075, b)
    return echoes(lowpass(out, 8000), [0.06], [0.22], 6000)


# ---------------------------------------------------------------- planetstrike: weapons (layered)
#
# Every shot is layers (a crack, a body, a sub, the mechanism) glued by `finish`: saturated, its
# attacks raised, set in a little room. Written "peak" (see `write`), so tails cost no punch.

def finish(x, drive=2.5, bias=0.05, attack=0.8, sustain=-0.2, comp=None, room=None):
    """A shot's layers glued: saturated (`drive`, `bias`), the attacks raised and what follows
    tightened (`attack`, `sustain`), compressed (`comp`: threshold dB, ratio) and set in a
    `room` (size, decay s, mix), if any."""
    x = transient(saturate(x, drive, bias), attack, sustain)
    if comp:
        x = compress(x, *comp)
    if room:
        x = reverb(x, size=room[0], decay=room[1], mix=room[2])
    return x


def dive(x, f0, f1, tau):
    """A frequency falling (or rising) from `f0` to `f1` Hz with time constant `tau` s."""
    return f1 + (f0 - f1) * np.exp(-x / tau)


def fm(f, depth, ratio=0.5, tau=0.03):
    """A sine at `f` (an array, Hz), frequency-modulated by one at `ratio` times it, `depth`
    dying away over `tau` s: an energy bolt's zap."""
    x = np.arange(len(f)) / RATE
    return np.sin(2 * np.pi * phase(f) + depth * np.exp(-x / tau) * np.sin(2 * np.pi * phase(f * ratio)))


def tone(f):
    return np.sin(2 * np.pi * phase(f))


# ---- infantry

def rifle_fire():
    T = 0.3
    x = seconds(T)
    # The pulse rifle: a fast FM zap diving from 2.3 kHz, a crack, a punch with a sub under
    # it, the bolt cycling.
    f = dive(x, 2300, 420, 0.018)
    zap = (fm(f, 1.6) + 0.3 * square(f * 0.5, 0.3)) * env(T, 0.0004, 0.028, hold=0.008)
    crack = click(T, 2500, 13000, 0.0018) * 1.3
    punch = thump(T, 125, 0.022, drop=0.45) * 1.1
    low = sub(T, 68, 0.035) * 0.35
    bolt = at(T, 0.04, mech(0.08, "bolt")) * 0.1
    return finish(lowpass(zap * 0.95, 9000) + crack + punch + low + bolt, 2.5, room=(0.4, 0.16, 0.12))


def ember_fire():
    T = 0.5
    x = seconds(T)
    # Red's rifle: low, slow, boomy: a growling square diving from 950 Hz, a blast of noise, a
    # deep thump and sub, embers crackling after it, a heavy breech.
    f = dive(x, 950, 150, 0.035)
    growl = lowpass(square(f, 0.35) * 0.6 + saw(f * 0.5) * 0.5, 3200)
    growl *= (1 + 0.4 * np.sin(2 * np.pi * 62 * x)) * env(T, 0.0006, 0.06, hold=0.012)
    crack = click(T, 1500, 9000, 0.003) * 1.3
    blast = lowpass(noise(T, 120, 4000), 2200) * env(T, 0.0008, 0.03) * 0.8
    punch = thump(T, 85, 0.04, drop=0.5) * 1.1
    low = sub(T, 52, 0.08) * 0.55
    embers = crackle(T, 900, 2000, 9000) * env(T, 0.01, 0.07, hold=0.02) * 0.25
    clunk = at(T, 0.07, mech(0.12, "clunk")) * 0.1
    return finish(growl * 1.0 + crack + blast + punch + low + embers + clunk, 3.5, bias=0.12, attack=0.7,
                  room=(0.7, 0.28, 0.16))


def volt_fire():
    T = 0.32
    x = seconds(T)
    # Blue's rifle: a tight electric snap: a very short bright crack, a high zap from 5.6 kHz,
    # the coils ringing, an arc's crackle, a small hard punch.
    f = dive(x, 5600, 1500, 0.012)
    zap = fm(f, 2.2, 1.5, 0.02) * env(T, 0.0003, 0.02, hold=0.004)
    coil = ring(T, [3100, 4650, 6980], [0.05, 0.035, 0.02], [0.5, 0.35, 0.2]) * 0.3
    crack = click(T, 4000, 16000, 0.001) * 1.2
    arc = crackle(T, 3000, 3500, 14000) * env(T, 0.0005, 0.018) * 0.5
    punch = thump(T, 165, 0.014, drop=0.4) * 0.9
    low = sub(T, 82, 0.022) * 0.35
    latch = at(T, 0.035, mech(0.05, "latch")) * 0.08
    return finish(zap * 0.6 + coil + crack + arc + punch + low + latch, 2.0, attack=1.1, sustain=-0.35,
                  room=(0.35, 0.14, 0.1))


def hive_fire():
    T = 0.17
    x = seconds(T)
    # Green's rifle: thin, fast, chittering: three ring-modulated chirps a hundredth of a
    # second apart over an insect's buzz, a light tap.
    out = np.zeros(len(x))
    for i, (t0, g) in enumerate(((0.0, 1.0), (0.011, 0.75), (0.023, 0.5))):
        d = 0.05
        xx = seconds(d)
        chirp = square(dive(xx, 2500 - 250 * i, 1250, 0.006), 0.25) * np.sin(2 * np.pi * 310 * xx)
        out += at(T, t0, lowpass(chirp * env(d, 0.0003, 0.007), 7000) * g)
    crack = click(T, 3500, 12000, 0.0012) * 0.7
    punch = thump(T, 200, 0.012, drop=0.4) * 0.8
    buzz = lowpass(saw(np.full(len(x), 155.0)), 2500) * env(T, 0.001, 0.02) * 0.25
    return finish(out * 0.8 + crack + punch + buzz, 2.2, attack=0.9, sustain=-0.3, room=(0.3, 0.1, 0.08))


def scatter_fire():
    T = 0.85
    x = seconds(T)
    # The scatter gun: a wide blast of noise and a spray of pellets over a deep thump and sub,
    # hard driven, then the pump racked.
    crack = click(T, 1200, 12000, 0.004) * 1.9
    blast = lowpass(noise(T, 90, 7000), 3800) * env(T, 0.0006, 0.045) * 1.5
    spray = crackle(T, 2500, 2500, 12000) * env(T, 0.001, 0.025) * 0.8
    zap = lowpass(saw(dive(x, 1500, 200, 0.02)), 4500) * env(T, 0.0006, 0.035) * 0.35
    punch = thump(T, 78, 0.06, drop=0.5) * 1.2
    low = sub(T, 48, 0.11) * 0.55
    shot = finish(crack + blast + spray + zap + punch + low, 4.0, bias=0.1, attack=0.7, sustain=-0.1,
                  comp=(-14, 3), room=(1.0, 0.45, 0.2))
    return shot + at(T, 0.3, mech(0.24, "slide")) * 0.22 * np.max(np.abs(shot))


def lance_crack():
    T = 1.7
    x = seconds(T)
    # The marksman's lance: a supersonic crack, a searing zap from 8.5 kHz, a punch and sub,
    # a long tail rolling off the hills, the bolt worked after it.
    crack = click(T, 2500, 17000, 0.0012) * 2.2 + noise(T, 4000, 15000) * env(T, 0.0003, 0.008) * 0.7
    f = dive(x, 8500, 750, 0.035)
    zap = (tone(f) + 0.3 * saw(f * 0.5)) * env(T, 0.0004, 0.09, hold=0.008) * 0.7
    punch = thump(T, 72, 0.06, drop=0.5) * 0.9
    low = sub(T, 44, 0.12) * 0.45
    sear = noise(T, 5000, 14000) * env(T, 0.01, 0.12) * 0.08
    dry = finish(lowpass(crack + zap + punch + low + sear, 15000), 3.0, attack=1.2, sustain=-0.2, comp=(-16, 3))
    wet = slapback(reverb(dry, size=1.8, decay=1.1, mix=0.3, predelay=0.03), 0.21, 0.22, 3000)
    return wet + at(T, 0.55, mech(0.3, "slide")) * 0.1 * np.max(np.abs(wet))


def striker_fire():
    T = 1.3
    x = seconds(T)
    # A shoulder rocket: the tube's hollow thunk, a sharp ignition and a whoosh tearing away.
    thunk = thump(T, 92, 0.05, drop=0.5) * 1.3 + click(T, 300, 4500, 0.006) * 0.9 + sub(T, 50, 0.1) * 0.5
    tube = ring(T, [310, 505, 790], [0.06, 0.04, 0.03], [0.5, 0.35, 0.2]) * 0.3
    ignite = at(T, 0.03, click(0.1, 2000, 12000, 0.004) * 0.9)
    whoosh = sweep_noise(T, 2500, 9000, lo=300) * np.sin(np.pi * np.clip(x / 0.35, 0, 1)) ** 2
    whoosh += sweep_noise(T, 9000, 1500, lo=300) * np.where(x > 0.175, env(T, 0.001, 0.28), 0) * 0.8
    burn = lowpass(saw(dive(x, 700, 200, 0.4)), 2500) * env(T, 0.03, 0.25, hold=0.1) * 0.2
    return finish(thunk + tube + ignite + highpass(whoosh, 250) * 0.45 + burn, 2.8, attack=0.6, sustain=0.0,
                  room=(1.0, 0.5, 0.18))


def skyhook_fire():
    T = 1.25
    x = seconds(T)
    # The anti-air launcher: the seeker's chirp, a pneumatic pop throwing the missile out, its
    # motor lighting a moment later and screaming away with a rising whistle.
    pop = thump(T, 150, 0.025, drop=0.5) * 0.9 + click(T, 1500, 10000, 0.003) + sub(T, 60, 0.05) * 0.5
    puff = noise(T, 800, 6000) * env(T, 0.002, 0.03) * 0.5
    d = T - 0.06
    motor = at(T, 0.06, sweep_noise(d, 11000, 2500, lo=600) * env(d, 0.02, 0.4, hold=0.15)) * 0.6
    f = 1100 + 1300 * (1 - np.exp(-x / 0.22))
    whistle = lowpass(saw(f) * 0.4 + tone(f * 2) * 0.3, 6000) * env(T, 0.05, 0.35, hold=0.12) * 0.28
    shot = finish(pop + puff + motor + whistle, 2.2, attack=0.7, sustain=0.0, room=(0.9, 0.45, 0.16))
    return shot + at(T, 0.0, beep(0.06, 3520, 0.03)) * 0.12 * np.max(np.abs(shot))


def pistol_fire():
    T = 0.3
    x = seconds(T)
    # The sidearm: a snappy, higher zap, a crack and a small punch, the slide.
    zap = fm(dive(x, 3100, 700, 0.02), 1.0) * env(T, 0.0004, 0.03, hold=0.006)
    crack = click(T, 3000, 13000, 0.0015) * 0.9
    punch = thump(T, 150, 0.018, drop=0.45) * 0.9
    low = sub(T, 78, 0.025) * 0.4
    slide = at(T, 0.05, mech(0.09, "slide")) * 0.14
    return finish(zap * 0.6 + crack + punch + low + slide, 2.2, attack=1.0, sustain=-0.3, room=(0.35, 0.14, 0.12))


def medgun_loop():
    T = 2.0
    x = seconds(T)
    # The heal beam (a loop): a warm fifth and octave pulsing five times a second over a soft
    # low body, an airy shimmer and bell sparkles scattered round the loop.
    trem = 0.7 + 0.3 * np.sin(2 * np.pi * cyc(T, 5.0) * x)
    hum = (np.sin(2 * np.pi * cyc(T, 330) * x) + 0.6 * np.sin(2 * np.pi * cyc(T, 495.5) * x)
           + 0.4 * np.sin(2 * np.pi * cyc(T, 660) * x) + 0.2 * np.sin(2 * np.pi * cyc(T, 990.5) * x)) * trem
    body = np.sin(2 * np.pi * cyc(T, 110) * x) * (0.8 + 0.2 * np.sin(2 * np.pi * cyc(T, 2.5) * x))
    air = noise(T, 3000, 9000) * (0.6 + 0.4 * np.sin(2 * np.pi * cyc(T, 2.0) * x)) * 0.07
    notes = [2093.0, 2349.3, 2637.0, 3136.0, 3520.0]
    sparkle = np.zeros(len(x))
    for i in range(10):
        hz = notes[rng.integers(len(notes))]
        sparkle += loop_at(T, i * 0.2 + 0.05 * rng.random(), bell(0.4, hz, 0.07) * (0.5 + 0.5 * rng.random()))
    return loop_echoes(hum * 0.25 + body * 0.2 + air + sparkle * 0.12, [0.11], [0.25], 6000)


def repair_loop():
    T = 2.0
    x = seconds(T)
    # The repair beam (a loop): a welder's arc: a buzzing mains hum, sparks crackling in
    # bursts, a thin servo whine and hiss.
    f = const(T, 100)
    buzz = lowpass(square(f, 0.3) * 0.5 + saw(f * 2) * 0.4, 2600) * (0.75 + 0.25 * pulses(T, 50, 2))
    arc = crackle(T, 500, 2500, 12000) * (0.4 + 0.6 * pulses(T, 7, 3))
    whine = np.sin(2 * np.pi * cyc(T, 1480) * x) * 0.08 * (0.7 + 0.3 * np.sin(2 * np.pi * cyc(T, 3) * x))
    hiss = noise(T, 4000, 11000) * 0.05
    return buzz * 0.3 + arc * 0.35 + whine + hiss


# ---- MAX arms

def pounder_fire():
    T = 0.15
    # A MAX's twin chainguns: two heavy barks a hundredth apart over a sub, the belt rattling.

    def one(d):
        xx = seconds(d)
        zap = lowpass(square(dive(xx, 900, 300, 0.008), 0.35), 4500) * env(d, 0.0004, 0.011) * 0.25
        return (click(d, 1600, 11000, 0.0016) * 1.4 + noise(d, 250, 4500) * env(d, 0.0004, 0.009) * 0.8
                + thump(d, 108, 0.02, drop=0.5) * 1.0 + zap)

    shot = one(T) + at(T, 0.011, one(T - 0.011)) * 0.65
    return finish(shot + sub(T, 62, 0.03) * 0.4 + mech(T, "belt") * 0.12, 3.2, bias=0.1, room=(0.3, 0.1, 0.08))


def mauler_fire():
    T = 1.0
    # A MAX's twin cannons: a double thud (the second barrel a twentieth later), a sub, the
    # tubes ringing, the breeches and a servo after.

    def one(d, hz):
        return (click(d, 1300, 12000, 0.004) * 1.7 + lowpass(noise(d, 120, 5500), 3200) * env(d, 0.0008, 0.035) * 0.9
                + thump(d, hz, 0.075, drop=0.5) * 1.2)

    shot = one(T, 88) + at(T, 0.048, one(T - 0.048, 76)) * 0.85
    tube = ring(T, [420, 690, 1130], [0.05, 0.035, 0.02], [0.4, 0.3, 0.2]) * 0.2
    shot = finish(shot + sub(T, 50, 0.13) * 0.55 + tube, 3.6, bias=0.1, attack=0.7, sustain=-0.1, comp=(-14, 3),
                  room=(0.9, 0.4, 0.18))
    k = np.max(np.abs(shot))
    return shot + (at(T, 0.2, mech(0.2, "clunk")) * 0.18 + at(T, 0.28, mech(0.18, "servo")) * 0.05) * k


def burster_fire():
    T = 0.24
    x = seconds(T)
    # A MAX's flak arms: a hollow "thoomp" out of a tube, twice.
    thoomp = tone(dive(x, 210, 95, 0.02)) * env(T, 0.0006, 0.035) * 1.3
    crack = click(T, 1500, 10000, 0.002)
    bark = noise(T, 400, 5000) * env(T, 0.0005, 0.012) * 0.7
    tube = ring(T, [640, 1010, 1620], [0.03, 0.02, 0.012], [0.5, 0.35, 0.2]) * 0.3
    shot = thoomp + crack + bark + tube
    return finish(shot + at(T, 0.014, shot) * 0.55 + sub(T, 70, 0.035) * 0.6, 3.0, room=(0.4, 0.14, 0.1))


# ---- vehicles and turrets

def boom_body(T, hz, decay, rumble_decay, gain=1.0):
    """An explosion's low end: a falling sub thump and a dull rumble."""
    return (thump(T, hz, decay, drop=0.5) * 1.3
            + lowpass(noise(T, 20, 400), 180) * env(T, 0.003, rumble_decay) * 0.7) * gain


def cannon_light():
    T = 1.4
    x = seconds(T)
    # The light tank's cannon: crack, blast, a thump with a sub under it, the barrel's clank;
    # hard driven, then out over the land and back off a wall; the breech after.
    crack = click(T, 1500, 13000, 0.004) * 2.0
    blast = lowpass(noise(T, 150, 6000), 4000) * env(T, 0.0008, 0.04) * 1.3
    body = boom_body(T, 80, 0.09, 0.22)
    low = sub(T, 46, 0.14) * 0.5
    clank = ring(T, [880, 1530, 2470], [0.06, 0.04, 0.025], [0.4, 0.3, 0.2]) * 0.18
    zap = lowpass(saw(dive(x, 1600, 180, 0.03)), 5000) * env(T, 0.0008, 0.04) * 0.3
    dry = finish(crack + blast + body + low + clank + zap, 3.5, bias=0.1, sustain=-0.05, comp=(-14, 3))
    wet = slapback(reverb(dry, size=1.4, decay=0.8, mix=0.25, predelay=0.025), 0.14, 0.2, 2000)
    return wet + at(T, 0.32, mech(0.2, "clunk")) * 0.1 * np.max(np.abs(wet))


def cannon_heavy():
    T = 2.6
    x = seconds(T)
    # The heavy tank's: deeper, longer: a 34 Hz sub, a rolling rumble, a long tail.
    crack = click(T, 1000, 12000, 0.007) * 2.2
    blast = lowpass(noise(T, 60, 5000), 2500) * env(T, 0.001, 0.07) * 1.5
    body = boom_body(T, 52, 0.25, 0.55, 1.2)
    low = sub(T, 34, 0.3) * 0.75
    rumble = lowpass(noise(T, 20, 200), 90) * env(T, 0.05, 0.6, hold=0.1) * 0.5
    clank = ring(T, [520, 910, 1480], [0.12, 0.08, 0.05], [0.4, 0.3, 0.2]) * 0.15
    zap = lowpass(saw(dive(x, 1100, 90, 0.05)), 3500) * env(T, 0.001, 0.08) * 0.3
    dry = finish(crack + blast + body + low + rumble + clank + zap, 4.0, bias=0.1, attack=0.7, sustain=0.0,
                 comp=(-14, 3))
    wet = slapback(reverb(dry, size=2.2, decay=1.6, mix=0.3, predelay=0.03), 0.23, 0.25, 1500)
    return wet + at(T, 0.5, mech(0.3, "clunk")) * 0.1 * np.max(np.abs(wet))


def rail_fire():
    T = 2.0
    x = seconds(T)
    # The railgun: a searing crack, a zap falling all the way from 9.5 kHz, a huge sub snap,
    # the rails humming down and sizzling after.
    crack = click(T, 3000, 18000, 0.001) * 2.4
    f = dive(x, 9500, 180, 0.05)
    zap = (fm(f, 2.5, 0.5, 0.08) + 0.4 * saw(f * 0.25)) * env(T, 0.0004, 0.12, hold=0.01) * 0.85
    snap = thump(T, 60, 0.09, drop=0.6) * 1.1
    low = sub(T, 36, 0.25) * 0.75
    sizzle = crackle(T, 1800, 3000, 14000) * env(T, 0.01, 0.3, hold=0.03) * 0.45
    hum = tone(dive(x, 240, 60, 0.3)) * (1 + 0.5 * np.sin(2 * np.pi * 50 * x)) * env(T, 0.002, 0.35) * 0.4
    dry = finish(lowpass(crack + zap + snap + low + sizzle + hum, 16000), 3.2, attack=1.2, sustain=-0.1,
                 comp=(-16, 3))
    return slapback(reverb(dry, size=2.0, decay=1.4, mix=0.3, predelay=0.03), 0.25, 0.2, 2500)


def autocannon_fire():
    T = 0.26
    # A vehicle's autocannon: a hard, dry, chesty thump and crack, the bolt.
    crack = click(T, 1400, 10000, 0.0025) * 1.6
    bark = noise(T, 200, 4000) * env(T, 0.0005, 0.014) * 0.9
    punch = thump(T, 118, 0.03, drop=0.5) * 1.2
    bolt = at(T, 0.05, mech(0.08, "bolt")) * 0.14
    return finish(crack + bark + punch + sub(T, 58, 0.045) * 0.5 + bolt, 3.4, bias=0.1, room=(0.5, 0.2, 0.12))


def flak_fire():
    T = 0.2
    x = seconds(T)
    # The AA gun: a short, hard pop: crack, bark, punch, a tiny zap, the barrel's ring.
    crack = click(T, 1500, 10000, 0.002)
    bark = noise(T, 400, 5000) * env(T, 0.0005, 0.012) * 0.7
    punch = thump(T, 150, 0.025, drop=0.5) * 1.2
    zap = lowpass(square(dive(x, 1300, 300, 0.012), 0.3), 5000) * env(T, 0.0005, 0.015) * 0.25
    barrel = ring(T, [900, 1450], [0.03, 0.02], [0.5, 0.3]) * 0.15
    return finish(crack + bark + punch + zap + barrel + sub(T, 75, 0.03) * 0.35, 2.8, room=(0.5, 0.16, 0.1))


def rocket_fire():
    T = 1.3
    x = seconds(T)
    # Ignition pop, then a hissing roar whooshing away, its tone falling as it leaves.
    pop = thump(T, 110, 0.05, drop=0.5) + click(T, 600, 8000, 0.005) * 0.9 + sub(T, 55, 0.08) * 0.6
    roar = sweep_noise(T, 9000, 1200, lo=200) * env(T, 0.01, 0.35, hold=0.1) * 0.6
    f = dive(x, 660, 140, 0.6)
    burn = lowpass(saw(f) * 0.5 + square(f * 1.01, 0.4) * 0.3, 2500) * env(T, 0.01, 0.3, hold=0.08) * 0.35
    rumble = lowpass(noise(T, 25, 300), 150) * env(T, 0.005, 0.25) * 0.5
    return finish(pop + roar + burn + rumble, 2.5, attack=0.6, sustain=0.0, room=(1.0, 0.5, 0.16))


def missile_fire():
    T = 1.2
    x = seconds(T)
    # A homing missile off its rail: a sharp release, a hissing motor, a rising whistle.
    release = click(T, 1500, 10000, 0.003) + thump(T, 140, 0.03, drop=0.5) * 0.7 + mech(T, "latch") * 0.3
    hiss = sweep_noise(T, 10000, 2500, lo=600) * env(T, 0.02, 0.4, hold=0.15) * 0.6
    f = 900 + 900 * (1 - np.exp(-x / 0.25))
    whistle = lowpass(saw(f) * 0.4 + tone(f * 2) * 0.3, 5000) * env(T, 0.03, 0.35, hold=0.1) * 0.25
    return finish(release + hiss + whistle, 2.0, attack=0.7, sustain=0.0, room=(0.9, 0.4, 0.14))


def air_cannon():
    T = 0.13
    x = seconds(T)
    # An aircraft's rotary cannon, one round of its "brrrt": a bright pop with a saw's buzz.
    crack = click(T, 2500, 12000, 0.0012) * 1.4
    punch = thump(T, 185, 0.013, drop=0.5)
    buzz = lowpass(saw(np.full(len(x), 370.0)), 5000) * env(T, 0.0003, 0.012) * 0.6
    zap = lowpass(square(dive(x, 1800, 500, 0.006), 0.3), 7000) * env(T, 0.0003, 0.01) * 0.3
    return finish(crack + punch + buzz + zap + sub(T, 92, 0.018) * 0.4, 2.6, attack=1.0, sustain=-0.3,
                  room=(0.3, 0.1, 0.08))


def chaingun():
    T = 0.12
    x = seconds(T)
    # A vehicle's machine gun: crack, bark and punch, the belt.
    crack = click(T, 1800, 11000, 0.0015) * 1.4
    bark = noise(T, 300, 5000) * env(T, 0.0004, 0.008) * 0.6
    punch = thump(T, 135, 0.016, drop=0.5) * 1.1
    zap = lowpass(square(dive(x, 1000, 350, 0.008), 0.35), 5000) * env(T, 0.0004, 0.01) * 0.25
    return finish(crack + bark + punch + zap + sub(T, 74, 0.02) * 0.3 + mech(T, "belt") * 0.1, 2.8,
                  room=(0.3, 0.1, 0.08))


def turret_wall_fire():
    T = 0.8
    x = seconds(T)
    # A base's wall turret: a heavy buzzing zap over a deep knock and sub, the mount's clank,
    # its servo settling.
    f = dive(x, 1100, 120, 0.05)
    zap = lowpass(square(f, 0.3) * 0.6 + saw(f * 1.5) * 0.4, 5000) * env(T, 0.001, 0.08)
    knock = thump(T, 95, 0.07, drop=0.55) * 1.2 + click(T, 300, 6000, 0.006) * 0.8
    clank = ring(T, [740, 1210, 1990], [0.06, 0.04, 0.025], [0.5, 0.35, 0.25]) * 0.3
    hiss = noise(T, 2000, 9000) * env(T, 0.002, 0.06) * 0.2
    shot = finish(zap + knock + clank + hiss + sub(T, 48, 0.1) * 0.5, 3.0, comp=(-14, 3), room=(1.0, 0.45, 0.2))
    return shot + at(T, 0.25, mech(0.2, "servo")) * 0.05 * np.max(np.abs(shot))


def spitfire_fire():
    T = 0.11
    x = seconds(T)
    # An engineer's little turret: a small, tinny, spitting zap.
    zap = square(dive(x, 2600, 900, 0.008), 0.2) * env(T, 0.0003, 0.012)
    crack = click(T, 4000, 13000, 0.0008) * 0.6
    punch = thump(T, 230, 0.01, drop=0.4) * 0.6
    tin = ring(T, [2900, 4300], [0.012, 0.008], [0.5, 0.3]) * 0.15
    return finish(lowpass(zap, 8000) * 0.6 + crack + punch + tin, 1.8, attack=0.9, sustain=-0.4)


# ---- shots heard from far off (played instead of the shot's own sound beyond its range)

def tail_small():
    T = 0.25
    src = click(T, 800, 6000, 0.003) + thump(T, 110, 0.03, drop=0.5) * 1.2 + noise(T, 200, 3000) * env(T, 0.001, 0.02)
    return far(src, tail=0.8, hi=1100)


def tail_medium():
    T = 0.6
    src = (click(T, 800, 8000, 0.005) + lowpass(noise(T, 100, 4000), 2500) * env(T, 0.001, 0.05)
           + boom_body(T, 78, 0.1, 0.25))
    return far(src, tail=1.5, hi=800)


def tail_heavy():
    T = 1.0
    src = (lowpass(noise(T, 60, 3000), 1800) * env(T, 0.001, 0.08) + boom_body(T, 48, 0.3, 0.6, 1.2)
           + sub(T, 32, 0.3))
    return far(src, tail=2.4, hi=550)


# ---- impacts

def impact_body(v):
    T = 0.28
    x = seconds(T)
    # A bolt into a body's armour: a dull slap and thud, the bolt's energy fizzing out.
    thud = thump(T, 150 - 20 * v, 0.03, drop=0.5) * 1.2
    slap = lowpass(noise(T, 250, 3000), 1800 + 400 * v) * env(T, 0.0006, 0.018) * 0.9
    fizz = lowpass(saw(dive(x, 1200 + 300 * v, 300, 0.03)), 5000) * env(T, 0.001, 0.04) * 0.3
    sizzle = crackle(T, 1500, 3000, 11000) * env(T, 0.002, 0.05) * 0.3
    return finish(thud + slap + fizz + sizzle, 2.5, sustain=-0.3)


def impact_shield():
    T = 0.6
    x = seconds(T)
    # A shot stopped by a shield: a fluttering inharmonic hum sagging, a crack and an arc.
    f = dive(x, 520, 300, 0.15)
    hum = (tone(f) + 0.5 * tone(f * 1.51) + 0.3 * tone(f * 2.27)) * (0.6 + 0.4 * np.sin(2 * np.pi * 38 * x))
    hum *= env(T, 0.001, 0.14)
    crack = click(T, 2000, 12000, 0.002) * 0.9
    arc = crackle(T, 2500, 2500, 13000) * env(T, 0.001, 0.07) * 0.6
    return finish(hum * 0.6 + crack + arc + sub(T, 90, 0.05) * 0.6, 2.0, room=(0.5, 0.25, 0.15))


def impact_armor(v):
    T = 0.5
    k = 1 + 0.18 * v
    # A round slamming into a vehicle's armour: a hard clang of inharmonic partials, a spark.
    clang = ring(T, [620 * k, 1340 * k, 2210 * k, 3480 * k, 4900 * k], [0.07, 0.05, 0.035, 0.02, 0.012],
                 [0.6, 0.5, 0.4, 0.3, 0.2])
    hit = click(T, 1500, 10000, 0.003) + thump(T, 170, 0.025, drop=0.5) * 0.9
    spark = crackle(T, 2500, 3000, 12000) * env(T, 0.001, 0.03) * 0.5
    return finish(clang * 0.6 + hit + spark + sub(T, 95, 0.03) * 0.5, 2.6, room=(0.4, 0.2, 0.1))


def impact_ground(v):
    T = 0.5
    x = seconds(T)
    # A shot thudding into the dirt: a low thud, a spray of grit and a little energy fizz.
    thud = thump(T, 85 + 10 * v, 0.05, drop=0.5) * 1.3
    dirt = lowpass(noise(T, 100, 3000), 1800 + 300 * v) * env(T, 0.001, 0.05) * 0.8
    grit = crackle(T, 900 + 300 * v, 1500, 8000) * env(T, 0.005, 0.08 + 0.02 * v) * 0.6
    fizz = lowpass(saw(dive(x, 800, 200, 0.05)), 4000) * env(T, 0.001, 0.05) * 0.2
    return finish(thud + dirt + grit + fizz + sub(T, 55, 0.06) * 0.5, 2.4, attack=0.7, sustain=0.0)


def impact_metal(v):
    T = 0.35
    k = 1 + 0.25 * v
    # A shot on a wall or a deck: a thin plate's ping, a tick and a spark.
    ping = ring(T, [1150 * k, 2480 * k, 3920 * k, 5230 * k], [0.05, 0.035, 0.02, 0.012], [0.6, 0.5, 0.35, 0.2])
    tick = click(T, 2500, 12000, 0.0012)
    thunk = thump(T, 240, 0.012, drop=0.4) * 0.5
    spark = crackle(T, 3000, 3500, 13000) * env(T, 0.001, 0.025) * 0.4
    return finish(ping * 0.5 + tick + thunk + spark, 2.0, room=(0.4, 0.18, 0.12))


def ricochet(v):
    T = 0.55
    x = seconds(T)
    # A bolt glancing off: a tick and a whine sliding down and away, wavering.
    f0 = (2600, 3400, 2100)[v]
    f = f0 * np.exp(-x / (0.5, 0.35, 0.7)[v]) * (1 + 0.02 * np.sin(2 * np.pi * (28 + 6 * v) * x))
    whine = (tone(f) + 0.35 * tone(f * 2.006)) * env(T, 0.002, (0.14, 0.1, 0.18)[v])
    tick = click(T, 3000, 13000, 0.001) * 0.9 + ring(T, [f0 * 1.5, f0 * 2.4], [0.02, 0.012], [0.4, 0.25]) * 0.3
    return finish(lowpass(whine * 0.6 + tick, 12000), 1.6, sustain=0.0, room=(0.6, 0.3, 0.2))


# ---- explosions

def blast_mix(T, size):
    """An explosion's layers before the space it is in: `size` 0 (a small shell) to 1.3 (a
    vehicle going up): bigger ones lower, longer, with more debris."""
    x = seconds(T)
    hz = 70 - 22 * size
    crack = click(T, 1000 - 200 * size, 12000, 0.006 + 0.004 * size) * 1.9
    blast = lowpass(noise(T, 80 - 40 * size, 6000), 3000 - 800 * size) * env(T, 0.001, 0.07 + 0.05 * size) * 1.4
    body = boom_body(T, hz, 0.15 + 0.25 * size, 0.35 + 0.45 * size, 1 + 0.3 * size)
    low = sub(T, hz * 0.62, 0.2 + 0.3 * size, drop=0.4) * 0.7
    debris = crackle(T, 600 - 200 * size, 1200, 7000) * env(T, 0.01 + 0.01 * size, 0.25 + 0.35 * size,
                                                         hold=0.05 + 0.1 * size) * 0.6
    zap = lowpass(saw(dive(x, 900, 50, 0.05 + 0.1 * size)), 3000) * env(T, 0.002, 0.08 + 0.12 * size) * 0.3
    return crack + blast + body + low + debris + zap


def rolling(T, gain):
    """A big blast's rumble rolling on."""
    x = seconds(T)
    rumble = lowpass(noise(T, 18, 250), 100) * env(T, 0.08, 1.0, hold=0.2) * 0.7
    roll = lowpass(noise(T, 40, 900), 500) * env(T, 0.1, 0.6, hold=0.1) * (0.7 + 0.3 * np.sin(2 * np.pi * 3.3 * x)) * 0.3
    return (rumble + roll) * gain


def clanks(T, n, t0, gap):
    """`n` pieces of metal coming down, the first `t0` s in, about `gap` s apart."""
    out = np.zeros(int(T * RATE))
    for i in range(n):
        t = t0 + gap * i + 0.1 * rng.random()
        k = 0.8 + 0.5 * rng.random()
        c = ring(0.4, [610 * k, 1130 * k, 1870 * k], [0.08, 0.05, 0.03], [0.5, 0.35, 0.2]) + click(0.4, 800, 6000, 0.004) * 0.5
        out += at(T, t, c * (0.5 - 0.4 * i / n))
    return out


def explosion_small():
    T = 1.7
    dry = finish(blast_mix(T, 0.0), 3.5, bias=0.1, sustain=0.0, comp=(-14, 3))
    return slapback(reverb(dry, size=1.4, decay=0.9, mix=0.25, predelay=0.02), 0.15, 0.2, 2000)


def explosion_medium():
    T = 2.4
    dry = finish(blast_mix(T, 0.5) + rolling(T, 0.5), 3.8, bias=0.1, attack=0.7, sustain=0.0, comp=(-14, 3))
    return slapback(reverb(dry, size=1.8, decay=1.3, mix=0.28, predelay=0.025), 0.2, 0.22, 1800)


def explosion_large():
    T = 3.4
    dry = finish(blast_mix(T, 1.0) + rolling(T, 1.0), 4.0, bias=0.1, attack=0.7, sustain=0.0, comp=(-14, 3))
    return slapback(reverb(dry, size=2.2, decay=1.9, mix=0.3, predelay=0.03), 0.26, 0.25, 1500)


def explosion_wreck():
    T = 4.4
    # A vehicle going up: the biggest blast, something inside cooking off after it, plates and
    # fittings raining down and clattering.
    cook = at(T, 0.45, thump(0.6, 70, 0.1, drop=0.5) + click(0.6, 900, 9000, 0.005) * 0.8
              + lowpass(noise(0.6, 100, 4000), 2500) * env(0.6, 0.001, 0.05) * 0.7) * 0.55
    clatter = crackle(T, 250, 1500, 9000) * env(T, 0.2, 0.9, hold=0.5) * 0.35
    mix = blast_mix(T, 1.3) + rolling(T, 1.2) + cook + clanks(T, 8, 0.4, 0.3) * 0.45 + clatter
    dry = finish(mix, 4.0, bias=0.1, attack=0.7, sustain=0.0, comp=(-14, 3))
    return slapback(reverb(dry, size=2.4, decay=2.2, mix=0.3, predelay=0.03), 0.28, 0.25, 1500)


def airburst():
    T = 1.1
    # A flak shell bursting in mid-air: a sharp crack, a puff and a crackling spray of
    # fragments, nothing near to echo off.
    crack = click(T, 1200, 12000, 0.005) * 1.3
    puff = lowpass(noise(T, 100, 3000), 1500) * env(T, 0.001, 0.08) * 0.8
    spray = crackle(T, 1500, 1500, 9000) * env(T, 0.003, 0.18, hold=0.04) * 1.2
    body = thump(T, 95, 0.07, drop=0.5) * 0.9 + sub(T, 58, 0.1) * 0.6
    dry = finish(crack + puff + spray + body, 3.0, sustain=0.0, comp=(-14, 3))
    return reverb(dry, size=1.6, decay=0.8, mix=0.3, predelay=0.03)


# ---- near misses

def bullet_whiz(v):
    T = 0.42
    x = seconds(T)
    # A near miss: a whistle sweeping down past the ear (Doppler) with an airy rip, loudest
    # as it passes, a tiny snap.
    t0, w, f0 = 0.12, (0.045, 0.035, 0.06)[v], (1700, 2300, 1300)[v]
    passing = np.exp(-((x - t0) / w) ** 2)
    f = f0 + f0 * 0.9 * (1 - 1 / (1 + np.exp(-(x - t0) / (w * 0.45))))
    whistle = (tone(f) + 0.3 * tone(f * 2.03)) * passing * 0.45
    rip = sweep_noise(T, 9000, 2000, lo=800) * passing * 0.7
    snap = at(T, t0 - 0.02, click(0.05, 3000, 12000, 0.0015) * 0.4)
    return whistle + highpass(rip, 1000) + snap


def whiz_heavy():
    T = 0.7
    x = seconds(T)
    # A shell or rocket going by: a low tearing whoosh, its tone dropping as it passes.
    t0, w = 0.25, 0.1
    passing = np.exp(-((x - t0) / w) ** 2)
    f = 190 + 260 * (1 - 1 / (1 + np.exp(-(x - t0) / 0.05)))
    drone = (tone(f) + 0.4 * saw(f * 2.01) * 0.5) * passing * 0.5
    rush = sweep_noise(T, 4000, 500, lo=100) * passing
    flutter = 1 + 0.3 * np.sin(2 * np.pi * 31 * x)
    return (lowpass(drone, 2500) + rush * 0.8) * flutter


# ---- reloads and spent cases

def placed(T, events):
    """The `events` ((start s, samples, gain), ...) in `T` s, in a small room."""
    out = sum(at(T, t, y) * g for t, y, g in events)
    return reverb(out, size=0.3, decay=0.14, mix=0.1)


def cell_hiss(d):
    return noise(d, 3000, 10000) * env(d, 0.005, d * 0.25) * 0.4


def reload_rifle():
    T = 1.5
    # A rifle: the catch, the cell out with a hiss, a short charging whir, the new cell in, the
    # bolt run home.
    return placed(T, [
        (0.0, mech(0.06, "latch"), 0.7), (0.1, mech(0.16, "slide"), 0.8), (0.14, cell_hiss(0.2), 0.5),
        (0.4, servo(0.4, 260, 900, 0.25) * env(0.4, 0.02, 0.06, hold=0.3), 0.25),
        (0.9, mech(0.12, "clunk"), 0.9), (0.95, mech(0.06, "latch"), 0.5), (1.2, mech(0.1, "bolt"), 1.0),
    ])


def reload_scatter():
    T = 2.0
    # The scatter gun: four shells thumbed into the tube, the pump racked.

    def shell(k):
        d = 0.14
        return (click(d, 1200, 7000, 0.003) + thump(d, 300 * k, 0.012, drop=0.3) * 0.7
                + ring(d, [520 * k, 870 * k], [0.03, 0.02], [0.4, 0.25]))

    events = [(0.12 + 0.36 * i, shell(1 + 0.06 * i), 0.8) for i in range(4)]
    return placed(T, events + [(1.58, mech(0.26, "slide"), 1.0)])


def reload_lance():
    T = 2.3
    # The lance: the bolt drawn, the spent cell hissing out, a long charge rising, the cell
    # home, the bolt closed and locked.
    return placed(T, [
        (0.0, mech(0.2, "slide"), 0.9), (0.28, cell_hiss(0.3), 0.6),
        (0.6, servo(0.9, 220, 1400, 0.5) * env(0.9, 0.03, 0.08, hold=0.78), 0.28),
        (1.58, mech(0.16, "clunk"), 1.0), (1.9, mech(0.2, "slide"), 0.9), (2.12, mech(0.06, "latch"), 0.7),
    ])


def reload_launcher():
    T = 1.8
    # A launcher: the tube opened, a rocket slid down it and seated, the catch, a ready beep.
    d = 0.6
    x = seconds(d)
    slide_in = (sweep_noise(d, 900, 4000, lo=200) * np.sin(np.pi * np.clip(x / d, 0, 1)) ** 2
                + ring(d, [310, 505], [0.2, 0.15], [0.15, 0.1]))
    seat = mech(0.25, "clunk") + sub(0.25, 70, 0.05) * 0.6
    return placed(T, [
        (0.0, mech(0.2, "clunk"), 0.8), (0.4, slide_in, 0.35), (1.03, seat, 1.0),
        (1.4, mech(0.06, "latch"), 0.7), (1.55, beep(0.12, 1760, 0.07), 0.3),
    ])


def reload_pistol():
    T = 1.0
    # The pistol: the catch, the cell dropped, a new one slapped in, the slide.
    return placed(T, [
        (0.0, mech(0.05, "latch"), 0.7), (0.07, cell_hiss(0.12), 0.4), (0.48, mech(0.1, "bolt"), 0.9),
        (0.7, mech(0.14, "slide"), 1.0),
    ])


def reload_heavy():
    T = 2.4
    # A MAX's arms: servos opening the feed, a belt run through, the breeches slammed.
    return placed(T, [
        (0.0, mech(0.55, "servo"), 0.3), (0.5, mech(0.8, "belt"), 0.8), (1.38, mech(0.25, "clunk"), 1.0),
        (1.5, mech(0.45, "servo"), 0.25), (2.02, mech(0.2, "clunk"), 0.9), (2.1, mech(0.2, "clunk"), 0.7),
        (2.26, mech(0.06, "latch"), 0.6),
    ])


def reload_done():
    T = 0.3
    # Ready again: a latch and a quick two-note chirp.
    chirp = sum(at(T, 0.05 * i, np.sin(2 * np.pi * hz * seconds(T - 0.05 * i)) * env(T - 0.05 * i, 0.002, 0.04))
                for i, hz in enumerate([1318.5, 1975.5]))
    return at(T, 0.0, mech(0.06, "latch")) * 0.6 + lowpass(chirp, 9000) * 0.35


def casing(v):
    T = 0.5
    # A spent case on the floor: a few tinks, closer and quieter each bounce.
    out = np.zeros(int(T * RATE))
    for t, g in ((0.0, 1.0), (0.09 + 0.02 * v, 0.6), (0.15 + 0.03 * v, 0.35), (0.19 + 0.035 * v, 0.2)):
        out += at(T, t, mech(0.12, "tink")) * g
    return out


def casing_shell():
    T = 0.7
    # A cannon's case: bigger, hollow.
    out = np.zeros(int(T * RATE))
    for t, g in ((0.0, 1.0), (0.16, 0.6), (0.27, 0.35), (0.34, 0.2)):
        k = 0.92 + 0.16 * rng.random()
        c = (ring(0.2, [1250 * k, 2080 * k, 3340 * k], [0.06, 0.04, 0.025], [1.0, 0.6, 0.35])
             + click(0.2, 1500, 9000, 0.001) * 0.6 + thump(0.2, 300, 0.01, drop=0.3) * 0.3)
        out += at(T, t, c) * g
    return out


SOUNDS = {
    # name: (generator, level dBFS, seed[, "peak"]): the level is the whole sound's RMS, or
    # with "peak" that of its loudest 100 ms (see `write`).
    "footstep_1": (lambda: footstep(1), -27.0, 11),
    "footstep_2": (lambda: footstep(2), -27.0, 12),
    "footstep_3": (lambda: footstep(3), -27.0, 13),
    "laser_fire": (laser_fire, -16.0, 21),
    "laser_fire_red": (laser_fire_red, -16.0, 22),
    "hit_player": (hit_player, -16.0, 31),
    "player_hit": (player_hit, -16.0, 35),
    "tagged_out": (tagged_out, -16.0, 41),
    "hit_target": (hit_target, -14.0, 51),
    "grenade_bounce": (grenade_bounce, -22.0, 61),
    "grenade_throw": (grenade_throw, -20.0, 66),
    "grenade_arm": (grenade_arm, -21.5, 71),
    "grenade_burst": (grenade_burst, -20.0, 81),
    "respawn": (respawn, -20.0, 91),
    "deny": (deny, -21.0, 101),
    "empty": (empty, -22.0, 111),
    "reload": (reload, -24.0, 121),
    "pickup": (pickup, -19.0, 141),
    "score_bleep": (score_bleep, -22.0, 151),
    "score_tag": (score_tag, -21.0, 161),
    "mega_win": (mega_win, -13.0, 171),
    "power_tick": (power_tick, -22.0, 181),
    "power_up": (power_up, -18.0, 191),
    "turret_fire": (turret_fire, -18.0, 201),
    "turret_down": (turret_down, -19.0, 211),
    "shield_zap": (shield_zap, -15.0, 221),
    "shield_hum": (shield_hum, -24.0, 231),
    "mirror_ping": (mirror_ping, -21.0, 241),
    "jump_pad": (jump_pad, -20.0, 251),
    "gate_hum": (gate_hum, -27.0, 261),
    "charge_loop": (charge_loop, -26.0, 271),
    "charge_full": (charge_full, -21.0, 281),
    "beam_fire": (beam_fire, -14.0, 291),
    "lance_fire": (lance_fire, -14.0, 292),
    "headshot": (headshot, -15.0, 293),
    "sensor_pulse": (sensor_pulse, -21.0, 294),
    "barrier_up": (barrier_up, -19.0, 301),
    "barrier_hit": (barrier_hit, -20.0, 311),
    "barrier_down": (barrier_down, -22.0, 321),
    "piece_slide": (piece_slide, -23.0, 381),
    "piece_stuck": (piece_stuck, -22.0, 382),
    "puzzle_solved": (puzzle_solved, -18.0, 383),
    "box_open": (box_open, -20.0, 384),
    "turret_extend": (turret_extend, -20.0, 385),
    "turret_retract": (turret_retract, -21.0, 386),
    "box_close": (box_close, -20.0, 387),
    "event_blackout": (event_blackout, -15.0, 331),
    "event_overdrive": (event_overdrive, -16.0, 341),
    "event_double": (event_double, -16.0, 351),
    "event_storm": (event_storm, -16.0, 361),
    "event_lowgrav": (event_lowgrav, -17.0, 371),
    # Planetstrike: vehicle engines (loops).
    "engine_light_tank": (engine_light_tank, -25.0, 401),
    "engine_heavy_tank": (engine_heavy_tank, -24.0, 402),
    "engine_bus": (engine_bus, -26.0, 403),
    "engine_vtol": (engine_vtol, -25.0, 404),
    "tracks": (tracks, -28.0, 405),
    # Planetstrike: lock-ons, kits and gear.
    "lock_tone": (lock_tone, -22.0, 419),
    "lock_warning": (lock_warning, -19.0, 420),
    "flares": (flares, -18.0, 460),
    "cloak": (cloak, -20.0, 461),
    "kit_down": (kit_down, -18.0, 462),
    "silo_warning": (silo_warning, -19.0, 464),
    "jet_thrust": (jet_thrust, -20.0, 465),
    "chute_open": (chute_open, -19.0, 466),
    "hack_done": (hack_done, -20.0, 463),
    # Planetstrike: world and base.
    "vehicle_spawn": (vehicle_spawn, -18.0, 442),
    "rearm_loop": (rearm_loop, -27.0, 443),
    "deploy": (deploy, -18.0, 444),
    "undeploy": (undeploy, -19.0, 445),
    "cloak_on": (cloak_on, -20.0, 446),
    "warp": (warp, -18.0, 447),
    "warpgate_hum": (warpgate_hum, -26.0, 448),
    "forcefield_buzz": (forcefield_buzz, -28.0, 449),
    "forcefield_deny": (forcefield_deny, -17.0, 450),
    "terminal_open": (terminal_open, -23.0, 451),
    "terminal_select": (terminal_select, -24.0, 452),
    "hack_start": (hack_start, -20.0, 453),
    "hack_tick": (hack_tick, -26.0, 454),
    "hack_stopped": (hack_stopped, -21.0, 455),
    "base_captured": (base_captured, -16.0, 456),
    "base_lost": (base_lost, -16.0, 457),
    "continent_captured": (continent_captured, -14.0, 458),
    "alert": (alert, -18.0, 459),
    # Planetstrike: player.
    "heal": (heal, -21.0, 461),
    "fall_thud": (fall_thud, -19.0, 462),
    "hurt": (hurt, -17.0, 463),
    "death": (death, -16.0, 464),
    "spawn": (spawn, -19.0, 465),
    "hitmarker": (hitmarker, -21.0, 466),
    "kill_confirm": (kill_confirm, -20.0, 467),
    # Planetstrike: weapons, impacts and explosions ("peak": the level of the loudest 100 ms).
    "rifle_fire": (rifle_fire, -13.0, 501, "peak"),
    "ember_fire": (ember_fire, -10.0, 502, "peak"),
    "volt_fire": (volt_fire, -13.0, 503, "peak"),
    "hive_fire": (hive_fire, -15.0, 504, "peak"),
    "scatter_fire": (scatter_fire, -7.0, 505, "peak"),
    "lance_crack": (lance_crack, -6.5, 506, "peak"),
    "striker_fire": (striker_fire, -8.5, 507, "peak"),
    "skyhook_fire": (skyhook_fire, -10.0, 508, "peak"),
    "pistol_fire": (pistol_fire, -14.0, 509, "peak"),
    "medgun_loop": (medgun_loop, -27.0, 510),
    "repair_loop": (repair_loop, -27.0, 511),
    "pounder_fire": (pounder_fire, -12.0, 512, "peak"),
    "mauler_fire": (mauler_fire, -7.5, 513, "peak"),
    "burster_fire": (burster_fire, -12.0, 514, "peak"),
    "cannon_light": (cannon_light, -6.5, 515, "peak"),
    "cannon_heavy": (cannon_heavy, -6.0, 516, "peak"),
    "rail_fire": (rail_fire, -6.0, 517, "peak"),
    "autocannon_fire": (autocannon_fire, -10.0, 518, "peak"),
    "flak_fire": (flak_fire, -12.0, 519, "peak"),
    "rocket_fire": (rocket_fire, -9.0, 520, "peak"),
    "missile_fire": (missile_fire, -10.5, 521, "peak"),
    "air_cannon": (air_cannon, -13.0, 522, "peak"),
    "chaingun": (chaingun, -13.0, 523, "peak"),
    "turret_wall_fire": (turret_wall_fire, -8.5, 524, "peak"),
    "spitfire_fire": (spitfire_fire, -16.0, 525, "peak"),
    "tail_small": (tail_small, -16.0, 526, "peak"),
    "tail_medium": (tail_medium, -13.0, 527, "peak"),
    "tail_heavy": (tail_heavy, -11.0, 528, "peak"),
    "impact_body_1": (lambda: impact_body(0), -15.0, 531, "peak"),
    "impact_body_2": (lambda: impact_body(1), -15.0, 532, "peak"),
    "impact_shield": (impact_shield, -14.0, 533, "peak"),
    "impact_armor_1": (lambda: impact_armor(0), -14.0, 534, "peak"),
    "impact_armor_2": (lambda: impact_armor(1), -14.0, 535, "peak"),
    "impact_ground_1": (lambda: impact_ground(0), -15.0, 536, "peak"),
    "impact_ground_2": (lambda: impact_ground(1), -15.0, 537, "peak"),
    "impact_ground_3": (lambda: impact_ground(2), -15.0, 538, "peak"),
    "impact_metal_1": (lambda: impact_metal(0), -16.0, 539, "peak"),
    "impact_metal_2": (lambda: impact_metal(1), -16.0, 540, "peak"),
    "ricochet_1": (lambda: ricochet(0), -18.0, 541, "peak"),
    "ricochet_2": (lambda: ricochet(1), -18.0, 542, "peak"),
    "ricochet_3": (lambda: ricochet(2), -18.0, 543, "peak"),
    "explosion_small": (explosion_small, -6.5, 551, "peak"),
    "explosion_medium": (explosion_medium, -6.0, 552, "peak"),
    "explosion_large": (explosion_large, -6.0, 553, "peak"),
    "explosion_wreck": (explosion_wreck, -5.5, 554, "peak"),
    "airburst": (airburst, -8.0, 555, "peak"),
    "bullet_whiz_1": (lambda: bullet_whiz(0), -17.0, 561, "peak"),
    "bullet_whiz_2": (lambda: bullet_whiz(1), -17.0, 562, "peak"),
    "bullet_whiz_3": (lambda: bullet_whiz(2), -17.0, 563, "peak"),
    "whiz_heavy": (whiz_heavy, -14.0, 564, "peak"),
    "reload_rifle": (reload_rifle, -19.0, 571, "peak"),
    "reload_scatter": (reload_scatter, -19.0, 572, "peak"),
    "reload_lance": (reload_lance, -19.0, 573, "peak"),
    "reload_launcher": (reload_launcher, -19.0, 574, "peak"),
    "reload_pistol": (reload_pistol, -19.0, 575, "peak"),
    "reload_heavy": (reload_heavy, -19.0, 576, "peak"),
    "reload_done": (reload_done, -20.0, 577, "peak"),
    "casing_1": (lambda: casing(0), -26.0, 581, "peak"),
    "casing_2": (lambda: casing(1), -26.0, 582, "peak"),
    "casing_shell": (casing_shell, -24.0, 583, "peak"),
}

# Sounds played as loops.
LOOPS = {"shield_hum", "gate_hum", "charge_loop",
         "engine_light_tank", "engine_heavy_tank", "engine_bus", "engine_vtol", "tracks",
         "medgun_loop", "repair_loop", "rearm_loop", "warpgate_hum", "forcefield_buzz"}


if __name__ == "__main__":
    wanted = [os.path.splitext(os.path.basename(a))[0] for a in sys.argv[1:]] or list(SOUNDS)
    unknown = [w for w in wanted if w not in SOUNDS]
    if unknown:
        sys.exit(f"unknown sound(s): {', '.join(unknown)}; known: {', '.join(SOUNDS)}")
    os.makedirs(OUT, exist_ok=True)
    for name in wanted:
        make, level, seed, *mode = SOUNDS[name]
        rng = np.random.default_rng(seed)
        write(name, make(), level, loop=name in LOOPS, mode=mode[0] if mode else "rms")
