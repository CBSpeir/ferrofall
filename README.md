# Oxidefall

[![Build status][build-badge]][build]
[![Play Oxidefall online][play-badge]][play]
[![MIT or Apache 2.0 license][license-badge]][license]
[![Built with Rust and WebAssembly][technology-badge]][technology]

Oxidefall is a falling-block puzzle game written in Rust. Its primary release
is a static WebAssembly website rendered with `eframe` and egui. The same
deterministic engine and interface also build as a native desktop app. An
original adaptive industrial-electronic score and sound effects provide
optional audio on both targets.

![Oxidefall gameplay][gameplay]

## Run the website locally

Install rustup and [Trunk 0.21.14][trunk]. The repository pins Rust 1.97.1 and
installs the WebAssembly target through `rust-toolchain.toml`:

```sh
rustup show
cargo install --locked trunk --version 0.21.14
python3 tools/trunk.py serve
```

Then open <http://127.0.0.1:8080>. The production bundle is generated in
`dist/` with:

```sh
python3 tools/trunk.py build --release --cargo-profile web-release
python3 tools/check_web_bundle.py dist
```

The web release supports current desktop Chrome, Edge, Firefox, and Safari,
plus current iOS Safari and Android Chrome. Phones use a purpose-built touch
HUD in portrait or landscape; narrow keyboard windows use the same compact
layout without touch controls. The official phone target starts at a 360 by
640 CSS-pixel screen. After browser chrome and safe-area insets, the playable
canvas requires at least 320 by 500 logical pixels in portrait or 500 by 320
in landscape.

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

On touch devices, the lower controls provide Left, Soft Drop, Right, Hold,
both rotations, and Hard Drop. Movement buttons support press-and-hold and
sliding between directions. Hold and rotations fire on contact. Hard Drop fires
only when the finger lifts inside its button, so sliding away cancels it. Every
touch target is at least 48 by 48 CSS pixels, and independent contacts allow
movement and rotation at the same time.

The mouse is used only for menus and overlay buttons. Losing focus, hiding the
browser tab, or rotating a touch device automatically pauses the game and
clears held input. Returning focus never resumes automatically. Ordinary
mobile browser-toolbar resizing does not pause play.

The web title screen offers an optional Fullscreen action when the browser
supports it, but normal in-browser play is the baseline. Reloading, closing,
or navigating away abandons the active run without a confirmation dialog.
Oxidefall has no accounts, analytics, cookies, service worker, or remote
telemetry.

The settings control offers System, Light, and Dark appearance modes alongside
separate Music and Effects sliders. System follows live operating-system theme
changes, while explicit choices override them. Appearance, both volumes, and
the mute state are stored locally. Opening settings during a run pauses the
game; closing settings never resumes automatically. `M` globally mutes or
restores audio without resetting the music timeline. Music starts with
gameplay, follows level and stack danger, and suspends at the current position
when the game pauses. Audio starts only after player interaction and never
blocks gameplay.

## Gameplay

Oxidefall uses a seven-bag randomizer, five-piece preview queue, hold, ghost
piece, hard drop, soft drop, SRS rotation and kicks, modern scoring, combos,
back-to-back bonuses, perfect clears, T-spins, and level-based gravity.

The engine runs at 60 simulation steps per second. Horizontal input uses
engine-controlled delayed auto-shift and repeat timing, so operating-system
keyboard repeat settings do not affect play.

See [DESIGN.md](DESIGN.md) for the complete rules and architecture.

## Quality checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p oxidefall-core
cargo check --target wasm32-unknown-unknown
python3 tools/trunk.py build --release --cargo-profile web-release
python3 tools/check_web_bundle.py dist
npm ci
npx playwright install chromium
npm run test:web
python3 tools/generate_audio.py --check
python3 tools/generate_music.py --check
python3 tools/generate_fonts.py --check
```

Run `cargo run --features audio-lab` to open the development-only soundboard.
It previews individual cues, pitch and pan variants, compound scoring events,
and every adaptive music tier. Regenerate the Ogg Vorbis effects with
`python3 tools/generate_audio.py` and the synchronized music stems with
`python3 tools/generate_music.py`. FFmpeg is preferred, with the repository's
source-built Rust encoder available as a fallback. Regenerate the renamed UI
font subsets with `python3 tools/generate_fonts.py` and HarfBuzz `hb-subset`.

The release gate limits the compressed critical path to 1.5 MiB and the
complete runtime download to 3.5 MiB. CI treats those limits as hard failures
and archives optimized web-build timing data for trend review.

The GitHub Actions workflow runs native quality gates, builds the release
WebAssembly bundle, and exercises it in headless Chromium for pull requests,
pushes to `main` or `master`, manual runs, and release tags. Only a newly
created release tag can deploy the static output to GitHub Pages. The
repository must use GitHub Actions as its Pages source before the first
deployment.

## Publish a web release

Update `[package].version` in the root `Cargo.toml`, refresh `Cargo.lock`, and
merge the version bump to `master`. The internal `oxidefall-core` package does
not need to share the app version. After the version-bump commit passes CI,
create and push its matching release tag. For example:

```sh
git switch master
git pull --ff-only
git tag -a v0.2.0 -m "Oxidefall v0.2.0"
git push origin v0.2.0
```

Release tags use stable SemVer in the exact form `vX.Y.Z`. The version must
match the root package, be higher than every existing release tag, and point
to a commit contained in `master`. CI rebuilds and retests the tagged commit
before deployment. Lightweight and annotated tags are both accepted, but
moving an existing tag, dispatching the workflow manually, or rerunning CI for
a non-release event cannot publish. To retry a failed deployment, rerun the
original tag-triggered workflow.

## License

The source is available under either the MIT License or Apache License 2.0,
at your option. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).

The source licenses do not grant rights to the Oxidefall name or branding.
Generated sound effects and music stems in `assets/audio`, plus their
composition data in `assets/audio-metadata`, are separately dedicated to the
public domain under CC0 1.0.

Saira Condensed ExtraBold and IBM Plex Mono Medium are bundled under the SIL
Open Font License 1.1. Their license texts and source details are in
[`assets/fonts`](assets/fonts/README.md).

[build-badge]: https://img.shields.io/github/actions/workflow/status/CBSpeir/oxidefall/web.yml?branch=master&style=flat-square&logo=github&label=build
[build]: https://github.com/CBSpeir/oxidefall/actions/workflows/web.yml
[gameplay]: assets/gameplay.png
[license-badge]: https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-526b77?style=flat-square
[license]: #license
[play-badge]: https://img.shields.io/badge/Play%20Online-b9680f?style=flat-square
[play]: https://oxidefall.fyi
[technology-badge]: https://img.shields.io/badge/Rust%20%2B%20WebAssembly-526b77?style=flat-square&logo=rust&logoColor=white
[technology]: Cargo.toml
[trunk]: https://trunk-rs.github.io/trunk/
