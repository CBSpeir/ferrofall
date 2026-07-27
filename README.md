# Ferrofall

Ferrofall is a falling-block puzzle game written in Rust. Its primary release
is a static WebAssembly website rendered with `eframe` and egui. The same
deterministic engine and interface also build as a native desktop app.
Original industrial-electronic sound effects provide optional gameplay and
menu feedback on both targets.

## Run the website locally

Install the current stable Rust toolchain, the WebAssembly target, and
[Trunk][trunk]:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
trunk serve
```

Then open <http://127.0.0.1:8080>. The production bundle is generated in
`dist/` with:

```sh
trunk build --release
```

The first web release supports current desktop Chrome, Edge, Firefox, and
Safari. It requires a keyboard and a viewport of at least 720 by 560 logical
pixels. Touch-only and smaller devices receive an explanatory screen rather
than a compressed game layout.

## Run the native app

```sh
cargo run --release
```

The native build remains supported on macOS, Windows, and Linux. It keeps its
session-only best score, while the primary web build stores the best score in
the browser's local storage.

## Controls

- Left and Right arrows: move
- Down arrow: soft drop
- Space: hard drop
- Up arrow or `X`: rotate clockwise
- `Z`: rotate counterclockwise
- `C` or Left Shift: hold
- Escape: pause or resume
- `R`: restart while paused or after game over
- `M`: mute or unmute sound

The mouse is used only for menus and overlay buttons. Losing focus or hiding
the browser tab automatically pauses the game and clears held input.

The web title screen offers an optional Fullscreen action. Reloading or closing
the page abandons the active run without a confirmation dialog. Ferrofall has
no accounts, analytics, cookies, service worker, or remote telemetry.

The speaker control opens a master sound-effects volume slider. Volume and mute
state are stored locally on both web and native builds. Audio starts only after
player interaction and never blocks gameplay when output is unavailable.

## Gameplay

Ferrofall uses a seven-bag randomizer, five-piece preview queue, hold, ghost
piece, hard drop, soft drop, SRS rotation and kicks, modern scoring, combos,
back-to-back bonuses, perfect clears, T-spins, and level-based gravity.

The engine runs at 60 simulation steps per second. Horizontal input uses
engine-controlled delayed auto-shift and repeat timing, so operating-system
keyboard repeat settings do not affect play.

See [DESIGN.md](DESIGN.md) for the complete rules and architecture.

## Quality checks

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo check --target wasm32-unknown-unknown
trunk build --release
npm ci
npx playwright install chromium
npm run test:web
python3 tools/generate_audio.py --check
```

Run `cargo run --features audio-lab` to open the development-only soundboard.
It previews individual cues, pitch and pan variants, and compound scoring
events. Regenerate the original WAV bank with
`python3 tools/generate_audio.py`.

The GitHub Actions workflow runs native quality gates, builds the release
WebAssembly bundle, and exercises it in headless Chromium. Successful pushes
to `main` or `master` deploy the static output to GitHub Pages. The repository
must use GitHub Actions as its Pages source before the first deployment.

## License

The source is available under either the MIT License or Apache License 2.0,
at your option. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).

The source licenses do not grant rights to the Ferrofall name or branding.
Generated sound effects in `assets/audio` are separately dedicated to the
public domain under CC0 1.0.

[trunk]: https://trunk-rs.github.io/trunk/
