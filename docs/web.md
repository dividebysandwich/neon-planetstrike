# The browser build

The browser build joins internet servers (through the master server's relay, see below),
built to WebAssembly with [Trunk](https://trunkrs.dev). It plays on the flat screen only, and
can't run an offline world of its own (the world's continents run on threads).

## Building and playing

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --release        # http://127.0.0.1:8080
trunk build --release        # dist/web: serve the folder as it is
```

`Trunk.toml` and `web/index.html` hold the setup. Each build makes two games: the WebGPU one in
`dist/web` and the WebGL2 one in `dist/web/gl` (see below). The `wasm-release` profile in
`Cargo.toml` builds quickly. Its comment lists the settings for a smaller download, which take
longer to build. Any static web server works, as long as it serves `.wasm` as
`application/wasm`. Its URLs are relative (`public_url` in `Trunk.toml`), so it can be served
under a path too. CI builds the folder in the `web` job of `.github/workflows/build.yml` and
uploads it as one zip (`planetstrike-<ref>-web.zip`).

The build also writes gzipped copies (`*.wasm.gz`, `*.js.gz`, from `web/post_build.sh`). With
nginx, `gzip_static on;` sends them to browsers that accept gzip, without compressing the wasm
on every request. The hashed wasm and script names can be cached forever. `index.html` should
not be.

**GitHub Pages.** The `pages` job of the same workflow puts the folder (without the `.gz`
copies: Pages compresses by itself) on <https://dividebysandwich.github.io/neon-planetstrike/>: for
each release tag (`v*`), and on a manual run of the workflow (Actions > Build > Run workflow,
or `gh workflow run build.yml --ref main`). Releases only, since the page joins the servers
players run. The repository's Pages source is "GitHub Actions" (Settings > Pages), and the
`github-pages` environment allows the `main` branch and `v*` tags to deploy.

**Browser.** The game runs on WebGPU: a current Chrome or Edge, Firefox 141 or later, or Safari
26, with hardware acceleration on. Without WebGPU, or without a WebGPU adapter, the page goes on
to the WebGL2 build in `gl/`. `?gl` asks for the WebGL2 build. Without WebGL2 either, the page
says so.

**Page parameters.**

| Parameter | Meaning |
|---|---|
| `?name=` | The player's name for this visit (default: the one set in OPTIONS, else "Player") |
| `?master=` | Another master server for the list (default `https://ps-master.playspoon.com`) |
| `?relay=` | Another relay to join through (default: the master server's) |
| `?gl` | The WebGL2 build |
| `?join=` | Joins this listed server (address or name) as soon as the list is in: a link to a server |

**Settings.** The settings (`src/settings.rs`) are kept in the browser's localStorage under
`planetstrike/settings.ron`. They belong to that browser and site.

**Not in the browser build:**

- VR
- the offline world, hosting a game, the LAN list, and joining a server that isn't on the public list (see below)
- the command line options and the `PS_*` environment variables
- QUIT (the tab is closed instead)
- reflections, in the WebGL2 build

## How it differs from the desktop build

- **Files the game reads itself.** The models (`assets/models/*.glb`) are compiled into the
  wasm by `build.rs` (`assets::embedded`). The fonts already were. Sounds and the shader are
  loaded by Bevy's asset server from `assets/` beside the page, and the page copies just those
  folders.
- **Time.** `Instant` in `ps-net` is `web_time`'s, since std's panics in the browser. The game
  itself uses Bevy's `Time`.
- **Random numbers.** `getrandom` takes the browser's crypto. That needs both its `wasm_js`
  feature and the `getrandom_backend` cfg flag in `.cargo/config.toml`.
- **Native-only code.** `ps-net`'s master server reports (`Reporter`), port mapping and LAN
  listening are left out of the wasm build or fail there. `game::HostNet` is uninhabited, since
  `Server::listen` can't bind a UDP socket in the browser.
- **Window.** Bevy draws into the page's `<canvas id="planetstrike">`, sized to the page, and keeps
  the browser's keys for the game. The page resumes the audio on the first click or key, as
  browsers keep it suspended until then.

## The WebGL2 build

Bevy picks WebGPU or WebGL2 when it is compiled, so there are two wasm builds: with the
`webgpu` feature (`web/index.html`) and without it. `web/post_build.sh`, the Trunk hook, builds
the second one into `gl/`. It runs Trunk again on `web/index.gl.html`, a copy of the page that it
writes. The copy is marked `data-gl` and has `<base href="../">`, so the assets come from the
WebGPU build's directory and aren't there twice. `crate::WEBGL2` is true in that build.

WebGL2 has no deferred rendering, so the screen-space reflections (`src/reflections.rs`) stay
off there and their option is hidden.

## Multiplayer in the browser

Browsers can't open UDP sockets. A page served over HTTPS may only connect with `wss://`
(WebSocket over TLS), WebTransport or WebRTC, all with a certificate the browser trusts. Game
servers are hosted by players, by IP address, without certificates. So the browser joins
through a relay on the master server, which has a certificate:

1. MULTIPLAYER lists the public servers (`GET /api/servers`). The master server answers with
   `Access-Control-Allow-Origin: *`.
2. JOIN on a listed server, or JOIN ADDRESS with a listed server's address or name, opens
   `wss://ps-master.playspoon.com/relay/<server id>` (`ps_net::relay_url`).
3. The relay (`crates/ps-master/src/relay.rs`) opens a UDP socket for this browser. It passes
   each binary WebSocket message on as one datagram to the server listed under that id, and
   each datagram back as one message.
4. In the game, `ps-net`'s transport sends through `Datagrams`: a UDP socket on the desktop, the
   WebSocket in the browser (`crates/ps-net/src/websocket.rs`). The connection layer above it is
   the same.

Game servers need nothing for this. They see an ordinary client at the master server's
address, one UDP port per browser.

The relay only passes on the game's packets (`PSTK` first) to servers on the list, at most 4
relays per client address and 512 in all, and at most 300 packets a second from each browser.
It closes a relay after 15 seconds without packets. It refuses servers on the master server's
own machine or LAN, except with `--relay-internal`. A refused join shows why on the server list
(the browser only says that the WebSocket was refused).

The cost: one more hop, and TCP's head-of-line blocking. A lost segment holds up the packets
behind it, which shows as a stutter rather than a lost snapshot. All traffic also goes through
the master server twice.

**Deploying.** The relay listens on a port of its own (`--relay-bind`, default
`127.0.0.1:27511`). Caddy sends `/relay/*` there and the rest to the list
(`crates/ps-master/deploy/Caddyfile`). See `crates/ps-master/README.md`.

**Testing it locally**, all on one machine:

```sh
cargo run -p ps-master -- --relay-internal              # list on :27510, relay on :27511
cargo run -p ps-server -- --master http://127.0.0.1:27510
trunk serve --release
# http://127.0.0.1:8080/?master=http://127.0.0.1:27510&relay=ws://127.0.0.1:27511
```

`cargo test -p ps-net --test relay` runs a client through the relay natively.
