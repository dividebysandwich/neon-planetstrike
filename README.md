# Neon Planetstrike

Three factions — RED, BLUE and GREEN — fight for continents linked by warp gates, on foot and in
vehicles, in procedurally generated neon worlds. For the flat screen and VR headsets (OpenXR).
Built with Bevy, on the engine, netcode, avatars, sounds and VR of
[Neon Lasertag](https://github.com/dividebysandwich/neon-lasertag).

## Getting started

```
cargo run --release                                  # start menu (VR if a headset is running)
cargo run --release -- --flat --offline --seed 42    # straight into an offline world
cargo run --release -- --flat --connect host:27430   # join a server
cargo run --release -p ps-server -- --seed 42        # dedicated server
```

The start menu offers an **Offline world** (the server runs inside the game, with bots),
**Multiplayer** (the internet server list, LAN games, joining by address, hosting) and
**Options**.

## The world

- A world is made from a seed: three **sanctuaries** (one per faction, each with a home base
  and two warp gates) and a number of **contested continents** (`--world small|medium|large`:
  4, 6 or 9), each with two or three **warp gates**. Gates link neighbouring continents on the
  planet map; nobody can warp into an enemy sanctuary.
- Every continent has its own **theme** — Synth Dunes, Neon Jungle, Glacier, Ember, Fungal
  Marsh, Circuit Mesa — shaping its land (dunes, ridges, craters, terraces, wetlands) and what
  grows and stands on it, with its own sky and neon; mountains and lakes by theme. The marsh
  and the dunes are nearly flat; the mesas and Ember have their cliffs and craters; the jungle
  and the glacier are rugged, with much of the land too steep to drive (never round the bases
  and gates). Continents
  are 4 km islands, big enough for armour and aircraft. A day passes in 30 minutes; the neon
  glows through the night and the bases' lights come on.
- One contested continent of every world is **Hollow Reach**, the canyon caves: a rock plateau
  cut by steep canyons along the roads, roofed with rock in long stretches (glowing stalactites
  under them, skylights between, the light dim beneath), every base and warp gate in a sinkhole
  with a rocky overhang round its rim, open to the sky in the middle. Armour goes through the
  canyons (the towers stand in pits with their own roads); aircraft come in through the
  sinkholes and skylights, and stop under the roofs. Its bases are built of alien stuff: ribbed
  walls and spires, crystal and carapace equipment. A **zipline** runs from each base's main
  building's roof up to the rim: E within reach rides it the way you look (faster downhill),
  puts you down at the far end, and jump (or E) lets go wherever you are. Riders shoot and are
  shot as on foot.
- Continents have a **center base** and five to eight outer bases joined by a **lattice**. The
  lattice also runs to the warp gates, so it spans the whole world from sanctuary to sanctuary.
  **Roads** follow it from base gateway to base gateway and into the warp gates, graded into
  the land and winding round the steep ground, so armour can always get through.
- **Capturing**: a base can be hacked by a faction that holds a base linked to it, or whose
  faction holds the continent behind a linked warp gate. Go into the base's main building, find
  the core room and solve the console's sliding puzzle: the hack runs for 5 minutes. The owners
  stop it by solving the console again. Holding a continent's center base captures the whole
  continent — every base on it flips — which opens the next continents to attack.
- **Capture points**: a third of the outposts have no console but three **points**, A in the
  front yard, B in the core room and C in the back of the yard, each a pylon with a ring round
  it on the ground. Standing in a point not your faction's takes it over 24 s (faster with more
  of you, up to two and a half times); a rival's progress there drains first, two factions in
  it hold it as it is, and its holders alone in it drain the takers'. A faction that may attack
  the base and holds two points runs a **capture** of 2.5 minutes that takes the base; the owners
  stop it by taking a point back (a capture under way ends if the server restarts). Taking a
  point earns XP, retaking one during a capture counts as a defence.
- **Towers** stand near most bases: a capture point near the top (taken at once by solving
  its puzzle) and spawn tubes in a bunker below ground: a foothold for the attack.
- Bots fight only on **contested** continents (bases of different factions, or a neighbour of
  another faction's), split between the factions with a stake there. They go in squads of four,
  take towers before bases, defend hacked bases, find their way over the land, drive tanks,
  rovers and AAA (aircraft first), fly VTOL rocket runs, hunt aircraft in interceptors (missiles
  once locked), fly dropships for their gunners, crew each other's guns, bring spawn buses and
  harvesters, fire rockets at armour and repair what's broken. Humans take bots' places.
- The world's state is kept: the dedicated server saves it to `world_state.ron` (an offline
  world to the settings folder) and picks it up again for the same seed.

## Bases

A walled compound with walkable walls (stairs up inside; buttressed outside, pylons flanking the
gateways), vehicle gateways with **forcefields**
that let infantry and the owners' vehicles through, and big **wall turrets** raised on pillars
over the walls: automatic (at 30 % of the rate of fire), or manned by a player (E) at the full
rate, seen from the turret's own sight. Turrets can be destroyed and repaired. The main building, on one or two
floors, has the **spawn room** behind a forcefield only the owners pass, **equipment
terminals**, **medical terminals**, offices and the **core room** with the console. A
**generator** powers the forcefields, turrets and vehicle pad: destroy it to take them down,
repair it to bring them back. In the yard: the **vehicle pad** with its terminal, the **rearm
pad**, and the **resource silo**. **Bars** float over what is fought over: the generator's and
the turrets' health (near, or while damaged), each capture point's (its letter over it in its
holder's colour, the bar filling in the taker's, flashing while contested) and a hack's or
capture's progress over the console (or the base).

**Resources**: turrets firing and vehicles ordered use up a base's resources. A base that runs
dry goes neutral (white), and nobody spawns there until its silo is refilled and it is
captured again. Harvesters fill silos from resource nodes that turn up around the continent.

**Facilities**: every outpost is also one of four facilities, taken in turn round each
continent (the map and the spawn list name them). A faction holding one with its generator up
has its benefit on that continent (a sanctuary's owner has them all there):

| facility | benefit |
|---|---|
| **Tech Plant** | heavy tanks, interceptors and dropships may be ordered (the vehicle terminal says so otherwise) |
| **Bio Lab** | respawning takes half as long |
| **Amp Station** | the faction's wall turrets take half damage and fire faster when left to themselves |
| **Interlink** | enemies within 250 m of the faction's bases show on its minimaps |

**Construction** (as in PlanetSide 2): at the back of one of your faction's **harvesters**,
**E** opens its construction terminal; pulling a structure costs the harvester's load, and you
carry it until you put it down (**N** takes it out again; a ghost shows where it goes: green,
amber where no silo would keep it going, red where it can't go; click to place, right click to
put it away; in VR the command wheel's PLACE puts it 6 m ahead). Not in or by a base, within
40 m of you, on level open ground:

| structure | cost | |
|---|---|---|
| **Mobile Silo** | 40 | holds three harvester loads (it starts at 10 %); keeps the structures within 120 m going |
| **Wall** | 15 | 6 m wide, 3.5 m high: stops bodies, vehicles and shots |
| **Bastion Turret** | 50 | an armoured bunker whose turret fires at enemy infantry, vehicles and aircraft within 150 m (220 m for aircraft) |
| **Forward Spawn Tube** | 60 | the faction spawns at it (one near a silo; on the spawn list) |

Every structure draws a little from the nearest mobile silo of its faction all the time (a
silo from itself), so silos need refilling: park a harvester by one and it unloads into it (a
full load fills a sixth of a base's silo, a third of a mobile silo). Without a silo that isn't
empty, structures decay until they fall (an empty silo too); a silo running low or dry warns its
faction with an alarm. Structures can be shot down (XP for it) and repaired, and are kept when
the server restarts (`structures.ron` beside the world's state).

## Alerts, locks and population

**Objective markers**: what matters on your continent is marked on screen with a word and the
distance. The consoles (a diamond): **HACK** (amber: a base or tower your faction may hack),
**DEFEND** (red: one of yours being hacked) and **HOLD** (green: one your faction is hacking;
keep the enemy off its console). The capture points (a circle, with the point's letter):
**CAPTURE** (one not yours at a base you may take), **DEFEND** (one of your base's taken or
being taken) and **HOLD** (one you hold while your capture runs). Near you (a square): the
generators and turrets to **DESTROY** at a base you may take, and your own to **REPAIR** when
damaged. Out of view they sit at the screen's edge on the side they're on; in VR they float
over them, seen through walls. Ten show: the nearest consoles and points, and a few of the
nearest generators and turrets. The map and minimap show who holds each capture point, and
the HUD line of the base you're in (A:R B:B C:R, and the capture's time).

- **Alerts**: every half hour (the first ten minutes in) the busiest contested continent gets
  a 15-minute **alert**: whichever faction holds the most of its bases (outposts and the
  center) when time is up wins it, and its players earn 1000 XP (250 for those elsewhere). The
  HUD shows the alert, the time left and the bases each faction holds (the scoreboard and the
  VR menu too), and its end.
- **Continent locks**: taking a continent (its center) earns the faction's players there 500
  XP, and every continent a faction holds (not counting sanctuaries) makes its vehicles, MAXes
  and structures 10 % cheaper, up to 30 % (the scoreboard says so).
- **Population**: the map lists who is on each continent (players and bots, by faction). A
  faction with at least 4 players on a continent and half again as many plus two as the
  smallest faction with a stake there is **overpopulated** there: its players can't spawn onto
  it from elsewhere (the map and spawn list say OVERPOP). Bots fill the factions short of
  players, as before.
- **Spotting**: **Q** marks the enemy one aims at (infantry within 400 m, vehicles within
  700 m, in sight) for one's faction for 10 s: a red mark over it on their screens and a dot on
  their minimaps (an Interlink's spotted enemies show the same way); a team mate's kill on it
  pays the spotter 25 XP.

## Players

- **Health and armor** in percent; armor takes two thirds of the damage while it lasts. No
  regeneration: heal at a medical terminal, or have a team mate use the **heal gun**. Falls and
  being run over hurt.
- **Revive**: whoever falls on foot (not by suicide or in a vehicle) is **downed** for 30 s: a
  medic's heal gun held on the body for 2.5 s brings them back where they fell, with 40 health,
  no armor and their ammunition as it was. Their faction sees a green cross over the body (and
  on the minimap); the downed can wait for a medic or spawn elsewhere as usual. Bot medics
  revive, and dead bots wait a little while a medic is near.
- **Loadout** (equipment terminal): armor, a rifle (Pulse Rifle, Scatter Gun, Prism Lance, or
  the faction's own: Red's hard-hitting Ember Carbine, Blue's fast and accurate Volt Rifle,
  Green's big-magazine Hive Repeater), a sidearm, and a special: Striker (anti-vehicle rockets),
  Skyhook (anti-air missiles), Medgun (heals players) or Repair Gun (vehicles, turrets,
  generators); and a grenade: **Frag**, **EMP** (strips armour, drops and drains cloaks, hurts
  enemy deployables), **Smoke** (a 12 s cloud nobody sees through: bots, turrets and lock-ons
  neither) or **Flash** (blinds the enemies in sight of it for a few seconds).
- **Light armour** has **jump jets**: hold jump in the air (fuel refills on the ground; the HUD
  shows it). Leaving an aircraft in the air you fall under a **chute**, unhurt.
- **Forcefields** stop every shot and grenade, the owners' own too. Damage falls off with distance; headshots count
  double. Shots spread: less aiming down the sights or crouching, more on the move and in the
  air.
- **MAX** (the fifth armor): a power-armoured exosuit with 250 armour, slow and never running,
  carrying one of three arms in place of the rifle and nothing else: **Pounder** (twin
  chainguns, against infantry), **Mauler** (twin cannons, against armour) or **Burster** (flak
  that bursts near aircraft). Taking one costs the base's silo 40 resources (free at a
  sanctuary). A MAX can't drive or man a gun (it rides in a dropship's passenger seats), work a
  console or throw grenades; medics' heal guns pass it by and engineers' repair guns mend its
  armour, then its health.
- **Infiltrator** (the infiltration suit, 15 armour): **B** cloaks (12 s from full, refilling
  slowly): enemies are sent a cloaked player only within 10 m, drawn as a faint shimmer, and
  turrets, spitfires and bots don't see one further off. Firing, throwing, hacking or a seat
  drops it. At an enemy equipment, medical or vehicle terminal or wall turret, **E hacks** it:
  standing still at it for 5 s turns it to your faction's for 5 minutes (the terminal serves
  you, the turret fires for you and you may man it); the base changing hands clears its hacks.
- **Engineers** (the Repair Gun) carry a kit, put down with **B** a step ahead (refilled at an
  equipment terminal): a **Spitfire** auto turret that fires at enemy infantry within 40 m,
  three **anti-tank mines** that go off under enemy ground vehicles (enemies see them only
  within 15 m), or an **ammo pack** that tops up the reserves of everyone of the faction within
  5 m for two minutes. One spitfire, one ammo pack and three mines at a time (the oldest goes
  for a new one); they can be shot and repaired, and go when their engineer leaves the
  continent. Bots take MAXes and infiltration suits now and then, cloak on the way, and
  engineers put their kit down at the bases they fight over.
- The menu's **suicide** gets one back to the spawn screen to start somewhere else.
- Dead, the map opens as the spawn screen: spawn at any base your faction holds, on any
  continent, at a deployed spawn bus on your continent, on your squad leader, or at your
  sanctuary.

## Battle Rank

Players earn **XP** for what they do for their faction: kills (more for a headshot), assists
(20 damage to someone a team mate kills within 10 s), destroying vehicles, revives, healing,
repairs (vehicles, turrets, generators), starting or stopping a hack at a console, delivering
resources and team mates spawning on one's bus or on one as squad leader. XP shows as it comes
("+100 KILL") under the crosshair; it makes the **Battle Rank** (BR 2 at 1000 XP, 5 at 10 000,
10 at 45 000, up to 40), and each rank brings a **certification point** (counted, not yet
spent on anything). The server keeps everyone's XP by name in `profiles.ron` next to the
world's state. Holding **Tab** shows the scoreboard: everyone on the continent by faction with
their BR, kills, deaths, assists and XP this session.

## Squads

Players of a faction team up in **squads** of up to ten, across continents (P, or SQUAD in the
menu: create, join, leave; the leader hands on the lead and kicks). The leader puts the squad's
**waypoint** where they aim (T; at the sky it goes away) or by clicking the map: a beam of light
over the land, on the map and the minimap (at its edge when it's further). The squad is listed
in the HUD's top left with health, or where they are; squad mates show by name over their heads
and in yellow on the map and minimap. Members **spawn on the leader** (beside them on foot, in a
free seat of their vehicle) wherever they are; a chat line starting `/s ` goes to the squad
only. Bots of the faction on the leader's continent fill a squad up to one short of full (a
human can always join, and a bot then makes room): they go to the waypoint, or follow the
leader, and spawn on it.

Bots also shoot the enemy's structures, deployables and turrets when there's nobody to fight
(launchers on the big ones), and infiltrator bots hack enemy terminals and turrets.

## Vehicles

From a base's vehicle terminal, onto its pad (E to get in and out, F to change seats):

| Vehicle | |
|---|---|
| **Rover** | a fast, cheap buggy: a driver and a gunner on a chaingun |
| **Light Tank** | fast, a cannon turning all the way round |
| **Light AAA** | the light tank with a four-barrelled flak gun whose shells burst near aircraft |
| **Heavy Tank** | slow, heavy cannon; a second seat with a chaingun |
| **Spawn Bus** | wheeled, five seats; **B** deploys it: a cloak hides it beyond 25 m from enemies, and its faction spawns at it |
| **VTOL** | Space hovers up, Ctrl down, W/S speed up and slow down, the mouse yaws and pitches, A/D roll; unguided rockets |
| **Interceptor** | flies like the VTOL, much faster: twin nose cannons, and homing missiles against aircraft on the right mouse button |
| **Dropship** | a slow transport: a pilot, two side gunners and seven passengers; squad members spawn into its free seats |
| **Harvester** | drive within 30 m of a resource node and hold fire: its beam locks on and mines it (the HUD shows the load), then park by a silo of your faction's to unload |

**Lock-ons**: the Skyhook locks on to an enemy aircraft held in its sights for 1.5 s (the
Striker on a ground vehicle the same way, aimed down its sights; the interceptor's missiles on an
aircraft ahead of its nose); a box shows the lock and its missile then homes on the target. The
crew of a vehicle being locked on are warned ("LOCK WARNING", "MISSILE INCOMING"); an
aircraft's pilot lets flares go with **B** (every 12 s), which breaks locks and leads missiles
astray.

**Vehicle loadouts**: at the vehicle terminal pick a vehicle, then its main gun (the light tank
and AAA: an autocannon instead; the heavy tank: a railgun; the VTOL: a nose cannon; the rover's
gunner: an autocannon) and its defensive slot, used with **B** by the driver: **Flares**
(aircraft), a **Smoke Screen** (breaks locks, hides it), **Fire Suppression** (a quick repair)
or passive **Armour Plating** (a fifth less damage); then ORDER.

Vehicles go through warp gates with everyone in them. V switches between the first and third
person views in vehicles and turrets. From inside nothing of the vehicle is in the way: a tank's
view (and a turret's) turns with its barrel, a ring showing where the gun is still turning to;
the bus's, the harvester's and the VTOL's look ahead. Seen from behind, guns aim at what the
crosshair is on. The vehicle aim and the flight pitch can be inverted in Options.

Nobody can fire inside a warp gate's bubble, and no shot goes through it.

## Controls (flat screen)

| | |
|---|---|
| WASD, mouse | move, look (in vehicles: drive, aim) |
| Shift, Ctrl, Space | sprint, crouch, jump (VTOL: hover down / up) |
| Left / right mouse | fire / aim (the lance's scope) |
| R, G, 1, 2, 3 | reload, grenade, rifle, sidearm, special |
| E | use: enter or leave vehicles, terminals, consoles; ride a zipline, let go |
| B, F, V | deploy the bus (aircraft: flares), change seat, vehicle camera |
| M | map: the world and the continent |
| P, T | squad panel, squad waypoint (the leader) |
| N | build panel (an engineer) |
| Q | spot the enemy aimed at |
| Tab (held) | scoreboard |
| Enter | chat (`/s ` first: to the squad) |
| Esc | menu |

On foot the view bobs, the gun sways and kicks, and hits, blasts and hard landings shake the
view (the head bob option turns it all off). Taking damage reddens the screen's edges (pulsing
at low health) with arcs pointing to where it came from; the hit marker turns amber on the
head, cyan on a vehicle, and magenta on a kill.

Keys can be changed in Options. The minimap (bottom right) shows the land round the player,
north up, with the bases, gates, resource nodes, the faction's players and vehicles, enemy
vehicles in sight and enemies who fire.

**VR** (`--vr`, or `--gl-vr`: our own foveated renderer for the Steam Frame): the left stick
walks or drives, the right turns; A jumps, B reloads, X throws a grenade, Y (tapped) spots the
enemy you look at (seated too: where the gun aims), the right stick pulled back cloaks (an
infiltrator) or puts the engineer's kit down. **Y held** brings up the **command wheel** round
the left hand: the left stick picks (walking and driving wait meanwhile), letting go of Y does
it, the stick centred cancels. On it: the squad waypoint where you look (the leader), switching
seats (in a vehicle), the map, the scoreboard, the squad panel, building (an engineer), the
cloak (an infiltrator), and callouts to your faction with the nearest base's name: medic,
repairs, a ride, attack here, defend here. The weapons are
holstered on the body: the rifle over the right shoulder, the sidearm on the right hip, the
special on the left (mirrored when left-handed). Grip near one to draw it, let go to put it
back (a ring glows and the hand ticks when one is in reach). The drawn gun's trigger fires, the
other hand's trigger uses. The gun sits turned nose down in the hand as in Neon Lasertag
(the gun tilt setting, `PS_GUN_TILT`) and fires along its barrel. Hits taken buzz harder in the
hand on their side, and the view's edges glow red. Seated, the view turns with
the seat (a tank's or turret's barrel, the hull ahead) and the right stick aims the gun; in the
VTOL the right stick pitches and yaws, the left stick speeds up and slows down, the grips climb
and sink. The right stick's click, held, fires a seat's second gun (the interceptor's missiles)
and locks the Striker on. Squad waypoints and the downed's crosses show in VR too. Menus, the spawn points, terminals and consoles come up as panels in front of you,
drawn over the world: point with either hand and pull the trigger (the menu's SQUAD panel
creates, joins and leaves squads). The menu button opens the
menu, a click of the left stick (not pushed) the map; the left wrist shows health, ammo (a
harvester's load) and what the trigger would use, the back of the left hand the minimap. `tools/frame.sh` builds, deploys and runs the game on a Steam Frame.

**Accounts**: the dedicated server keeps player accounts (`--accounts FILE`, default
`accounts.db`, SQLite; `--no-accounts` to do without): joining it you log in or register (the
password twice), on the flat screen or with VR's keyboard. Only a secret derived from the
password goes over the network, and the server keeps an Argon2 hash of that. The account keeps
your XP and your **stats** (kills, deaths, accuracy, headshots, kills by weapon, vehicles,
revives, hacks, structures built, time played: STATS in the pause or VR menu). Offline and
hosted worlds have no accounts (stats go by name).

**Killcam**: killed by someone, the view turns to the killer for a few seconds, with who, the
weapon, the distance and the killer's health and armour, before the spawn screen.

**Browser**: `trunk build --release` (see `docs/web.md`) builds a version that joins servers
through the master server's relay; `?join=NAME` links straight to a server.

## Command line options

```
neon-planetstrike [--vr | --gl-vr | --flat] [--name NAME] [--faction red|blue|green|auto]
                  [--offline] [--seed N] [--world small|medium|large] [--bots N]
                  [--difficulty easy|normal|hard]
                  [--host [PORT]] [--public] [--server-name NAME] [--connect HOST[:PORT]]
                  [--screenshot FILE --after SECS] [--quit-after SECS]

ps-server [--port 27430] [--name NAME] [--seed N] [--continents N] [--bots N]
          [--difficulty easy|normal|hard] [--state FILE] [--max-players N]
          [--master URL] [--private] [--no-lan] [--snapshot-threads N] [--quiet]
```

`--bots` is the number of bots on each contested continent. Without `--seed` the server keeps
the seed of its saved world.

## How it works

| Crate | |
|---|---|
| `crates/ps-sim` | The game without a renderer: world generation (`worldgen.rs`), terrain and themes, roads (`roads.rs`), the canyon caves (`caves.rs`) and ziplines (`zip.rs`), scenery, bases, the continent simulation (`sim.rs`: players, vehicles, turrets, projectiles with lag compensation, consoles), capture points (`capture.rs`: each continent steps its points and tells the server who holds two; the server runs the capture like a hack), vehicles' physics, world state and capture rules (`state.rs`), bots |
| `crates/ps-net` | UDP transport (reliable and unreliable channels), the protocol (postcard, quantized snapshots), the client, LAN discovery, master server reporting, the browser's WebSocket |
| `crates/ps-server` | The server: a hub thread (network, world state, transfers, warps, bot quotas, persistence) and **one thread per continent**, with scoped threads building that continent's snapshots |
| `crates/ps-master` | The server list and the browser's WebSocket-to-UDP relay |
| `src/` | The client: terrain (`terrain.rs`, `assets/shaders/terrain.wgsl`), scenery (`flora.rs`), bases (`bases.rs`), avatars, vehicles, effects, HUD, map, terminals, audio, menus, VR |

Clients rebuild the world from the seed; the server sends only state. Each player gets a
snapshot of what it can see: infantry within 500 m, ground vehicles within 900 m, aircraft
within 1400 m, a deployed enemy spawn bus only within 25 m. Snapshots go as **deltas** against
the last one the client acknowledged, and players and vehicles beyond 200 m are refreshed
every second snapshot, beyond 500 m every third (the client smooths over the gaps). Shots come
as effects; players predict their own movement and driving, and their own shots and where
they hit (the server's word on those impacts is then passed over). Bots find their way with a
navigation grid over each continent (`crates/ps-sim/src/nav.rs`).

## Development

```
cargo test --workspace                 # world rules, terrain, sim, vehicles, bots, protocol, server
tools/models/build_all.sh              # rebuild and check the models (Blender 5.2)
python3 tools/gen_sfx.py               # resynthesize the sounds
```

Debug environment variables:

| Variable | Effect |
|---|---|
| `PS_AUTOSPAWN=home\|front\|NAME\|C,B` | Spawn automatically when dead: sanctuary, a contested continent, a continent by name, base B of continent C |
| `PS_VIEW=x,y,z,tx,ty,tz` | Fix the camera (relative to the player with `PS_VIEW_REL=1`) |
| `PS_CHEAT=1` | Server: order vehicles and take loadouts from anywhere, without a cooldown; build for nothing |
| `PS_DEV_VEHICLE=rover\|light_tank\|light_aaa\|heavy_tank\|spawn_bus\|vtol\|interceptor\|dropship\|harvester` | Order a vehicle after spawning (with `PS_CHEAT=1`) |
| `PS_DEV_DRIVE=throttle,steer,lift` | Drive by itself |
| `PS_INSIDE=1` | Start in vehicles and turrets with the view from inside |
| `PS_FPS=1` | Log the frame rate |
| `PS_NOVSYNC=1` | Present without vsync (a hidden test window isn't throttled) |
| `PS_DAY=0..1` | Fix the time of day (0.5 noon) |
| `PS_SAY=TEXT` | Say this in the chat once spawned |
| `PS_SQUAD=1` | Start a squad once in the game, and put its waypoint 60 m ahead |
| `PS_FIRE=1` | Hold the trigger |
| `PS_ALT=1` | Hold the aim / second-gun button once locked on |
| `PS_DEV_LOADOUT=max\|burster\|mauler\|stealth\|spitfire\|mines\|ammo` | Take that loadout once spawned (with `PS_CHEAT=1`: anywhere) |
| `PS_DEV_BUILD=wall\|bastion\|tube\|silo` | Ask to build one at eight spots 25 m round, two seconds after a `PS_DEV_LOADOUT` (with `PS_CHEAT=1`: without carrying it, anywhere) |
| `PS_DEV_HELD=MESH` | Show that weapon model in one's own hand (screenshots) |
| `PS_DEV_DEPLOY=1` | Then press the deploy key (cloak, deployable) a second later |
| `PS_SHOW_SELF=1` | Draw one's own avatar (to look at with `PS_VIEW`) |
| `PS_BOT_ARMOR=max\|stealth` | Server: every bot added wears that suit |
| `PS_DEV_TARGET=1` | Server (with `PS_CHEAT=1`): an aircraft ordered is lifted 80 m and held there, an enemy VTOL hovering 220 m ahead to lock on to (it pops flares at the first missile) |
| `PS_SCORES=1` | Keep the scoreboard up |
| `PS_SPOT=1` | Press the spot key every second |
| `PS_ALERT_EVERY=SECS`, `PS_ALERT_FIRST=SECS`, `PS_ALERT_LENGTH=SECS` | Server: alerts every so often (1800), the first after (a third of that), lasting (900) |
| `PS_FEEL_TEST=1` | Fake hits taken (from round about) and given every 1.5 s, to check the damage and hit feedback |
| `PS_VR_MENU=1` | Open the VR menu at the start |
| `PS_XR_SHOT=SECS` | VR: save both eyes' images that long into the run |
| `PS_HOLSTER_<0\|1\|2>=x,y,z` | VR: move a holster (rifle, sidearm, special; m from the eyes along the body, x towards the gun side) |
| `PS_MASTER=URL` | Use another master server |
| `PS_MENU=top\|botmatch\|multiplayer\|internet\|lan` | Open the start menu at that screen |

### Builds and releases

CI (`.github/workflows/build.yml`) builds, tests and packages every push and pull request:
Linux x86_64/aarch64 tarballs and AppImages, a Windows zip and MSI, a macOS Apple silicon zip,
and the dedicated server (`ps-server`) and master server (`ps-master`) for Linux. `/build` in a
pull request comment replies with download links; pushing a `v*` tag publishes a GitHub
release. The `web` job builds the browser version (`docs/web.md`) into a zip to serve, and the
`pages` job puts each release's on GitHub Pages.

The packaging scripts also run locally on Linux:

```
tools/openxr_loader.sh [--arch aarch64]          # the OpenXR loader, into target/openxr-ARCH
tools/appimage.sh --build                        # x86_64 AppImage into dist/
tools/appimage.sh --arch aarch64 --build         # Steam Frame AppImage, cross-compiled
tools/package.sh game x86_64-unknown-linux-gnu --loader target/openxr-x86_64   # tarball
```
