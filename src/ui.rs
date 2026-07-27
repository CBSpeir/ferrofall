use std::time::Duration;

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, RichText, Stroke, StrokeKind,
    Vec2, pos2, vec2,
};
use web_time::Instant;

#[cfg(feature = "audio-lab")]
use crate::audio::{CompoundPreview, Cue};
use crate::game::{
    BOARD_HEIGHT, BOARD_WIDTH, Game, GameEvent, Point, Tetromino, VISIBLE_HEIGHT, VISIBLE_TOP,
    preview_offsets,
};
use crate::platform::BrowserSupportIssue;

const BACKGROUND: Color32 = Color32::from_rgb(7, 16, 24);
const SURFACE: Color32 = Color32::from_rgb(13, 26, 36);
const SURFACE_DEEP: Color32 = Color32::from_rgb(8, 18, 27);
const GRID: Color32 = Color32::from_rgb(30, 51, 64);
const BORDER: Color32 = Color32::from_rgb(76, 99, 112);
const TEXT: Color32 = Color32::from_rgb(218, 226, 230);
const MUTED: Color32 = Color32::from_rgb(126, 148, 160);
const AMBER: Color32 = Color32::from_rgb(225, 153, 42);
const LINE_FLASH_DURATION: Duration = Duration::from_millis(150);
const DROP_TRAIL_DURATION: Duration = Duration::from_millis(120);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Screen {
    Title,
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum UiAction {
    None,
    Play,
    Quit,
    Fullscreen,
    Pause,
    Resume,
    Restart,
    MainMenu,
    ToggleAudioControls,
    ToggleMute,
    SetAudioVolume(f32),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum TouchControlAction {
    Left,
    SoftDrop,
    Right,
    Hold,
    RotateCounterclockwise,
    RotateClockwise,
    HardDrop,
}

impl TouchControlAction {
    const ALL: [Self; 7] = [
        Self::Left,
        Self::SoftDrop,
        Self::Right,
        Self::Hold,
        Self::RotateCounterclockwise,
        Self::RotateClockwise,
        Self::HardDrop,
    ];

    pub(crate) const fn is_held(self) -> bool {
        matches!(self, Self::Left | Self::SoftDrop | Self::Right)
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::SoftDrop => "SOFT",
            Self::Right => "RIGHT",
            Self::Hold => "HOLD",
            Self::RotateCounterclockwise => "ROTATE LEFT",
            Self::RotateClockwise => "ROTATE RIGHT",
            Self::HardDrop => "HARD DROP",
        }
    }

    const fn text_glyph(self) -> Option<&'static str> {
        match self {
            Self::Left => Some("◀"),
            Self::SoftDrop => Some("SOFT"),
            Self::Right => Some("▶"),
            Self::Hold => Some("HOLD"),
            Self::RotateCounterclockwise | Self::RotateClockwise => None,
            Self::HardDrop => Some("DROP"),
        }
    }

    pub(crate) const fn data_label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::SoftDrop => "soft-drop",
            Self::Right => "right",
            Self::Hold => "hold",
            Self::RotateCounterclockwise => "rotate-ccw",
            Self::RotateClockwise => "rotate-cw",
            Self::HardDrop => "hard-drop",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LayoutMode {
    Desktop,
    CompactPortrait,
    CompactLandscape,
}

impl LayoutMode {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::CompactPortrait => "compact-portrait",
            Self::CompactLandscape => "compact-landscape",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UiOutput {
    pub(crate) action: UiAction,
}

impl UiOutput {
    const NONE: Self = Self {
        action: UiAction::None,
    };
}

#[derive(Clone, Copy)]
pub(crate) struct AudioUiState<'a> {
    pub(crate) volume: f32,
    pub(crate) muted: bool,
    pub(crate) available: bool,
    pub(crate) controls_open: bool,
    pub(crate) notice: Option<&'a str>,
    pub(crate) failure_reason: Option<&'a str>,
}

pub(crate) struct UiState<'a> {
    pub(crate) game: Option<&'a Game>,
    pub(crate) session_best: u64,
    pub(crate) effects: &'a VisualEffects,
    pub(crate) now: Instant,
    pub(crate) audio: AudioUiState<'a>,
    pub(crate) touch_controls: bool,
    pub(crate) active_touch_controls: &'a [TouchControlAction],
}

#[cfg(feature = "audio-lab")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AudioLabAction {
    None,
    Preview(Cue),
    PreviewCompound(CompoundPreview),
    Stop,
    ToggleMute,
    SetVolume(f32),
    SetRate(f32),
    SetPan(f32),
}

#[derive(Default)]
pub(crate) struct VisualEffects {
    line_flashes: Vec<LineFlash>,
    drop_trails: Vec<DropTrail>,
}

struct LineFlash {
    rows: Vec<usize>,
    started: Instant,
}

struct DropTrail {
    from: [Point; 4],
    to: [Point; 4],
    started: Instant,
}

impl VisualEffects {
    pub(crate) fn observe(&mut self, event: &GameEvent, now: Instant) {
        match event {
            GameEvent::Cleared { rows, .. } => self.line_flashes.push(LineFlash {
                rows: rows.clone(),
                started: now,
            }),
            GameEvent::HardDropped { from, to } => self.drop_trails.push(DropTrail {
                from: *from,
                to: *to,
                started: now,
            }),
            GameEvent::Moved { .. }
            | GameEvent::Rotated { .. }
            | GameEvent::Held
            | GameEvent::Grounded { .. }
            | GameEvent::PieceLocked { .. }
            | GameEvent::LevelChanged(_)
            | GameEvent::GameOver => {}
        }
    }

    pub(crate) fn clear(&mut self) {
        self.line_flashes.clear();
        self.drop_trails.clear();
    }

    pub(crate) fn retain_active(&mut self, now: Instant) {
        self.line_flashes
            .retain(|flash| now.duration_since(flash.started) < LINE_FLASH_DURATION);
        self.drop_trails
            .retain(|trail| now.duration_since(trail.started) < DROP_TRAIL_DURATION);
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.line_flashes.is_empty() || !self.drop_trails.is_empty()
    }
}

pub(crate) fn show(ui: &mut egui::Ui, screen: Screen, state: UiState<'_>) -> UiOutput {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BACKGROUND);
    paint_background_grid(ui.painter(), rect);

    let layout_mode = layout_mode(rect, state.touch_controls);

    let action = match screen {
        Screen::Title => show_title(ui, rect, state.touch_controls),
        Screen::Playing | Screen::Paused | Screen::GameOver => {
            let Some(game) = state.game else {
                return UiOutput::NONE;
            };
            show_game(ui, rect, game, screen, layout_mode, &state)
        }
    };

    let audio_action = show_audio_control(
        ui,
        rect,
        screen,
        state.audio,
        layout_mode,
        state.touch_controls,
    );
    let action = if audio_action != UiAction::None {
        audio_action
    } else {
        action
    };

    if screen == Screen::Playing
        && state.touch_controls
        && let Some(layout) = touch_control_layout(rect, layout_mode)
    {
        paint_touch_controls(ui, &layout, state.active_touch_controls);
    }

    UiOutput { action }
}

pub(crate) fn show_browser_support_issue(ui: &mut egui::Ui, issue: BrowserSupportIssue) {
    let rect = ui.max_rect();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, BACKGROUND);
    paint_background_grid(&painter, rect);

    let center = rect.center();
    paint_falling_mark(&painter, pos2(center.x, center.y - 120.0), 18.0);
    let (heading, detail) = match issue {
        BrowserSupportIssue::ViewportTooSmall => (
            "MAKE SOME ROOM",
            "Rotate the device or enlarge this window.\nPlayable safe area: 320 × 500 or 500 × 320.",
        ),
    };
    painter.text(
        pos2(center.x, center.y - 22.0),
        Align2::CENTER_CENTER,
        heading,
        display_font(34.0),
        TEXT,
    );
    painter.text(
        pos2(center.x, center.y + 42.0),
        Align2::CENTER_CENTER,
        detail,
        label_font(14.0),
        MUTED,
    );
}

#[cfg(feature = "audio-lab")]
pub(crate) fn show_audio_lab(
    ui: &mut egui::Ui,
    audio: AudioUiState<'_>,
    rate: f32,
    pan: f32,
) -> AudioLabAction {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BACKGROUND);
    paint_background_grid(ui.painter(), rect);
    let mut action = AudioLabAction::None;
    let mut volume = audio.volume;
    let mut preview_rate = rate;
    let mut preview_pan = pan;

    ui.vertical_centered(|ui| {
        ui.add_space(22.0);
        ui.label(
            RichText::new("FERROFALL AUDIO LAB")
                .font(display_font(30.0))
                .color(TEXT),
        );
        ui.label(
            RichText::new("Development-only cue and compound-event auditioning")
                .font(label_font(11.0))
                .color(MUTED),
        );
        ui.add_space(14.0);

        if !audio.available {
            ui.label(
                RichText::new(audio.failure_reason.unwrap_or("Audio output unavailable"))
                    .font(label_font(11.0))
                    .color(AMBER),
            );
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("VOLUME").font(label_font(10.0)).color(MUTED));
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(true))
                .changed()
            {
                action = AudioLabAction::SetVolume(volume);
            }
            if ui
                .button(if audio.muted {
                    "UNMUTE (M)"
                } else {
                    "MUTE (M)"
                })
                .clicked()
            {
                action = AudioLabAction::ToggleMute;
            }
            if ui.button("STOP ALL").clicked() {
                action = AudioLabAction::Stop;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("RATE").font(label_font(10.0)).color(MUTED));
            if ui
                .add(egui::Slider::new(&mut preview_rate, 0.75..=1.25).show_value(true))
                .changed()
            {
                action = AudioLabAction::SetRate(preview_rate);
            }
            ui.label(RichText::new("PAN").font(label_font(10.0)).color(MUTED));
            if ui
                .add(egui::Slider::new(&mut preview_pan, -0.35..=0.35).show_value(true))
                .changed()
            {
                action = AudioLabAction::SetPan(preview_pan);
            }
        });
        ui.add_space(12.0);

        egui::Grid::new("audio_lab_cues")
            .num_columns(4)
            .spacing(vec2(8.0, 8.0))
            .show(ui, |ui| {
                for (index, cue) in Cue::ALL.into_iter().enumerate() {
                    if ui
                        .add_sized(
                            vec2(145.0, 30.0),
                            egui::Button::new(RichText::new(cue.label()).font(label_font(10.0))),
                        )
                        .clicked()
                    {
                        action = AudioLabAction::Preview(cue);
                    }
                    if index % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(12.0);
        ui.label(
            RichText::new("COMPOUND EVENTS")
                .font(label_font(11.0))
                .color(MUTED),
        );
        ui.horizontal(|ui| {
            if ui.button("HARD DROP + T-SPIN COMBO").clicked() {
                action = AudioLabAction::PreviewCompound(CompoundPreview::TSpinCombo);
            }
            if ui.button("PERFECT FOUR + LEVEL").clicked() {
                action = AudioLabAction::PreviewCompound(CompoundPreview::PerfectFour);
            }
            if ui.button("GAME OVER + NEW BEST").clicked() {
                action = AudioLabAction::PreviewCompound(CompoundPreview::NewBest);
            }
        });
    });

    action
}

fn show_title(ui: &mut egui::Ui, rect: Rect, touch_controls: bool) -> UiAction {
    let painter = ui.painter().clone();
    let center = rect.center();
    let compact = layout_mode(rect, touch_controls) != LayoutMode::Desktop;
    let mark_y = if compact {
        rect.top() + rect.height() * 0.19
    } else {
        center.y - 154.0
    };
    paint_falling_mark(
        &painter,
        pos2(center.x, mark_y),
        if compact { 17.0 } else { 22.0 },
    );
    painter.text(
        pos2(
            center.x,
            if compact {
                rect.top() + rect.height() * 0.34
            } else {
                center.y - 55.0
            },
        ),
        Align2::CENTER_CENTER,
        "FERROFALL",
        display_font(if compact { 36.0 } else { 46.0 }),
        TEXT,
    );

    let play_y = if compact {
        rect.top() + rect.height() * 0.53
    } else {
        center.y + 28.0
    };
    let play_rect = Rect::from_center_size(pos2(center.x, play_y), vec2(210.0, 52.0));
    let secondary_rect = play_rect.translate(vec2(0.0, 64.0));
    let play = styled_button(ui, play_rect, "PLAY", true);
    let show_secondary = !cfg!(target_arch = "wasm32") || crate::platform::fullscreen_available();
    let secondary = show_secondary
        && styled_button(
            ui,
            secondary_rect,
            if cfg!(target_arch = "wasm32") {
                "FULLSCREEN"
            } else {
                "QUIT"
            },
            false,
        );

    painter.text(
        pos2(
            center.x,
            if show_secondary {
                secondary_rect.bottom() + if compact { 24.0 } else { 38.0 }
            } else {
                play_rect.bottom() + 32.0
            },
        ),
        Align2::CENTER_TOP,
        if touch_controls {
            "TWO-THUMB CONTROLS  ·  TAP PAUSE ANY TIME"
        } else {
            "ENTER  PLAY    ·    ESC  PAUSE    ·    R  RESTART    ·    M  MUTE"
        },
        label_font(if compact { 9.5 } else { 12.0 }),
        MUTED,
    );

    if play {
        UiAction::Play
    } else if secondary {
        if cfg!(target_arch = "wasm32") {
            UiAction::Fullscreen
        } else {
            UiAction::Quit
        }
    } else {
        UiAction::None
    }
}

fn show_game(
    ui: &mut egui::Ui,
    rect: Rect,
    game: &Game,
    screen: Screen,
    layout_mode: LayoutMode,
    state: &UiState<'_>,
) -> UiAction {
    let layout = GameLayout::new(rect, layout_mode);
    let painter = ui.painter().clone();

    painter.text(
        layout.header.left_top(),
        Align2::LEFT_TOP,
        "FERROFALL",
        display_font(if layout_mode == LayoutMode::Desktop {
            28.0
        } else {
            21.0
        }),
        TEXT,
    );
    let header_button = if layout_mode == LayoutMode::Desktop {
        vec2(38.0, 34.0)
    } else {
        Vec2::splat(48.0)
    };
    let pause_rect = Rect::from_min_size(
        pos2(layout.header.right() - header_button.x, layout.header.top()),
        header_button,
    );
    let pause_clicked = if screen == Screen::Playing {
        pause_button(ui, pause_rect)
    } else {
        false
    };

    match layout_mode {
        LayoutMode::Desktop => {
            paint_left_rail(&painter, layout.left, game, state.session_best);
            paint_next_rail(&painter, layout.right, game);
            paint_controls(&painter, layout.footer);
        }
        LayoutMode::CompactPortrait => {
            paint_compact_left_rail(&painter, layout.left, game, state.session_best);
            paint_compact_next_rail(&painter, layout.right, game, false);
        }
        LayoutMode::CompactLandscape => {
            let controls = state
                .touch_controls
                .then(|| touch_control_layout(rect, layout_mode))
                .flatten();
            let left_info = controls
                .as_ref()
                .map_or(layout.left, |controls| controls.info_rect(layout.left));
            let right_info = controls
                .as_ref()
                .map_or(layout.right, |controls| controls.info_rect(layout.right));
            paint_landscape_left_rail(&painter, left_info, game, state.session_best);
            paint_compact_next_rail(&painter, right_info, game, true);
        }
    }
    paint_board(&painter, layout.board, game, state.effects, state.now);

    if pause_clicked {
        return UiAction::Pause;
    }

    match screen {
        Screen::Paused => paint_overlay(ui, rect, Overlay::Paused),
        Screen::GameOver => paint_overlay(
            ui,
            rect,
            Overlay::GameOver {
                score: game.score(),
                best: state.session_best,
            },
        ),
        Screen::Title | Screen::Playing => UiAction::None,
    }
}

fn paint_background_grid(painter: &Painter, rect: Rect) {
    let spacing = 32.0;
    let color = Color32::from_rgba_unmultiplied(28, 49, 61, 70);
    let mut x = rect.left();
    while x <= rect.right() {
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(0.5, color),
        );
        x += spacing;
    }
    let mut y = rect.top();
    while y <= rect.bottom() {
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(0.5, color),
        );
        y += spacing;
    }
}

fn paint_falling_mark(painter: &Painter, center: Pos2, cell: f32) {
    let pieces = [
        (Tetromino::I, -3.0, -1.2),
        (Tetromino::T, -0.2, -2.3),
        (Tetromino::L, 2.4, -0.5),
    ];
    for (kind, x, y) in pieces {
        let anchor = pos2(center.x + x * cell, center.y + y * cell);
        paint_preview_piece(painter, kind, anchor, cell * 0.78, 210);
    }
}

fn paint_left_rail(painter: &Painter, rect: Rect, game: &Game, session_best: u64) {
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        "HOLD",
        label_font(14.0),
        MUTED,
    );
    let hold_rect = Rect::from_min_max(
        pos2(rect.left(), rect.top() + 28.0),
        pos2(rect.right(), rect.top() + 118.0),
    );
    painter.rect_filled(hold_rect, 2.0, SURFACE_DEEP);
    painter.rect_stroke(hold_rect, 2.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    if let Some(kind) = game.held_piece() {
        paint_preview_piece(
            painter,
            kind,
            hold_rect.center(),
            (hold_rect.width() / 5.5).min(20.0),
            if game.hold_available() { 255 } else { 105 },
        );
    }

    let mut y = hold_rect.bottom() + 28.0;
    y = paint_stat(painter, rect, y, "SCORE", format_number(game.score()));
    y = paint_stat(
        painter,
        rect,
        y,
        "BEST",
        format_number(session_best.max(game.score())),
    );
    y = paint_stat(painter, rect, y, "LEVEL", game.level().to_string());
    y = paint_stat(painter, rect, y, "LINES", game.lines().to_string());

    if let Some(combo) = game.combo().filter(|combo| *combo > 0) {
        painter.text(
            pos2(rect.left(), y + 4.0),
            Align2::LEFT_TOP,
            format!("COMBO ×{combo}"),
            label_font(13.0),
            AMBER,
        );
        y += 31.0;
    }
    if game.back_to_back() {
        painter.text(
            pos2(rect.left(), y),
            Align2::LEFT_TOP,
            "BACK-TO-BACK",
            label_font(12.0),
            AMBER,
        );
    }
}

fn paint_compact_left_rail(painter: &Painter, rect: Rect, game: &Game, session_best: u64) {
    let label_size = (rect.width() * 0.145).clamp(8.0, 11.0);
    let value_size = (rect.width() * 0.24).clamp(12.0, 18.0);
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        "HOLD",
        label_font(label_size),
        MUTED,
    );
    let hold_height = (rect.width() * 0.92)
        .clamp(48.0, 82.0)
        .min(rect.height() * 0.23);
    let hold_rect = Rect::from_min_size(
        pos2(rect.left(), rect.top() + label_size + 8.0),
        vec2(rect.width(), hold_height),
    );
    painter.rect_filled(hold_rect, 2.0, SURFACE_DEEP);
    painter.rect_stroke(hold_rect, 2.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    if let Some(kind) = game.held_piece() {
        paint_preview_piece(
            painter,
            kind,
            hold_rect.center(),
            (hold_rect.width() / 5.2).min(hold_rect.height() / 4.2),
            if game.hold_available() { 255 } else { 105 },
        );
    }

    let stats = [
        ("SCORE", format_number(game.score())),
        ("BEST", format_number(session_best.max(game.score()))),
        ("LEVEL", game.level().to_string()),
        ("LINES", game.lines().to_string()),
    ];
    let stats_top = hold_rect.bottom() + 10.0;
    let status_height = if game.combo().is_some_and(|combo| combo > 0) || game.back_to_back() {
        28.0
    } else {
        0.0
    };
    let stat_height = ((rect.bottom() - stats_top - status_height) / 4.0).max(31.0);
    for (index, (label, value)) in stats.into_iter().enumerate() {
        let top = stats_top + index as f32 * stat_height;
        painter.text(
            pos2(rect.left(), top),
            Align2::LEFT_TOP,
            label,
            label_font(label_size),
            MUTED,
        );
        painter.text(
            pos2(rect.left(), top + label_size + 4.0),
            Align2::LEFT_TOP,
            value,
            number_font(value_size),
            TEXT,
        );
    }

    let mut status_y = rect.bottom();
    if game.back_to_back() {
        painter.text(
            pos2(rect.left(), status_y),
            Align2::LEFT_BOTTOM,
            "B2B",
            label_font(label_size),
            AMBER,
        );
        status_y -= label_size + 4.0;
    }
    if let Some(combo) = game.combo().filter(|combo| *combo > 0) {
        painter.text(
            pos2(rect.left(), status_y),
            Align2::LEFT_BOTTOM,
            format!("COMBO ×{combo}"),
            label_font(label_size),
            AMBER,
        );
    }
}

fn paint_landscape_left_rail(painter: &Painter, rect: Rect, game: &Game, session_best: u64) {
    let label_size = 9.0;
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        "HOLD",
        label_font(label_size),
        MUTED,
    );
    let hold_rect = Rect::from_min_size(
        pos2(rect.left(), rect.top() + 16.0),
        vec2((rect.width() * 0.36).clamp(58.0, 76.0), 56.0),
    );
    painter.rect_filled(hold_rect, 2.0, SURFACE_DEEP);
    painter.rect_stroke(hold_rect, 2.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    if let Some(kind) = game.held_piece() {
        paint_preview_piece(
            painter,
            kind,
            hold_rect.center(),
            (hold_rect.width() / 5.2).min(13.0),
            if game.hold_available() { 255 } else { 105 },
        );
    }

    let right_x = hold_rect.right() + 10.0;
    paint_landscape_stat(
        painter,
        pos2(right_x, rect.top()),
        "SCORE",
        format_number(game.score()),
    );
    paint_landscape_stat(
        painter,
        pos2(right_x, rect.top() + 39.0),
        "BEST",
        format_number(session_best.max(game.score())),
    );
    paint_landscape_stat(
        painter,
        pos2(rect.left(), hold_rect.bottom() + 10.0),
        "LEVEL",
        game.level().to_string(),
    );
    paint_landscape_stat(
        painter,
        pos2(rect.left() + rect.width() * 0.36, hold_rect.bottom() + 10.0),
        "LINES",
        game.lines().to_string(),
    );

    let mut status = Vec::new();
    if let Some(combo) = game.combo().filter(|combo| *combo > 0) {
        status.push(format!("COMBO ×{combo}"));
    }
    if game.back_to_back() {
        status.push("BACK-TO-BACK".to_owned());
    }
    if !status.is_empty() {
        painter.text(
            rect.left_bottom(),
            Align2::LEFT_BOTTOM,
            status.join("  ·  "),
            label_font(9.0),
            AMBER,
        );
    }
}

fn paint_landscape_stat(painter: &Painter, pos: Pos2, label: &str, value: String) {
    painter.text(pos, Align2::LEFT_TOP, label, label_font(8.5), MUTED);
    painter.text(
        pos2(pos.x, pos.y + 13.0),
        Align2::LEFT_TOP,
        value,
        number_font(15.0),
        TEXT,
    );
}

fn paint_stat(painter: &Painter, rect: Rect, y: f32, label: &str, value: String) -> f32 {
    painter.text(
        pos2(rect.left(), y),
        Align2::LEFT_TOP,
        label,
        label_font(11.0),
        MUTED,
    );
    painter.text(
        pos2(rect.left(), y + 17.0),
        Align2::LEFT_TOP,
        value,
        number_font(22.0),
        TEXT,
    );
    let separator_y = y + 48.0;
    painter.line_segment(
        [
            pos2(rect.left(), separator_y),
            pos2(rect.right(), separator_y),
        ],
        Stroke::new(1.0, GRID),
    );
    y + 60.0
}

fn paint_next_rail(painter: &Painter, rect: Rect, game: &Game) {
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        "NEXT",
        label_font(14.0),
        MUTED,
    );
    let top = rect.top() + 28.0;
    let gap = 8.0;
    let slot_height = (rect.bottom() - top - gap * 4.0) / 5.0;
    for (index, kind) in game.next_pieces().enumerate() {
        let slot = Rect::from_min_max(
            pos2(rect.left(), top + index as f32 * (slot_height + gap)),
            pos2(
                rect.right(),
                top + index as f32 * (slot_height + gap) + slot_height,
            ),
        );
        painter.rect_filled(slot, 2.0, SURFACE_DEEP);
        painter.rect_stroke(slot, 2.0, Stroke::new(1.0, GRID), StrokeKind::Inside);
        paint_preview_piece(
            painter,
            kind,
            slot.center(),
            (slot.height() / 5.0).min(slot.width() / 5.5),
            255,
        );
    }
}

fn paint_compact_next_rail(painter: &Painter, rect: Rect, game: &Game, horizontal: bool) {
    let label_size = if horizontal { 9.0 } else { 10.0 };
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        "NEXT",
        label_font(label_size),
        MUTED,
    );
    let content = Rect::from_min_max(
        pos2(rect.left(), rect.top() + label_size + 8.0),
        rect.right_bottom(),
    );
    let gap = if horizontal { 3.0 } else { 5.0 };
    for (index, kind) in game.next_pieces().enumerate() {
        let slot = if horizontal {
            let width = (content.width() - gap * 4.0) / 5.0;
            Rect::from_min_size(
                pos2(content.left() + index as f32 * (width + gap), content.top()),
                vec2(width, content.height().min(62.0)),
            )
        } else {
            let height = (content.height() - gap * 4.0) / 5.0;
            Rect::from_min_size(
                pos2(
                    content.left(),
                    content.top() + index as f32 * (height + gap),
                ),
                vec2(content.width(), height),
            )
        };
        painter.rect_filled(slot, 2.0, SURFACE_DEEP);
        painter.rect_stroke(slot, 2.0, Stroke::new(1.0, GRID), StrokeKind::Inside);
        paint_preview_piece(
            painter,
            kind,
            slot.center(),
            (slot.width() / 5.3).min(slot.height() / 4.4),
            255,
        );
    }
}

fn paint_board(painter: &Painter, rect: Rect, game: &Game, effects: &VisualEffects, now: Instant) {
    painter.rect_filled(rect.expand(6.0), 2.0, SURFACE);
    painter.rect_stroke(
        rect.expand(6.0),
        2.0,
        Stroke::new(1.5, BORDER),
        StrokeKind::Inside,
    );
    painter.rect_filled(rect, 0.0, SURFACE_DEEP);
    let cell = rect.width() / BOARD_WIDTH as f32;

    for x in 0..=BOARD_WIDTH {
        let line_x = rect.left() + x as f32 * cell;
        painter.line_segment(
            [pos2(line_x, rect.top()), pos2(line_x, rect.bottom())],
            Stroke::new(0.75, GRID),
        );
    }
    for y in 0..=VISIBLE_HEIGHT {
        let line_y = rect.top() + y as f32 * cell;
        painter.line_segment(
            [pos2(rect.left(), line_y), pos2(rect.right(), line_y)],
            Stroke::new(0.75, GRID),
        );
    }

    for y in VISIBLE_TOP..BOARD_HEIGHT {
        for x in 0..BOARD_WIDTH {
            if let Some(kind) = game.board().cell(x, y) {
                paint_block(painter, board_cell_rect(rect, x, y, cell), kind, 255, false);
            }
        }
    }

    for block in game.ghost_blocks() {
        if block.y >= VISIBLE_TOP as i32 {
            paint_block(
                painter,
                board_cell_rect(rect, block.x as usize, block.y as usize, cell),
                game.active_kind(),
                88,
                true,
            );
        }
    }
    for block in game.active_blocks() {
        if block.y >= VISIBLE_TOP as i32 {
            paint_block(
                painter,
                board_cell_rect(rect, block.x as usize, block.y as usize, cell),
                game.active_kind(),
                255,
                false,
            );
        }
    }

    for trail in &effects.drop_trails {
        let progress =
            now.duration_since(trail.started).as_secs_f32() / DROP_TRAIL_DURATION.as_secs_f32();
        let alpha = ((1.0 - progress).clamp(0.0, 1.0) * 90.0) as u8;
        for (from, to) in trail.from.iter().zip(trail.to.iter()) {
            if to.y < VISIBLE_TOP as i32 {
                continue;
            }
            let start_y = from.y.max(VISIBLE_TOP as i32) as usize;
            let start = board_cell_rect(rect, from.x as usize, start_y, cell).center();
            let end = board_cell_rect(rect, to.x as usize, to.y as usize, cell).center();
            painter.line_segment(
                [start, end],
                Stroke::new(cell * 0.18, Color32::from_white_alpha(alpha)),
            );
        }
    }

    for flash in &effects.line_flashes {
        let progress =
            now.duration_since(flash.started).as_secs_f32() / LINE_FLASH_DURATION.as_secs_f32();
        let alpha = ((1.0 - progress).clamp(0.0, 1.0) * 180.0) as u8;
        for row in &flash.rows {
            if *row >= VISIBLE_TOP {
                let row_rect = Rect::from_min_size(
                    pos2(rect.left(), rect.top() + (*row - VISIBLE_TOP) as f32 * cell),
                    vec2(rect.width(), cell),
                );
                painter.rect_filled(
                    row_rect.shrink(1.0),
                    0.0,
                    Color32::from_rgba_unmultiplied(255, 238, 192, alpha),
                );
            }
        }
    }
}

fn board_cell_rect(board: Rect, x: usize, y: usize, cell: f32) -> Rect {
    Rect::from_min_size(
        pos2(
            board.left() + x as f32 * cell,
            board.top() + (y - VISIBLE_TOP) as f32 * cell,
        ),
        Vec2::splat(cell),
    )
}

fn paint_block(painter: &Painter, rect: Rect, kind: Tetromino, alpha: u8, ghost: bool) {
    let base = piece_color(kind);
    let color = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha);
    let block = rect.shrink(1.4);

    if ghost {
        painter.rect_filled(block, 0.0, Color32::from_black_alpha(18));
        painter.rect_stroke(block, 0.0, Stroke::new(1.4, color), StrokeKind::Inside);
        return;
    }

    painter.rect_filled(block, 1.0, color);
    let highlight = Color32::from_rgba_unmultiplied(255, 255, 255, alpha / 3);
    let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, alpha / 2);
    painter.line(
        vec![block.left_bottom(), block.left_top(), block.right_top()],
        Stroke::new(1.0, highlight),
    );
    painter.line(
        vec![block.right_top(), block.right_bottom(), block.left_bottom()],
        Stroke::new(1.0, shadow),
    );
    paint_inset_mark(painter, block.shrink(block.width() * 0.25), kind, alpha);
}

fn paint_inset_mark(painter: &Painter, rect: Rect, kind: Tetromino, alpha: u8) {
    let ink = Color32::from_rgba_unmultiplied(7, 16, 24, alpha.saturating_mul(2) / 3);
    let stroke = Stroke::new(1.0, ink);
    match kind {
        Tetromino::I => {
            painter.line_segment(
                [
                    pos2(rect.left(), rect.center().y),
                    pos2(rect.right(), rect.center().y),
                ],
                stroke,
            );
        }
        Tetromino::O => {
            painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
        }
        Tetromino::T => {
            painter.line_segment([rect.left_top(), rect.center_bottom()], stroke);
            painter.line_segment([rect.right_top(), rect.center_bottom()], stroke);
        }
        Tetromino::S => {
            painter.circle_filled(rect.left_center(), 1.2, ink);
            painter.circle_filled(rect.right_center(), 1.2, ink);
        }
        Tetromino::Z => {
            painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
            painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
        }
        Tetromino::J => {
            painter.line_segment([rect.left_top(), rect.left_bottom()], stroke);
            painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
        }
        Tetromino::L => {
            painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
            painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
        }
    }
}

fn paint_preview_piece(painter: &Painter, kind: Tetromino, center: Pos2, cell: f32, alpha: u8) {
    let offsets = preview_offsets(kind);
    let min_x = offsets.iter().map(|point| point.x).min().unwrap_or(0);
    let max_x = offsets.iter().map(|point| point.x).max().unwrap_or(0);
    let min_y = offsets.iter().map(|point| point.y).min().unwrap_or(0);
    let max_y = offsets.iter().map(|point| point.y).max().unwrap_or(0);
    let width = (max_x - min_x + 1) as f32 * cell;
    let height = (max_y - min_y + 1) as f32 * cell;
    let origin = pos2(center.x - width / 2.0, center.y - height / 2.0);

    for offset in offsets {
        let rect = Rect::from_min_size(
            pos2(
                origin.x + (offset.x - min_x) as f32 * cell,
                origin.y + (offset.y - min_y) as f32 * cell,
            ),
            Vec2::splat(cell),
        );
        paint_block(painter, rect, kind, alpha, false);
    }
}

fn paint_controls(painter: &Painter, rect: Rect) {
    let controls = [
        ("← →", "MOVE"),
        ("↓", "SOFT DROP"),
        ("Z / X", "ROTATE"),
        ("SPACE", "HARD DROP"),
        ("C", "HOLD"),
        ("ESC", "PAUSE"),
        ("M", "MUTE"),
    ];
    let segment = rect.width() / controls.len() as f32;
    for (index, (key, label)) in controls.into_iter().enumerate() {
        let center_x = rect.left() + segment * (index as f32 + 0.5);
        let key_width = (key.chars().count() as f32 * 6.4 + 13.0).max(25.0);
        let label_width = label.chars().count() as f32 * 5.6;
        let total_width = key_width + 7.0 + label_width;
        let key_rect = Rect::from_min_size(
            pos2(center_x - total_width / 2.0, rect.center().y - 12.0),
            vec2(key_width, 24.0),
        );
        painter.rect_filled(key_rect, 1.0, SURFACE_DEEP);
        painter.rect_stroke(key_rect, 1.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
        painter.text(
            key_rect.center(),
            Align2::CENTER_CENTER,
            key,
            label_font(9.5),
            TEXT,
        );
        painter.text(
            pos2(key_rect.right() + 7.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            label_font(8.5),
            MUTED,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct TouchControl {
    action: TouchControlAction,
    rect: Rect,
}

#[derive(Clone, Debug)]
pub(crate) struct TouchControlLayout {
    controls: Vec<TouchControl>,
    controls_top: f32,
}

impl TouchControlLayout {
    pub(crate) fn action_at(&self, pos: Pos2) -> Option<TouchControlAction> {
        self.controls
            .iter()
            .find(|control| control.rect.contains(pos))
            .map(|control| control.action)
    }

    pub(crate) fn held_action_at(&self, pos: Pos2) -> Option<TouchControlAction> {
        self.action_at(pos).filter(|action| action.is_held())
    }

    pub(crate) fn contains(&self, action: TouchControlAction, pos: Pos2) -> bool {
        self.controls
            .iter()
            .any(|control| control.action == action && control.rect.contains(pos))
    }

    pub(crate) fn metadata(&self) -> String {
        self.controls
            .iter()
            .map(|control| {
                format!(
                    "{}:{:.1},{:.1},{:.1},{:.1}",
                    control.action.data_label(),
                    control.rect.left(),
                    control.rect.top(),
                    control.rect.width(),
                    control.rect.height(),
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    fn info_rect(&self, side: Rect) -> Rect {
        Rect::from_min_max(
            side.left_top(),
            pos2(side.right(), (self.controls_top - 8.0).max(side.top())),
        )
    }

    #[cfg(test)]
    pub(crate) fn rect_for(&self, action: TouchControlAction) -> Rect {
        self.controls
            .iter()
            .find(|control| control.action == action)
            .map(|control| control.rect)
            .expect("touch control layout must contain every action")
    }
}

pub(crate) fn touch_control_layout(rect: Rect, mode: LayoutMode) -> Option<TouchControlLayout> {
    match mode {
        LayoutMode::Desktop => None,
        LayoutMode::CompactPortrait => portrait_touch_controls(rect),
        LayoutMode::CompactLandscape => landscape_touch_controls(rect),
    }
}

fn portrait_touch_controls(rect: Rect) -> Option<TouchControlLayout> {
    let layout = GameLayout::new(rect, LayoutMode::CompactPortrait);
    let area = layout.footer;
    let gap = 5.0;
    let target = ((area.height() - gap) / 2.0)
        .min((area.width() - gap * 3.0 - 16.0) / 5.0)
        .clamp(48.0, 68.0);
    let movement_width = target * 3.0 + gap * 2.0;
    let action_width = target * 2.0 + gap;
    if area.height() < target * 2.0 + gap || area.width() < movement_width + action_width + 16.0 {
        return None;
    }

    let top = area.center().y - (target * 2.0 + gap) / 2.0;
    let movement_left = area.left();
    let movement_top = top + target + gap;
    let action_left = area.right() - action_width;
    let mut controls = Vec::with_capacity(TouchControlAction::ALL.len());
    controls.push(TouchControl {
        action: TouchControlAction::Hold,
        rect: Rect::from_min_size(
            pos2(movement_left + (movement_width - target) / 2.0, top),
            Vec2::splat(target),
        ),
    });
    for (index, action) in [
        TouchControlAction::Left,
        TouchControlAction::SoftDrop,
        TouchControlAction::Right,
    ]
    .into_iter()
    .enumerate()
    {
        controls.push(TouchControl {
            action,
            rect: Rect::from_min_size(
                pos2(movement_left + index as f32 * (target + gap), movement_top),
                Vec2::splat(target),
            ),
        });
    }
    for (index, action) in [
        TouchControlAction::RotateCounterclockwise,
        TouchControlAction::RotateClockwise,
    ]
    .into_iter()
    .enumerate()
    {
        controls.push(TouchControl {
            action,
            rect: Rect::from_min_size(
                pos2(action_left + index as f32 * (target + gap), top),
                Vec2::splat(target),
            ),
        });
    }
    controls.push(TouchControl {
        action: TouchControlAction::HardDrop,
        rect: Rect::from_center_size(
            pos2(
                action_left + action_width / 2.0,
                movement_top + target / 2.0,
            ),
            vec2((target * 1.45).min(action_width), target),
        ),
    });

    Some(TouchControlLayout {
        controls,
        controls_top: top,
    })
}

fn landscape_touch_controls(rect: Rect) -> Option<TouchControlLayout> {
    let layout = GameLayout::new(rect, LayoutMode::CompactLandscape);
    let gap = 5.0;
    let target = ((layout.left.width() - gap * 2.0) / 3.0).clamp(48.0, 58.0);
    let movement_width = target * 3.0 + gap * 2.0;
    let action_width = target * 2.0 + gap;
    if layout.left.width() < movement_width
        || layout.right.width() < action_width
        || layout.left.height() < target * 2.0 + gap + 70.0
    {
        return None;
    }

    let movement_left = layout.left.center().x - movement_width / 2.0;
    let movement_top = layout.left.bottom() - target;
    let upper_top = movement_top - target - gap;
    let action_left = layout.right.center().x - action_width / 2.0;
    let mut controls = Vec::with_capacity(TouchControlAction::ALL.len());
    controls.push(TouchControl {
        action: TouchControlAction::Hold,
        rect: Rect::from_min_size(
            pos2(movement_left + (movement_width - target) / 2.0, upper_top),
            Vec2::splat(target),
        ),
    });
    for (index, action) in [
        TouchControlAction::Left,
        TouchControlAction::SoftDrop,
        TouchControlAction::Right,
    ]
    .into_iter()
    .enumerate()
    {
        controls.push(TouchControl {
            action,
            rect: Rect::from_min_size(
                pos2(movement_left + index as f32 * (target + gap), movement_top),
                Vec2::splat(target),
            ),
        });
    }
    for (index, action) in [
        TouchControlAction::RotateCounterclockwise,
        TouchControlAction::RotateClockwise,
    ]
    .into_iter()
    .enumerate()
    {
        controls.push(TouchControl {
            action,
            rect: Rect::from_min_size(
                pos2(action_left + index as f32 * (target + gap), upper_top),
                Vec2::splat(target),
            ),
        });
    }
    controls.push(TouchControl {
        action: TouchControlAction::HardDrop,
        rect: Rect::from_center_size(
            pos2(
                action_left + action_width / 2.0,
                movement_top + target / 2.0,
            ),
            vec2((target * 1.45).min(action_width), target),
        ),
    });

    Some(TouchControlLayout {
        controls,
        controls_top: upper_top,
    })
}

fn paint_touch_controls(
    ui: &mut egui::Ui,
    layout: &TouchControlLayout,
    active: &[TouchControlAction],
) {
    for control in &layout.controls {
        let response = ui.interact(
            control.rect,
            ui.make_persistent_id(("touch_control", control.action)),
            egui::Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                control.action.label(),
            )
        });
        let is_active = active.contains(&control.action) || response.is_pointer_button_down_on();
        let fill = if is_active {
            if control.action == TouchControlAction::HardDrop {
                Color32::from_rgb(229, 170, 62)
            } else {
                Color32::from_rgb(31, 51, 63)
            }
        } else {
            SURFACE_DEEP
        };
        let stroke = if control.action == TouchControlAction::HardDrop {
            AMBER
        } else if is_active {
            TEXT
        } else {
            BORDER
        };
        ui.painter().rect_filled(control.rect, 7.0, fill);
        ui.painter().rect_stroke(
            control.rect,
            7.0,
            Stroke::new(if is_active { 1.8 } else { 1.1 }, stroke),
            StrokeKind::Inside,
        );
        let foreground = if is_active && control.action == TouchControlAction::HardDrop {
            BACKGROUND
        } else {
            TEXT
        };
        match control.action {
            TouchControlAction::RotateCounterclockwise => {
                paint_rotation_icon(ui.painter(), control.rect, false, foreground);
            }
            TouchControlAction::RotateClockwise => {
                paint_rotation_icon(ui.painter(), control.rect, true, foreground);
            }
            action => {
                ui.painter().text(
                    control.rect.center(),
                    Align2::CENTER_CENTER,
                    action
                        .text_glyph()
                        .expect("non-rotation touch controls must have text glyphs"),
                    if matches!(
                        action,
                        TouchControlAction::Hold
                            | TouchControlAction::SoftDrop
                            | TouchControlAction::HardDrop
                    ) {
                        label_font((control.rect.height() * 0.20).clamp(10.0, 13.0))
                    } else {
                        display_font((control.rect.height() * 0.38).clamp(18.0, 25.0))
                    },
                    foreground,
                );
            }
        }
    }
}

fn paint_rotation_icon(painter: &Painter, rect: Rect, clockwise: bool, color: Color32) {
    let icon_size = (rect.height() * 0.5).clamp(24.0, 32.0);
    let radius = icon_size * 0.34;
    let stroke_width = (rect.height() * 0.045).clamp(2.2, 2.7);
    let start_angle = if clockwise {
        std::f32::consts::FRAC_PI_4
    } else {
        3.0 * std::f32::consts::FRAC_PI_4
    };
    let sweep = if clockwise {
        std::f32::consts::TAU * 0.75
    } else {
        -std::f32::consts::TAU * 0.75
    };
    let point_at = |angle: f32| {
        pos2(
            rect.center().x + radius * angle.cos(),
            rect.center().y + radius * angle.sin(),
        )
    };
    let segment_count = 24;
    let arc = (0..=segment_count)
        .map(|segment| point_at(start_angle + sweep * segment as f32 / segment_count as f32))
        .collect();
    painter.add(egui::Shape::line(arc, Stroke::new(stroke_width, color)));
    painter.circle_filled(point_at(start_angle), stroke_width / 2.0, color);

    let end_angle = start_angle + sweep;
    let tip = point_at(end_angle);
    let tangent = if clockwise {
        vec2(-end_angle.sin(), end_angle.cos())
    } else {
        vec2(end_angle.sin(), -end_angle.cos())
    };
    let normal = vec2(-tangent.y, tangent.x);
    let base_center = tip - tangent * (icon_size * 0.25);
    let half_width = icon_size * 0.14;
    painter.add(egui::Shape::convex_polygon(
        vec![
            tip,
            base_center + normal * half_width,
            base_center - normal * half_width,
        ],
        color,
        Stroke::NONE,
    ));
}

fn show_audio_control(
    ui: &mut egui::Ui,
    rect: Rect,
    screen: Screen,
    audio: AudioUiState<'_>,
    layout_mode: LayoutMode,
    touch_controls: bool,
) -> UiAction {
    let button_rect = match screen {
        Screen::Title => {
            let size = if touch_controls {
                Vec2::splat(48.0)
            } else {
                vec2(38.0, 34.0)
            };
            Rect::from_min_size(pos2(rect.right() - size.x - 16.0, rect.top() + 16.0), size)
        }
        Screen::Playing | Screen::Paused | Screen::GameOver => {
            let header = GameLayout::new(rect, layout_mode).header;
            if layout_mode == LayoutMode::Desktop {
                Rect::from_min_size(pos2(header.right() - 84.0, header.top()), vec2(38.0, 34.0))
            } else {
                Rect::from_min_size(
                    pos2(header.right() - 102.0, header.top()),
                    Vec2::splat(48.0),
                )
            }
        }
    };
    let response = ui
        .interact(
            button_rect,
            ui.make_persistent_id("audio_control_button"),
            if audio.available {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        )
        .on_hover_text(if audio.available {
            if audio.muted {
                "Sound muted (M)"
            } else {
                "Sound volume (M to mute)"
            }
        } else {
            audio.failure_reason.unwrap_or("Sound unavailable")
        });
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            audio.available,
            if audio.muted {
                "Sound muted"
            } else {
                "Sound volume"
            },
        )
    });

    let fill = if response.is_pointer_button_down_on() {
        Color32::from_rgb(25, 42, 53)
    } else if response.hovered() && audio.available {
        Color32::from_rgb(18, 34, 45)
    } else {
        SURFACE_DEEP
    };
    ui.painter().rect_filled(button_rect, 2.0, fill);
    ui.painter().rect_stroke(
        button_rect,
        2.0,
        Stroke::new(1.0, if audio.available { BORDER } else { GRID }),
        StrokeKind::Inside,
    );
    paint_speaker_icon(
        ui.painter(),
        button_rect,
        audio.muted || audio.volume <= 0.0,
        if audio.available { TEXT } else { MUTED },
    );

    if let Some(notice) = audio.notice {
        let notice_rect =
            Rect::from_center_size(pos2(rect.center().x, rect.top() + 42.0), vec2(190.0, 30.0));
        ui.painter().rect_filled(notice_rect, 2.0, SURFACE);
        ui.painter().rect_stroke(
            notice_rect,
            2.0,
            Stroke::new(1.0, BORDER),
            StrokeKind::Inside,
        );
        ui.painter().text(
            notice_rect.center(),
            Align2::CENTER_CENTER,
            notice,
            label_font(11.0),
            TEXT,
        );
    }

    if response.clicked() && audio.available {
        return UiAction::ToggleAudioControls;
    }
    if !audio.controls_open || !audio.available {
        return UiAction::None;
    }

    let panel_width = 270.0_f32.min(rect.width() - 16.0);
    let panel_left = (button_rect.right() - panel_width)
        .clamp(rect.left() + 8.0, rect.right() - panel_width - 8.0);
    let panel = Rect::from_min_size(
        pos2(panel_left, button_rect.bottom() + 8.0),
        vec2(panel_width, 100.0),
    );
    ui.painter().rect_filled(panel, 3.0, SURFACE);
    ui.painter()
        .rect_stroke(panel, 3.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    ui.painter().text(
        pos2(panel.left() + 14.0, panel.top() + 14.0),
        Align2::LEFT_TOP,
        "SOUND",
        label_font(11.0),
        MUTED,
    );
    ui.painter().text(
        pos2(panel.right() - 14.0, panel.top() + 14.0),
        Align2::RIGHT_TOP,
        format!("{}%", (audio.volume * 100.0).round() as u32),
        label_font(11.0),
        TEXT,
    );

    let mut volume = audio.volume;
    let slider_rect = Rect::from_min_size(
        pos2(panel.left() + 14.0, panel.top() + 38.0),
        vec2(120.0, 30.0),
    );
    let slider = ui
        .push_id("audio_volume_slider", |ui| {
            ui.put(
                slider_rect,
                egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false),
            )
        })
        .inner;
    let mute_rect = Rect::from_min_size(
        pos2(panel.right() - 122.0, panel.top() + 40.0),
        vec2(108.0, 26.0),
    );
    let mute = ui
        .push_id("audio_mute_button", |ui| {
            styled_button(
                ui,
                mute_rect,
                if audio.muted { "UNMUTE" } else { "MUTE" },
                false,
            )
        })
        .inner;
    ui.painter().text(
        pos2(panel.left() + 14.0, panel.bottom() - 13.0),
        Align2::LEFT_BOTTOM,
        "M toggles mute",
        label_font(9.0),
        MUTED,
    );

    if slider.changed() {
        UiAction::SetAudioVolume(volume)
    } else if mute {
        UiAction::ToggleMute
    } else {
        UiAction::None
    }
}

fn paint_speaker_icon(painter: &Painter, rect: Rect, muted: bool, color: Color32) {
    let center = rect.center();
    let body = Rect::from_center_size(pos2(center.x - 5.5, center.y), vec2(5.0, 8.0));
    painter.rect_filled(body, 0.0, color);
    painter.line_segment(
        [body.right_top(), pos2(center.x + 1.0, center.y - 6.0)],
        Stroke::new(1.5, color),
    );
    painter.line_segment(
        [body.right_bottom(), pos2(center.x + 1.0, center.y + 6.0)],
        Stroke::new(1.5, color),
    );
    painter.line_segment(
        [
            pos2(center.x + 1.0, center.y - 6.0),
            pos2(center.x + 1.0, center.y + 6.0),
        ],
        Stroke::new(1.5, color),
    );
    if muted {
        painter.line_segment(
            [
                pos2(center.x + 5.0, center.y - 5.0),
                pos2(center.x + 12.0, center.y + 5.0),
            ],
            Stroke::new(1.5, AMBER),
        );
        painter.line_segment(
            [
                pos2(center.x + 12.0, center.y - 5.0),
                pos2(center.x + 5.0, center.y + 5.0),
            ],
            Stroke::new(1.5, AMBER),
        );
    } else {
        painter.line_segment(
            [
                pos2(center.x + 5.0, center.y - 4.0),
                pos2(center.x + 8.0, center.y),
            ],
            Stroke::new(1.3, color),
        );
        painter.line_segment(
            [
                pos2(center.x + 8.0, center.y),
                pos2(center.x + 5.0, center.y + 4.0),
            ],
            Stroke::new(1.3, color),
        );
    }
}

enum Overlay {
    Paused,
    GameOver { score: u64, best: u64 },
}

fn paint_overlay(ui: &mut egui::Ui, rect: Rect, overlay: Overlay) -> UiAction {
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_black_alpha(205));
    let center = rect.center();
    let panel = Rect::from_center_size(
        center,
        vec2(
            360.0_f32.min(rect.width() - 16.0),
            330.0_f32.min(rect.height() - 16.0),
        ),
    );
    ui.painter().rect_filled(panel, 3.0, SURFACE);
    ui.painter()
        .rect_stroke(panel, 3.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);

    let (title, first_label, first_action, second_label, second_action, has_third) = match overlay {
        Overlay::Paused => (
            "PAUSED",
            "RESUME",
            UiAction::Resume,
            "RESTART",
            UiAction::Restart,
            true,
        ),
        Overlay::GameOver { .. } => (
            "GAME OVER",
            "RESTART",
            UiAction::Restart,
            "MAIN MENU",
            UiAction::MainMenu,
            false,
        ),
    };
    ui.painter().text(
        pos2(center.x, panel.top() + 48.0),
        Align2::CENTER_CENTER,
        title,
        display_font(32.0),
        TEXT,
    );

    if let Overlay::GameOver { score, best } = overlay {
        ui.painter().text(
            pos2(center.x, panel.top() + 91.0),
            Align2::CENTER_CENTER,
            format!(
                "SCORE {}    ·    BEST {}",
                format_number(score),
                format_number(best)
            ),
            label_font(12.0),
            MUTED,
        );
    }

    let button_height = 48.0;
    let button_gap = 10.0;
    let button_count = if has_third { 3.0 } else { 2.0 };
    let buttons_height = button_count * button_height + (button_count - 1.0) * button_gap;
    let first_top = panel.bottom() - 18.0 - buttons_height;
    let button_width = 210.0_f32.min(panel.width() - 32.0);
    let first_rect = Rect::from_min_size(
        pos2(center.x - button_width / 2.0, first_top),
        vec2(button_width, button_height),
    );
    let second_rect = first_rect.translate(vec2(0.0, button_height + button_gap));
    let third_rect = second_rect.translate(vec2(0.0, button_height + button_gap));

    if styled_button(ui, first_rect, first_label, true) {
        first_action
    } else if styled_button(ui, second_rect, second_label, false) {
        second_action
    } else if has_third && styled_button(ui, third_rect, "MAIN MENU", false) {
        UiAction::MainMenu
    } else {
        UiAction::None
    }
}

fn styled_button(ui: &mut egui::Ui, rect: Rect, label: &str, primary: bool) -> bool {
    let fill = if primary { AMBER } else { SURFACE_DEEP };
    let text_color = if primary { BACKGROUND } else { TEXT };
    ui.put(
        rect,
        egui::Button::new(
            RichText::new(label)
                .font(label_font(13.0))
                .color(text_color),
        )
        .fill(fill)
        .stroke(Stroke::new(1.0, if primary { AMBER } else { BORDER }))
        .corner_radius(2.0),
    )
    .clicked()
}

fn pause_button(ui: &mut egui::Ui, rect: Rect) -> bool {
    let response = ui.interact(
        rect,
        ui.make_persistent_id("pause_button"),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Pause")
    });
    let fill = if response.is_pointer_button_down_on() {
        Color32::from_rgb(25, 42, 53)
    } else if response.hovered() {
        Color32::from_rgb(18, 34, 45)
    } else {
        SURFACE_DEEP
    };
    ui.painter().rect_filled(rect, 2.0, fill);
    ui.painter()
        .rect_stroke(rect, 2.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    let bar_height = rect.height() * 0.36;
    let bar_width = 2.0;
    for offset in [-4.0, 4.0] {
        ui.painter().rect_filled(
            Rect::from_center_size(
                pos2(rect.center().x + offset, rect.center().y),
                vec2(bar_width, bar_height),
            ),
            0.0,
            TEXT,
        );
    }
    response.clicked()
}

fn piece_color(kind: Tetromino) -> Color32 {
    match kind {
        Tetromino::I => Color32::from_rgb(35, 203, 209),
        Tetromino::O => Color32::from_rgb(245, 197, 66),
        Tetromino::T => Color32::from_rgb(168, 85, 214),
        Tetromino::S => Color32::from_rgb(120, 196, 67),
        Tetromino::Z => Color32::from_rgb(227, 75, 75),
        Tetromino::J => Color32::from_rgb(59, 120, 216),
        Tetromino::L => Color32::from_rgb(238, 138, 46),
    }
}

fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn number_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

fn label_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

fn format_number(value: u64) -> String {
    let digits = value.to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push(',');
        }
        result.push(character);
    }
    result
}

struct GameLayout {
    header: Rect,
    left: Rect,
    board: Rect,
    right: Rect,
    footer: Rect,
}

pub(crate) fn layout_mode(rect: Rect, touch_controls: bool) -> LayoutMode {
    if !touch_controls && rect.width() >= 720.0 && rect.height() >= 560.0 {
        LayoutMode::Desktop
    } else if rect.height() >= rect.width() {
        LayoutMode::CompactPortrait
    } else {
        LayoutMode::CompactLandscape
    }
}

impl GameLayout {
    fn new(rect: Rect, mode: LayoutMode) -> Self {
        match mode {
            LayoutMode::Desktop => Self::desktop(rect),
            LayoutMode::CompactPortrait => Self::compact_portrait(rect),
            LayoutMode::CompactLandscape => Self::compact_landscape(rect),
        }
    }

    fn desktop(rect: Rect) -> Self {
        let margin = 24.0_f32.min(rect.width() * 0.03);
        let inner = rect.shrink(margin);
        let header_height = 54.0;
        let footer_height = 44.0;
        let vertical_gap = 12.0;
        let content_top = inner.top() + header_height + vertical_gap;
        let content_bottom = inner.bottom() - footer_height - vertical_gap;
        let available_height = (content_bottom - content_top).max(300.0);
        let horizontal_gap = 24.0_f32.min(inner.width() * 0.03);
        let minimum_rail = 142.0;
        let max_board_width =
            (inner.width() - 2.0 * minimum_rail - 2.0 * horizontal_gap).max(160.0);
        let board_width = (available_height / 2.0).min(max_board_width);
        let board_height = board_width * 2.0;
        let rail_width =
            ((inner.width() - board_width - 2.0 * horizontal_gap) / 2.0).clamp(minimum_rail, 220.0);
        let total_width = board_width + rail_width * 2.0 + horizontal_gap * 2.0;
        let left_x = inner.center().x - total_width / 2.0;
        let board_top = content_top + (available_height - board_height) / 2.0;

        let left = Rect::from_min_size(pos2(left_x, board_top), vec2(rail_width, board_height));
        let board = Rect::from_min_size(
            pos2(left.right() + horizontal_gap, board_top),
            vec2(board_width, board_height),
        );
        let right = Rect::from_min_size(
            pos2(board.right() + horizontal_gap, board_top),
            vec2(rail_width, board_height),
        );

        Self {
            header: Rect::from_min_max(
                inner.left_top(),
                pos2(inner.right(), inner.top() + header_height),
            ),
            left,
            board,
            right,
            footer: Rect::from_min_max(
                pos2(inner.left(), inner.bottom() - footer_height),
                inner.right_bottom(),
            ),
        }
    }

    fn compact_portrait(rect: Rect) -> Self {
        let margin = (rect.width() * 0.02).clamp(6.0, 14.0);
        let inner = rect.shrink2(vec2(margin, 6.0));
        let header_height = 48.0;
        let header_gap = 6.0;
        let controls_height = (inner.height() * 0.25).clamp(112.0, 154.0);
        let controls_gap = 6.0;
        let content_top = inner.top() + header_height + header_gap;
        let content_bottom = inner.bottom() - controls_height - controls_gap;
        let available_height = (content_bottom - content_top).max(250.0);
        let rail_min = if inner.width() >= 500.0 { 96.0 } else { 62.0 };
        let horizontal_gap = (inner.width() * 0.018).clamp(5.0, 14.0);
        let max_board_width = (inner.width() - rail_min * 2.0 - horizontal_gap * 2.0).max(125.0);
        let board_width = (available_height / 2.0).min(max_board_width);
        let board_height = board_width * 2.0;
        let rail_width = ((inner.width() - board_width - horizontal_gap * 2.0) / 2.0).max(rail_min);
        let total_width = board_width + rail_width * 2.0 + horizontal_gap * 2.0;
        let left_x = inner.center().x - total_width / 2.0;
        let board_top = content_top + (available_height - board_height) / 2.0;
        let left = Rect::from_min_size(pos2(left_x, board_top), vec2(rail_width, board_height));
        let board = Rect::from_min_size(
            pos2(left.right() + horizontal_gap, board_top),
            vec2(board_width, board_height),
        );
        let right = Rect::from_min_size(
            pos2(board.right() + horizontal_gap, board_top),
            vec2(rail_width, board_height),
        );

        Self {
            header: Rect::from_min_max(
                inner.left_top(),
                pos2(inner.right(), inner.top() + header_height),
            ),
            left,
            board,
            right,
            footer: Rect::from_min_max(
                pos2(inner.left(), inner.bottom() - controls_height),
                inner.right_bottom(),
            ),
        }
    }

    fn compact_landscape(rect: Rect) -> Self {
        let margin = (rect.height() * 0.02).clamp(6.0, 10.0);
        let inner = rect.shrink(margin);
        let header_height = 48.0;
        let content_top = inner.top() + header_height + 4.0;
        let content_height = (inner.bottom() - content_top).max(250.0);
        let board_height = content_height;
        let board_width = board_height / 2.0;
        let horizontal_gap = (inner.width() * 0.012).clamp(5.0, 10.0);
        let side_width = (inner.width() - board_width - horizontal_gap * 2.0) / 2.0;
        let left = Rect::from_min_size(
            pos2(inner.left(), content_top),
            vec2(side_width, board_height),
        );
        let board = Rect::from_min_size(
            pos2(left.right() + horizontal_gap, content_top),
            vec2(board_width, board_height),
        );
        let right = Rect::from_min_size(
            pos2(board.right() + horizontal_gap, content_top),
            vec2(side_width, board_height),
        );

        Self {
            header: Rect::from_min_max(
                inner.left_top(),
                pos2(inner.right(), inner.top() + header_height),
            ),
            left,
            board,
            right,
            footer: Rect::NOTHING,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_preserves_square_cells_at_minimum_window_size() {
        let layout = GameLayout::new(
            Rect::from_min_size(Pos2::ZERO, vec2(720.0, 560.0)),
            LayoutMode::Desktop,
        );
        assert!((layout.board.height() / layout.board.width() - 2.0).abs() < 0.0001);
        assert!(layout.left.width() >= 142.0);
        assert!(layout.right.right() <= 720.0);
    }

    #[test]
    fn title_screen_produces_paint_shapes() {
        let context = egui::Context::default();
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(960.0, 720.0))),
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    Screen::Title,
                    UiState {
                        game: None,
                        session_best: 0,
                        effects: &VisualEffects::default(),
                        now: Instant::now(),
                        audio: AudioUiState {
                            volume: 0.7,
                            muted: false,
                            available: true,
                            controls_open: false,
                            notice: None,
                            failure_reason: None,
                        },
                        touch_controls: false,
                        active_touch_controls: &[],
                    },
                );
            },
        );
        assert!(!output.shapes.is_empty());
    }

    #[test]
    fn unsupported_browser_screen_produces_paint_shapes() {
        let context = egui::Context::default();
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(720.0, 560.0))),
                ..Default::default()
            },
            |ui| show_browser_support_issue(ui, BrowserSupportIssue::ViewportTooSmall),
        );
        assert!(!output.shapes.is_empty());
    }

    #[test]
    fn portrait_touch_controls_preserve_minimum_target_size() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(320.0, 500.0));
        let layout = touch_control_layout(rect, LayoutMode::CompactPortrait).unwrap();
        for action in TouchControlAction::ALL {
            let target = layout.rect_for(action);
            assert!(target.width() >= 48.0);
            assert!(target.height() >= 48.0);
            assert!(rect.contains_rect(target));
        }
    }

    #[test]
    fn landscape_touch_controls_preserve_minimum_target_size() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(500.0, 320.0));
        let layout = touch_control_layout(rect, LayoutMode::CompactLandscape).unwrap();
        for action in TouchControlAction::ALL {
            let target = layout.rect_for(action);
            assert!(target.width() >= 48.0);
            assert!(target.height() >= 48.0);
            assert!(rect.contains_rect(target));
        }
    }

    #[test]
    fn responsive_mode_uses_space_and_input_capability() {
        let desktop = Rect::from_min_size(Pos2::ZERO, vec2(960.0, 720.0));
        assert_eq!(layout_mode(desktop, false), LayoutMode::Desktop);
        assert_eq!(layout_mode(desktop, true), LayoutMode::CompactLandscape);

        let phone = Rect::from_min_size(Pos2::ZERO, vec2(360.0, 640.0));
        assert_eq!(layout_mode(phone, false), LayoutMode::CompactPortrait);
        assert_eq!(layout_mode(phone, true), LayoutMode::CompactPortrait);
    }
}
