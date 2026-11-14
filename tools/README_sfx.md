# Sound effects

`tools/gen_sfx.py` synthesizes all sound effects in `assets/sounds/` (mono, 44.1 kHz Ogg
Vorbis) with numpy and encodes them with ffmpeg (needs libvorbis).

    python3 tools/gen_sfx.py                    # regenerate everything
    python3 tools/gen_sfx.py laser_fire deny    # only these

Each sound has a fixed random seed, so regenerating a subset gives the same audio as a full
run. Loudness is set per sound as an RMS level in the `SOUNDS` table at the bottom of the
script (fire/hits around -16 dBFS, footsteps -27 dBFS); peaks are softly limited below full
scale. Tweak the generator functions or levels there and rerun.

`tools/gen_derived_sfx.sh` derives the special weapons' fire sounds (`ricochet_fire`,
`dual_fire`, `charge_tap`, each with a `_red` twin) from the blasters', and `dash` from
`jump_pad`, with ffmpeg filters (pitch, filtering, other sounds mixed in). Rerun it after
regenerating those.

## Planetstrike sounds

Added for Neon Planetstrike (all in `SOUNDS`, seeds 401-467; the weapons 501-583):

- Vehicle engines (seamless loops): `engine_light_tank` (2 s), `engine_heavy_tank` (2.5 s),
  `engine_bus` (2 s), `engine_vtol` (2 s), `tracks` (1.5 s).
- Lock-ons and gear: `lock_tone`, `lock_warning`, `flares`, `cloak`, `kit_down`, `silo_warning`,
  `jet_thrust`, `chute_open`, `hack_done`.
- World/base: `turret_wall_fire`, `vehicle_spawn`, `rearm_loop` (2 s loop), `deploy`,
  `undeploy`, `cloak_on`, `warp`, `warpgate_hum` (3 s loop), `forcefield_buzz` (1.5 s loop),
  `forcefield_deny`, `terminal_open`, `terminal_select`, `hack_start`, `hack_tick`,
  `hack_stopped`, `base_captured`, `base_lost`, `continent_captured`, `alert`.
- Player: `heal`, `fall_thud`, `hurt`, `death`, `spawn`, `hitmarker`, `kill_confirm`.

Levels: loops -24 to -28 dBFS, UI -23 to -26 dBFS (the weapons: below).

Loops (listed in `LOOPS`, written without fades) are made seamless by construction: every
tone and modulation has a whole number of cycles over the loop length (`cyc()`), noise and
filters are FFT-based and therefore circular, and events/echoes wrap round the end
(`loop_at()`, `loop_echoes()`). Avoid `sweep_noise()`, `at()` and `echoes()` in loops.

## Weapons

Every weapon and every kind of ammunition has its own sound, built from layers and glued by
`finish()`:

- a **crack** (`click`, band noise), a **body** (`thump`, a `fm`/`saw`/`square` zap diving with
  `dive`), a **sub** (`sub`: a pure sine under the punch) and the **mechanism** (`mech`: bolt,
  slide, belt, clunk, latch, servo, tink) a little after the shot;
- `saturate` (an oversampled tanh waveshaper) for density, `transient` (a three-band transient
  shaper) to raise the attacks and tighten what follows, `compress` on the big ones;
- `reverb` (four damped combs, two all-passes; seeded like everything else) and `slapback` for
  the space: a small room for small arms, hills for cannons and blasts. Leave room for the tail
  in the sound's length.

These are written in **peak mode** (a fourth element `"peak"` in the `SOUNDS` tuple): the level
is the RMS of the loudest 100 ms, with a soft-knee limiter at -2 dBFS, so a long tail no longer
pushes the crack into the limiter as whole-file RMS did. Loops and the older sounds stay in RMS
mode. Loudest-100 ms levels: small arms -13 to -15 dBFS, heavy infantry weapons -7 to -10,
cannons and explosions -5.5 to -6.5, impacts -14 to -18, reloads -19, cases -24 to -26.

| group | sounds |
|---|---|
| infantry | `rifle_fire`, `ember_fire`, `volt_fire`, `hive_fire`, `scatter_fire`, `lance_crack`, `striker_fire`, `skyhook_fire`, `pistol_fire`, `medgun_loop`, `repair_loop` (2 s loops) |
| MAX | `pounder_fire`, `mauler_fire`, `burster_fire` |
| vehicles, turrets | `cannon_light`, `cannon_heavy`, `rail_fire`, `autocannon_fire`, `flak_fire`, `rocket_fire`, `missile_fire`, `air_cannon`, `chaingun`, `turret_wall_fire`, `spitfire_fire` |
| far off | `tail_small`, `tail_medium`, `tail_heavy` (`far()`: dulled, softened, a rolling tail): the game fades from a shot's own sound to its tail with distance |
| impacts | `impact_body_1..2`, `impact_shield`, `impact_armor_1..2`, `impact_ground_1..3`, `impact_metal_1..2`, `ricochet_1..3` |
| explosions | `explosion_small`, `explosion_medium`, `explosion_large`, `explosion_wreck`, `airburst` |
| near misses | `bullet_whiz_1..3`, `whiz_heavy` |
| mechanisms | `reload_rifle`, `reload_scatter`, `reload_lance`, `reload_launcher`, `reload_pistol`, `reload_heavy`, `reload_done`, `casing_1..2`, `casing_shell` |

Numbered sounds are variants of one generator; the game picks one at random each time.
