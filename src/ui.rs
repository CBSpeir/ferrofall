use std::time::Duration;

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, RichText, Stroke, StrokeKind,
    Vec2, pos2, vec2,
};
use web_time::Instant;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UiAction {
    None,
    Play,
    Quit,
    Fullscreen,
    Pause,
    Resume,
    Restart,
    MainMenu,
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
            GameEvent::PieceLocked | GameEvent::LevelChanged(_) | GameEvent::GameOver => {}
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

pub(crate) fn show(
    ui: &mut egui::Ui,
    screen: Screen,
    game: Option<&Game>,
    session_best: u64,
    effects: &VisualEffects,
    now: Instant,
) -> UiAction {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, BACKGROUND);
    paint_background_grid(ui.painter(), rect);

    match screen {
        Screen::Title => show_title(ui, rect),
        Screen::Playing | Screen::Paused | Screen::GameOver => {
            let Some(game) = game else {
                return UiAction::None;
            };
            show_game(ui, rect, game, session_best, effects, now, screen)
        }
    }
}

pub(crate) fn show_browser_support_issue(ui: &mut egui::Ui, issue: BrowserSupportIssue) {
    let rect = ui.max_rect();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, BACKGROUND);
    paint_background_grid(&painter, rect);

    let center = rect.center();
    paint_falling_mark(&painter, pos2(center.x, center.y - 120.0), 18.0);
    let (heading, detail) = match issue {
        BrowserSupportIssue::TouchOnly => (
            "DESKTOP REQUIRED",
            "Ferrofall's first web release requires a keyboard\nand a desktop or laptop browser.",
        ),
        BrowserSupportIssue::ViewportTooSmall => (
            "MAKE SOME ROOM",
            "Enlarge this window or reduce browser zoom.\nMinimum playable viewport: 720 × 560.",
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

fn show_title(ui: &mut egui::Ui, rect: Rect) -> UiAction {
    let painter = ui.painter().clone();
    let center = rect.center();
    paint_falling_mark(&painter, pos2(center.x, center.y - 154.0), 22.0);
    painter.text(
        pos2(center.x, center.y - 55.0),
        Align2::CENTER_CENTER,
        "FERROFALL",
        display_font(46.0),
        TEXT,
    );

    let play_rect = Rect::from_center_size(pos2(center.x, center.y + 28.0), vec2(210.0, 48.0));
    let secondary_rect = play_rect.translate(vec2(0.0, 62.0));
    let play = styled_button(ui, play_rect, "PLAY", true);
    let secondary_label = if cfg!(target_arch = "wasm32") {
        "FULLSCREEN"
    } else {
        "QUIT"
    };
    let secondary = styled_button(ui, secondary_rect, secondary_label, false);

    painter.text(
        pos2(center.x, secondary_rect.bottom() + 38.0),
        Align2::CENTER_TOP,
        "ENTER  PLAY    ·    ESC  PAUSE    ·    R  RESTART",
        label_font(12.0),
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
    session_best: u64,
    effects: &VisualEffects,
    now: Instant,
    screen: Screen,
) -> UiAction {
    let layout = GameLayout::new(rect);
    let painter = ui.painter().clone();

    painter.text(
        layout.header.left_top(),
        Align2::LEFT_TOP,
        "FERROFALL",
        display_font(28.0),
        TEXT,
    );
    let pause_rect = Rect::from_min_size(
        pos2(layout.header.right() - 38.0, layout.header.top()),
        vec2(38.0, 34.0),
    );
    let pause_clicked = if screen == Screen::Playing {
        pause_button(ui, pause_rect)
    } else {
        false
    };

    paint_left_rail(&painter, layout.left, game, session_best);
    paint_board(&painter, layout.board, game, effects, now);
    paint_next_rail(&painter, layout.right, game);
    paint_controls(&painter, layout.footer);

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
                best: session_best,
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

enum Overlay {
    Paused,
    GameOver { score: u64, best: u64 },
}

fn paint_overlay(ui: &mut egui::Ui, rect: Rect, overlay: Overlay) -> UiAction {
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_black_alpha(205));
    let center = rect.center();
    let panel = Rect::from_center_size(center, vec2(360.0, 330.0));
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

    let first_rect = Rect::from_center_size(pos2(center.x, panel.top() + 164.0), vec2(210.0, 45.0));
    let second_rect = first_rect.translate(vec2(0.0, 58.0));
    let third_rect = second_rect.translate(vec2(0.0, 58.0));

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

impl GameLayout {
    fn new(rect: Rect) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_preserves_square_cells_at_minimum_window_size() {
        let layout = GameLayout::new(Rect::from_min_size(Pos2::ZERO, vec2(720.0, 560.0)));
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
                    None,
                    0,
                    &VisualEffects::default(),
                    Instant::now(),
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
}
