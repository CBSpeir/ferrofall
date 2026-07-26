# Ferrofall Design Specification

This document records the decision-complete web-primary milestone for
Ferrofall. Implementation and tests should treat it as the source of truth.

## Product scope

Ferrofall is a polished, single-player falling-block puzzle game. Its primary
release is a static WebAssembly website. A native desktop build remains
supported from the shared Rust codebase. Both targets use a
guideline-inspired modern ruleset and a restrained arcade interface.

The web-primary milestone includes:

- a 10-cell-wide playfield with 20 visible rows;
- seven-bag randomization and five next-piece previews;
- hold, ghost piece, hard drop, and soft drop;
- SRS rotation and kicks;
- modern scoring, levels, combos, back-to-back, and perfect clears;
- title, playing, paused, and game-over screens;
- persistent browser-local best score;
- a full-viewport web shell with loading and unsupported-device states;
- automated WebAssembly build and Chromium smoke tests;
- GitHub Pages deployment; and
- deterministic engine tests and native macOS verification.

The milestone excludes:

- audio;
- native persistent scores or window state;
- settings and control remapping;
- alternate themes;
- multiplayer;
- replay files or visible random seeds;
- installers and signing;
- mobile or touch controls;
- iframe embedding;
- accounts, leaderboards, analytics, cookies, or remote telemetry; and
- service-worker caching, offline support, or PWA installation.

## Technical architecture

The crate targets stable Rust 2024, forbids unsafe code in application
sources, and builds for native platforms and `wasm32-unknown-unknown`.

The dependency surface is intentionally small:

- `eframe` supplies native windowing, WebAssembly integration, and egui;
- egui is used through the `eframe` re-export;
- `rand` supplies fresh production seeds;
- `rand_chacha` supplies deterministic, portable bag generation;
- `web-time` supplies an `Instant` implementation that works on both targets;
  and
- browser-only dependencies enable JavaScript entropy, startup, DOM access,
  local storage, and fullscreen.

Native builds use the wgpu renderer and platform accessibility support. Web
builds use Glow with WebGL, including a WebGL 1 fallback, to support current
desktop Chrome, Edge, Firefox, and Safari without requiring WebGPU.

The source is divided by responsibility:

- `main.rs` configures the native window and generated icon;
- `main.rs` also starts `eframe::WebRunner` in browser builds;
- `app.rs` owns screens, focus, wall-clock accumulation, and key mapping;
- `platform.rs` isolates browser storage, support checks, accessible status,
  and fullscreen behavior;
- `ui.rs` owns layout, painting, overlays, and visual effects;
- `game/mod.rs` owns simulation and the command/event API;
- `game/board.rs` owns locked cells, collision, and row compaction;
- `game/piece.rs` owns tetromino geometry and SRS data;
- `game/randomizer.rs` owns the seeded seven-bag;
- `game/scoring.rs` owns score, line, combo, and level progression;
- `index.html` owns the static full-viewport shell and branded loader; and
- `Trunk.toml` and `.github/workflows/web.yml` own packaging, browser smoke
  tests, and Pages deployment.

The engine is internal. The web-primary milestone does not promise a reusable
library API.

## Engine boundary

The engine exposes a narrow internal control surface:

- construction from a `GameConfig` and random seed;
- queued press and release commands;
- one fixed simulation step;
- read-only state accessors; and
- drained typed events for presentation effects.

The engine never reads the wall clock and never paints UI. Menus and pause
state remain outside it.

Given the same seed and tick-indexed command sequence, the board, queue, score,
and active piece must reproduce exactly.

## Playfield and pieces

The internal matrix is 10 columns by 40 rows. Rows 20 through 39 are visible.
The upper 20 rows are a hidden spawn and kick buffer.

Locked cells are stored as a fixed array of optional tetromino kinds. The
active and ghost pieces remain separate from the locked matrix. Each active
piece stores its kind, rotation, and signed integer origin, then derives four
block coordinates from static tables.

Pieces spawn centered at the standard SRS horizontal position in or near the
hidden buffer. A held piece always returns to its spawn position and spawn
orientation.

## Randomization and hold

The randomizer shuffles all seven tetromino kinds, consumes the bag, then
shuffles a fresh bag. The UI shows the next five pieces.

Normal games use a fresh random seed. Tests may inject a fixed seed. Restart
creates a fresh seed and does not replay the previous sequence.

Hold may be used once per active piece:

- the first hold stores the active piece and consumes the next piece;
- later holds swap the active and stored pieces; and
- hold becomes available again only after the active piece locks.

## Simulation timing

The app advances the engine at 60 fixed steps per second. It accumulates real
time outside the engine, caps catch-up at 250 milliseconds, and discards excess
time after longer stalls.

The default timing configuration is:

- delayed auto-shift: 10 ticks, approximately 167 milliseconds;
- horizontal auto-repeat: 2 ticks, approximately 33 milliseconds;
- soft-drop repeat: 2 ticks, approximately 33 milliseconds;
- lock delay: 30 ticks, exactly 500 milliseconds; and
- maximum grounded manipulation resets: 15.

The most recently pressed horizontal direction wins. Releasing it restores a
still-held opposite direction and restarts delayed auto-shift.

Repeat charge resets when a piece spawns or enters from hold. Physical held
state remains, but the new piece receives a fresh delay.

Once a piece locks during a simulation step, later one-shot actions in that
step cannot manipulate the new piece. Press and release state is still
updated.

## Gravity and dropping

Level starts at 1 and advances after every ten cleared lines. Gravity uses:

```text
seconds_per_row = (0.8 - (level - 1) × 0.007) ^ (level - 1)
```

Gravity is represented as deterministic fixed-point rows per tick and capped
at 20 rows per simulation step. Levels and score multipliers continue after
gravity reaches the cap. High gravity still tests collision one row at a time.

While Down is held, the engine uses the faster of natural gravity and the
33-millisecond soft-drop rate. It does not add two independent downward rates.
Soft drop awards one point per manually accelerated row. At levels where
natural gravity is already faster, it awards no manual-drop points.

Hard drop awards two points per traversed row and locks immediately. Active
piece movement is never visually tweened between cells.

## Lock delay

A grounded piece locks after 500 milliseconds. A successful horizontal move
or rotation while grounded restarts the timer until 15 resets have been used.

Descending to a lower origin row restarts the timer and replenishes the reset
allowance. A wall kick that temporarily lifts a grounded piece pauses lock
time and consumes a reset, but does not replenish the allowance unless the
piece later descends to a genuinely lower origin row.

Soft drop does not bypass lock delay. Hard drop does.

## Rotation and T-spins

Ferrofall supports clockwise and counterclockwise rotation. It does not
support 180-degree rotation.

JLSTZ pieces use the standard SRS kick table. The I piece uses its separate SRS
table. The O piece remains visually stationary.

A T-spin requires:

- a T piece;
- the last successful qualifying action to be a rotation; and
- at least three occupied corners around the T pivot.

Playfield boundaries count as occupied corners. Two occupied front corners
classify a full T-spin. Otherwise the result is a mini, except the fifth SRS
kick test upgrades it to a full T-spin.

A successful player translation after rotation invalidates the pending
classification. This includes horizontal movement, soft drop, and a hard drop
that travels at least one row. Automatic gravity does not invalidate it. A
zero-distance hard drop preserves it.

## Line clearing and top-out

Rows clear and compact immediately in engine time. The UI may show a
150-millisecond flash, but presentation effects never block gameplay.

The game ends when:

- a new piece collides at its spawn position; or
- a piece locks with all four blocks above the visible playfield.

A piece that locks only partly above the visible playfield is allowed. Later
spawn collision can still end the game.

## Scoring

All base clear scores are multiplied by the level active when the piece locks.
The clear then updates line count and level. New gravity applies to the next
piece.

Ordinary clears award:

- single: 100;
- double: 300;
- triple: 500; and
- four lines: 800.

Full T-spins award:

- no lines: 400;
- single: 800;
- double: 1,200; and
- triple: 1,600.

T-spin minis award:

- no lines: 100;
- single: 200; and
- double: 400.

A difficult clear is a four-line clear or a line-clearing T-spin. Consecutive
difficult clears receive a 1.5-times back-to-back multiplier. A non-difficult
line clear breaks back-to-back. A zero-line placement does not.

Every consecutive line clear advances the combo counter. The first clear has
combo index zero. The bonus is:

```text
50 × combo index × level
```

A zero-line placement ends the combo.

Perfect-clear bonuses are:

- single: 800;
- double: 1,200;
- triple: 1,800;
- four lines: 2,000; and
- back-to-back four lines: 3,200.

Drop points are not multiplied by level. Score arithmetic saturates rather
than wrapping.

## App state and input

The app has four top-level game screens:

- title, with Play and a platform-specific secondary action;
- playing;
- paused, with Resume, Restart, and Main Menu; and
- game over, with final score, session best, Restart, and Main Menu.

The native secondary title action is Quit. The web secondary title action is
Fullscreen. The browser's Escape behavior may exit fullscreen before a later
Escape pauses the game.

Escape pauses and resumes. `R` restarts only while paused or after game over.
The mouse affects menus and overlay buttons only.

Losing window focus pauses immediately, freezes all simulation timers, and
clears held-key state. Returning focus does not resume automatically.

Hiding the browser tab follows the same rule. Reloading, closing, or navigating
away abandons the current run without a confirmation prompt. In-progress game
state is never serialized.

The web build stores only the best score in same-origin `localStorage`. It uses
a versioned key, tolerates unavailable or malformed storage, and has no account
or server synchronization. The native build retains a session-only best score.

The web shell keeps a visually hidden semantic status synchronized with the
canvas screen so assistive technology and browser smoke tests can identify the
current high-level state.

## Visual system

The native window opens at 960 by 720 logical points and has a 720 by 560
minimum. It is resizable, but size and position are not persisted. The web
canvas fills the browser viewport.

The website is a top-level, full-viewport application with no surrounding site
navigation. It is not designed for iframe embedding. A web viewport smaller
than 720 by 560 shows a branded resize prompt and pauses active gameplay.
Touch-only devices show a desktop-and-keyboard requirement instead of the game.
Normal layout resumes when the viewport again meets the requirement, but the
user must explicitly resume a game that was paused by resizing.

The primary layout uses:

- Hold and statistics in an open left rail;
- a strongly framed 10-by-20 board in the center;
- five Next previews in the right rail; and
- compact control hints below the board.

The board always preserves square cells. The complete layout remains centered
as the window grows.

The palette uses a true near-black navy background, charcoal-navy surfaces,
cool-gray text, blue-gray grid lines, standard saturated piece colors, and a
single warm amber status accent.

Gameplay visuals are procedural egui primitives. No external fonts ship with
the app. The native window icon is generated from embedded RGBA data, and the
web shell ships a matching SVG favicon.

Accessibility requirements include:

- full gameplay keyboard control;
- strong text contrast;
- scalable logical-point layout;
- a ghost distinguished by both outline and opacity;
- per-piece inset marks so color is not the only identifier; and
- no rapid flashing effects.

## Completion bar

The web-primary milestone is complete only when:

- all specified mechanics work;
- deterministic engine and UI smoke tests pass;
- default and minimum window layouts are usable;
- focus, pause, held input, and top-out edges are verified;
- the release build runs smoothly on macOS;
- the release WebAssembly bundle loads in a desktop browser without relevant
  console errors;
- title-to-playing interaction and the undersized-viewport gate pass in
  headless Chromium;
- current Safari and Firefox receive manual launch checks;
- the static bundle works from relative paths suitable for GitHub Pages;
- formatting, strict Clippy, and tests are clean; and
- this specification and the README match the implementation.
