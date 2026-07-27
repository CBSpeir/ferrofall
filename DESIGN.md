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
- responsive portrait and landscape HUDs with multi-touch controls;
- automated WebAssembly build and Chromium smoke tests;
- GitHub Pages deployment; and
- deterministic engine tests and native macOS verification; and
- original sound effects with local volume and mute preferences.

The milestone excludes:

- native persistent scores or window state;
- general settings and control remapping;
- alternate themes;
- multiplayer;
- replay files or visible random seeds;
- installers and signing;
- gamepad input, haptics, or touch-control customization;
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
desktop Chrome, Edge, Firefox, and Safari plus current iOS Safari and Android
Chrome without requiring WebGPU. Other mobile browsers are best effort and are
never intentionally blocked by user-agent detection.

The source is divided by responsibility:

- `main.rs` configures the native window and generated icon;
- `main.rs` also starts `eframe::WebRunner` in browser builds;
- `app.rs` owns screens, lifecycle, wall-clock accumulation, keyboard mapping,
  and independent touch-contact state;
- `audio.rs` owns event-to-cue mapping, mixing, and the platform playback
  boundary;
- `platform.rs` isolates browser storage, input capability and viewport checks,
  accessible status, test metadata, and fullscreen behavior;
- `ui.rs` owns layout, painting, overlays, and visual effects;
- `game/mod.rs` owns simulation and the command/event API;
- `game/board.rs` owns locked cells, collision, and row compaction;
- `game/piece.rs` owns tetromino geometry and SRS data;
- `game/randomizer.rs` owns the seeded seven-bag;
- `game/scoring.rs` owns score, line, combo, and level progression;
- `assets/audio` contains generated mono WAV effects and `assets/audio.js`
  supplies browser Web Audio playback;
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
Fullscreen when the browser exposes that capability. Fullscreen is optional;
normal browser-viewport play is the baseline. The browser's Escape behavior
may exit fullscreen before a later Escape pauses the game.

Escape pauses and resumes. `R` restarts only while paused or after game over.
The mouse affects menus and overlay buttons only.

Touch gameplay uses seven labeled controls with targets of at least 48 by 48
CSS pixels. Left, Soft Drop, and Right form a slideable movement row. Hold is
isolated above that row. Counterclockwise and clockwise rotation sit together,
and Hard Drop is a larger separated action. Independent contacts allow a held
movement action and a rotation at the same time.

Movement controls emit normal engine press and release commands, preserving
delayed auto-shift and repeat timing. Sliding between movement buttons releases
the previous action and presses the new one. Hold and rotations fire on touch
start. Hard Drop arms on touch start, fires only on touch end inside its
button, and permanently disarms for that contact when the finger slides away.
Touch cancellation releases every action without firing Hard Drop.

Losing window focus pauses immediately, freezes all simulation timers, and
clears held keyboard and touch state. Returning focus does not resume
automatically.

Hiding the browser tab follows the same rule. Reloading, closing, or navigating
away abandons the current run without a confirmation prompt. In-progress game
state is never serialized.

Changing between portrait and landscape on a touch device pauses and clears
input before reflowing the HUD. The player must explicitly resume. Ordinary
viewport-height changes caused by expanding or collapsing browser chrome do
not pause play.

The web build stores its best score in same-origin `localStorage`. It uses a
versioned key, tolerates unavailable or malformed storage, and has no account
or server synchronization. The native build retains a session-only best score.
Both targets persist only the master sound volume and mute preference through
eframe storage. No game state is persisted.

The web shell keeps a visually hidden semantic status synchronized with the
canvas screen so assistive technology and browser smoke tests can identify the
current high-level state. Touch controls publish labeled egui button semantics.
The real-time canvas game does not claim complete screen-reader playability.

## Audio system

Audio is a presentation effect and never changes deterministic simulation
state. Successful engine actions emit typed events for movement, rotation,
hold, first ground contact, hard drop, lock, clears, level changes, and game
over. Failed movement and rotation attempts emit no event. The app drains each
simulation step as a batch so the mixer can prioritize simultaneous cues.

The sound bank has a restrained industrial-electronic character. Original
mono, 16-bit, 32-kHz WAV files are generated deterministically by
`tools/generate_audio.py`. The checked-in effects are licensed under CC0 1.0;
the generator follows the repository's source-code license. The complete bank
must remain below 750 KiB uncompressed.

The mix follows these rules:

- successful horizontal moves alternate between two quiet ticks, with small
  pitch differences for left and right;
- rotation, hold, first ground contact, hard drop, and lock have distinct cues;
- automatic gravity and individual soft-drop rows are silent;
- a clear replaces the ordinary lock cue while a hard-drop layer may remain;
- single, double, triple, and four-line clears escalate;
- T-spins, combos, back-to-back bonuses, and perfect clears add restrained
  accents;
- level-up pitch rises slightly, with an extra layer every five levels;
- ordinary game over uses a power-down cue, and a new best adds a flourish;
- board-position panning is limited to a narrow stereo range; and
- no more than 16 voices may play simultaneously.

The master sound-effects volume defaults to 70 percent. A speaker control is
available on title and game screens, including during active play. It opens an
inline volume slider and mute button. `M` toggles mute globally and produces a
short visual and accessible status confirmation. Audio never carries gameplay
information that is absent visually.

Intentional pause and resume have short cues. Pausing stops active sounds.
Focus loss, page hiding, and unsupported viewport transitions stop sounds
without playing a pause cue; returning focus never resumes either gameplay or
audio automatically.

Native playback uses Kira with predecoded static sounds. The web shell begins
fetching the same files only after the app is interactive, then uses Web Audio
and unlocks its context on the first pointer or keyboard gesture. Audio never
blocks the title screen. Initialization, decoding, device, and playback errors
are nonfatal: the game continues silently and disables the sound control when
the platform reports audio as unavailable.

The non-default `audio-lab` Cargo feature replaces the game UI with a
development soundboard. It previews every cue, rate and pan variations, and
representative compound events. Release builds do not enable this feature.

## Visual system

The native window opens at 960 by 720 logical points and has a 720 by 560
minimum. It is resizable, but size and position are not persisted. The web
canvas fills the browser viewport inside CSS safe-area insets.

The website is a top-level, full-viewport application with no surrounding site
navigation. It is not designed for iframe embedding. The official mobile
target is a current iOS Safari or Android Chrome browser on a 360-by-640 CSS
pixel phone or larger. After browser chrome and safe-area insets, the canvas
must provide at least 320 by 500 logical pixels in portrait or 500 by 320 in
landscape. A smaller safe canvas shows a branded resize-or-rotate prompt and
pauses active gameplay. Normal layout returns when the viewport fits, but the
player must explicitly resume.

The primary layout uses:

- Hold and statistics in an open left rail;
- a strongly framed 10-by-20 board in the center;
- five Next previews in the right rail; and
- compact control hints below the board.

A non-touch viewport of at least 720 by 560 uses the primary desktop layout.
Narrower keyboard viewports use the compact layout without touch controls.
Touch-primary devices use a compact layout at every size so controls remain
stable when a tablet has ample space.

Compact portrait keeps Hold and statistics left of the board, all five Next
previews right of it, and two thumb-control clusters below it. Compact
landscape keeps the board centered, merges Hold and statistics into the left
control zone, and merges the five Next previews into the right control zone.
Movement stays left and rotation plus Hard Drop stays right in both
orientations. Paused and game-over overlays hide all gameplay controls.

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
- labeled touch controls with targets of at least 48 by 48 CSS pixels;
- strong text contrast;
- scalable logical-point layout;
- a ghost distinguished by both outline and opacity;
- per-piece inset marks so color is not the only identifier; and
- sound used only as reinforcement for visible information; and
- no rapid flashing effects.

The canvas suppresses browser pan and pinch gestures because gameplay requires
independent multi-touch contacts. The web shell suppresses overscroll and
respects display cutouts and home-indicator safe areas. Reduced-motion
preferences disable optional motion without removing essential visible state.

## Completion bar

The web-primary milestone is complete only when:

- all specified mechanics work;
- deterministic engine and UI smoke tests pass;
- default, compact portrait, compact landscape, and tablet layouts are usable;
- focus, pause, orientation, multi-touch, held-input, and top-out edges are
  verified;
- the release build runs smoothly on macOS;
- the compressed critical web path remains at or below 3 MiB and shows the
  title within five seconds on a normal 4G connection;
- the release WebAssembly bundle loads in desktop and mobile-emulated Chromium
  without relevant console errors;
- keyboard and touch title-to-playing interactions, orientation pause,
  simultaneous contacts, responsive snapshots, and the undersized-viewport
  gate pass in headless Chromium;
- current desktop Safari and Firefox receive manual launch checks;
- one physical iPhone and one physical Android phone pass portrait, landscape,
  multi-touch, held release, orientation, audio, safe-area, browser-chrome,
  and background-resume checks;
- audio latency, balance, focus behavior, and preference persistence receive
  manual checks on the web and native macOS;
- generated audio files match `tools/generate_audio.py --check`;
- the static bundle works from relative paths suitable for GitHub Pages;
- formatting, strict Clippy, and tests are clean; and
- this specification and the README match the implementation.
