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

Added for Neon Planetstrike (all in `SOUNDS`, seeds 401-467):

- Vehicle engines (seamless loops): `engine_light_tank` (2 s), `engine_heavy_tank` (2.5 s),
  `engine_bus` (2 s), `engine_vtol` (2 s), `tracks` (1.5 s).
- Vehicle weapons: `cannon_light`, `cannon_heavy`, `flak_fire` (0.12 s, played ~11/s),
  `airburst`, `rocket_fire`, `chaingun` (0.08 s).
- Infantry weapons: `rifle_fire` (0.12 s), `lance_crack`, `striker_fire`, `pistol_fire`,
  `medgun_loop` (2 s loop), `reload_long`.
- Explosions/impacts: `explosion_small`, `explosion_large`, `impact_armor`, `impact_ground`,
  `bullet_whiz`.
- World/base: `turret_wall_fire`, `vehicle_spawn`, `rearm_loop` (2 s loop), `deploy`,
  `undeploy`, `cloak_on`, `warp`, `warpgate_hum` (3 s loop), `forcefield_buzz` (1.5 s loop),
  `forcefield_deny`, `terminal_open`, `terminal_select`, `hack_start`, `hack_tick`,
  `hack_stopped`, `base_captured`, `base_lost`, `continent_captured`, `alert`.
- Player: `heal`, `fall_thud`, `hurt`, `death`, `spawn`, `hitmarker`, `kill_confirm`.

Levels: weapons/explosions -14 to -18 dBFS, loops -24 to -28 dBFS, UI -23 to -26 dBFS.

Loops (listed in `LOOPS`, written without fades) are made seamless by construction: every
tone and modulation has a whole number of cycles over the loop length (`cyc()`), noise and
filters are FFT-based and therefore circular, and events/echoes wrap round the end
(`loop_at()`, `loop_echoes()`). Avoid `sweep_noise()`, `at()` and `echoes()` in loops.
