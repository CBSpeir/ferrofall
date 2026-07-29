mod board;
mod piece;
mod randomizer;
mod scoring;

use std::collections::VecDeque;

pub use board::{BOARD_HEIGHT, BOARD_WIDTH, Board, VISIBLE_HEIGHT, VISIBLE_TOP};
pub use piece::{Point, Tetromino, preview_offsets};
pub use scoring::{ClearResult, Spin};

use piece::{ActivePiece, Rotation, kick_tests};
use randomizer::SevenBag;
use scoring::ScoreState;

const GRAVITY_UNIT: u64 = 1_000_000;
const MAX_GRAVITY_UNITS: u64 = 20 * GRAVITY_UNIT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Left,
    Right,
    SoftDrop,
    RotateClockwise,
    RotateCounterclockwise,
    Hold,
    HardDrop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Press(Action),
    Release(Action),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MovementDirection {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameEvent {
    Moved {
        direction: MovementDirection,
        column: i32,
    },
    Rotated {
        direction: RotationDirection,
        column: i32,
    },
    Held,
    Grounded {
        column: i32,
    },
    HardDropped {
        from: [Point; 4],
        to: [Point; 4],
    },
    PieceLocked {
        column: i32,
    },
    Cleared {
        rows: Vec<usize>,
        result: ClearResult,
    },
    LevelChanged(u32),
    GameOver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameConfig {
    pub das_ticks: u32,
    pub arr_ticks: u32,
    pub lock_delay_ticks: u32,
    pub max_lock_resets: u8,
    pub soft_drop_units: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            das_ticks: 10,
            arr_ticks: 2,
            lock_delay_ticks: 30,
            max_lock_resets: 15,
            soft_drop_units: GRAVITY_UNIT / 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HorizontalDirection {
    Left,
    Right,
}

impl HorizontalDirection {
    const fn delta(self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct InputState {
    left: bool,
    right: bool,
    soft_drop: bool,
    horizontal: Option<HorizontalDirection>,
    das_elapsed: u32,
    arr_elapsed: u32,
}

impl InputState {
    fn reset_repeats(&mut self) {
        self.das_elapsed = 0;
        self.arr_elapsed = 0;
    }

    fn clear(&mut self) {
        *self = Self::default();
    }

    fn update_physical_only(&mut self, command: Command) {
        match command {
            Command::Press(Action::Left) => {
                self.left = true;
                self.horizontal = Some(HorizontalDirection::Left);
                self.reset_repeats();
            }
            Command::Press(Action::Right) => {
                self.right = true;
                self.horizontal = Some(HorizontalDirection::Right);
                self.reset_repeats();
            }
            Command::Release(Action::Left) => {
                self.left = false;
                if self.horizontal == Some(HorizontalDirection::Left) {
                    self.horizontal = self.right.then_some(HorizontalDirection::Right);
                    self.reset_repeats();
                }
            }
            Command::Release(Action::Right) => {
                self.right = false;
                if self.horizontal == Some(HorizontalDirection::Right) {
                    self.horizontal = self.left.then_some(HorizontalDirection::Left);
                    self.reset_repeats();
                }
            }
            Command::Press(Action::SoftDrop) => self.soft_drop = true,
            Command::Release(Action::SoftDrop) => self.soft_drop = false,
            Command::Press(
                Action::RotateClockwise
                | Action::RotateCounterclockwise
                | Action::Hold
                | Action::HardDrop,
            )
            | Command::Release(
                Action::RotateClockwise
                | Action::RotateCounterclockwise
                | Action::Hold
                | Action::HardDrop,
            ) => {}
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LastAction {
    Rotation { kick_index: usize },
}

pub struct Game {
    config: GameConfig,
    board: Board,
    active: ActivePiece,
    hold: Option<Tetromino>,
    hold_used: bool,
    queue: VecDeque<Tetromino>,
    bag: SevenBag,
    score: ScoreState,
    input: InputState,
    pending_commands: VecDeque<Command>,
    events: Vec<GameEvent>,
    gravity_accumulator: u64,
    lock_elapsed: u32,
    lock_resets: u8,
    lowest_origin_y: i32,
    last_action: Option<LastAction>,
    contact_reported: bool,
    game_over: bool,
}

impl Game {
    pub fn new(config: GameConfig, seed: u64) -> Self {
        let mut bag = SevenBag::new(seed);
        let mut queue = VecDeque::with_capacity(6);
        while queue.len() < 6 {
            queue.push_back(bag.next());
        }
        let first = queue
            .pop_front()
            .expect("the initial next queue must contain a piece");
        let active = ActivePiece::spawn(first);

        Self {
            config,
            board: Board::default(),
            active,
            hold: None,
            hold_used: false,
            queue,
            bag,
            score: ScoreState::default(),
            input: InputState::default(),
            pending_commands: VecDeque::new(),
            events: Vec::new(),
            gravity_accumulator: 0,
            lock_elapsed: 0,
            lock_resets: 0,
            lowest_origin_y: active.origin.y,
            last_action: None,
            contact_reported: false,
            game_over: false,
        }
    }

    pub fn apply(&mut self, command: Command) {
        self.pending_commands.push_back(command);
    }

    pub fn clear_input(&mut self) {
        self.pending_commands.clear();
        self.input.clear();
    }

    pub fn step(&mut self) {
        if self.game_over {
            self.pending_commands.clear();
            return;
        }

        let mut locked_this_tick = false;
        while let Some(command) = self.pending_commands.pop_front() {
            if locked_this_tick {
                self.input.update_physical_only(command);
                continue;
            }
            locked_this_tick = self.handle_command(command);
        }

        if locked_this_tick || self.game_over {
            return;
        }

        self.apply_horizontal_repeat();
        self.apply_gravity();

        if self.grounded() {
            if !self.contact_reported {
                self.events.push(GameEvent::Grounded {
                    column: self.sound_column(),
                });
                self.contact_reported = true;
            }
            self.lock_elapsed = self.lock_elapsed.saturating_add(1);
            if self.lock_elapsed >= self.config.lock_delay_ticks {
                self.lock_active();
            }
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn active_kind(&self) -> Tetromino {
        self.active.kind
    }

    pub fn active_blocks(&self) -> [Point; 4] {
        self.active.blocks()
    }

    pub fn ghost_blocks(&self) -> [Point; 4] {
        let mut ghost = self.active;
        while !self.board.collides(ghost.translated(0, 1)) {
            ghost = ghost.translated(0, 1);
        }
        ghost.blocks()
    }

    pub fn held_piece(&self) -> Option<Tetromino> {
        self.hold
    }

    pub fn hold_available(&self) -> bool {
        !self.hold_used && !self.game_over
    }

    pub fn next_pieces(&self) -> impl Iterator<Item = Tetromino> + '_ {
        self.queue.iter().copied().take(5)
    }

    pub fn score(&self) -> u64 {
        self.score.score()
    }

    pub fn lines(&self) -> u32 {
        self.score.lines()
    }

    pub fn level(&self) -> u32 {
        self.score.level()
    }

    pub fn combo(&self) -> Option<u32> {
        self.score.combo()
    }

    pub fn back_to_back(&self) -> bool {
        self.score.back_to_back()
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = GameEvent> + '_ {
        self.events.drain(..)
    }

    fn handle_command(&mut self, command: Command) -> bool {
        match command {
            Command::Press(action) => self.press(action),
            Command::Release(action) => {
                self.release(action);
                false
            }
        }
    }

    fn press(&mut self, action: Action) -> bool {
        match action {
            Action::Left => {
                if self.input.left {
                    return false;
                }
                self.input.left = true;
                self.input.horizontal = Some(HorizontalDirection::Left);
                self.input.reset_repeats();
                self.try_horizontal(-1);
            }
            Action::Right => {
                if self.input.right {
                    return false;
                }
                self.input.right = true;
                self.input.horizontal = Some(HorizontalDirection::Right);
                self.input.reset_repeats();
                self.try_horizontal(1);
            }
            Action::SoftDrop => {
                if self.input.soft_drop {
                    return false;
                }
                self.input.soft_drop = true;
                if self.try_descend(true) {
                    self.score.add_drop_points(1);
                }
            }
            Action::RotateClockwise => {
                self.try_rotate(true);
            }
            Action::RotateCounterclockwise => {
                self.try_rotate(false);
            }
            Action::Hold => {
                self.try_hold();
            }
            Action::HardDrop => {
                self.hard_drop();
                return true;
            }
        }

        false
    }

    fn release(&mut self, action: Action) {
        match action {
            Action::Left => {
                self.input.left = false;
                if self.input.horizontal == Some(HorizontalDirection::Left) {
                    self.input.horizontal = self.input.right.then_some(HorizontalDirection::Right);
                    self.input.reset_repeats();
                }
            }
            Action::Right => {
                self.input.right = false;
                if self.input.horizontal == Some(HorizontalDirection::Right) {
                    self.input.horizontal = self.input.left.then_some(HorizontalDirection::Left);
                    self.input.reset_repeats();
                }
            }
            Action::SoftDrop => self.input.soft_drop = false,
            Action::RotateClockwise
            | Action::RotateCounterclockwise
            | Action::Hold
            | Action::HardDrop => {}
        }
    }

    fn apply_horizontal_repeat(&mut self) {
        let Some(direction) = self.input.horizontal else {
            return;
        };

        if self.input.das_elapsed < self.config.das_ticks {
            self.input.das_elapsed += 1;
            if self.input.das_elapsed == self.config.das_ticks {
                self.try_horizontal(direction.delta());
            }
            return;
        }

        self.input.arr_elapsed += 1;
        if self.input.arr_elapsed >= self.config.arr_ticks {
            self.input.arr_elapsed = 0;
            self.try_horizontal(direction.delta());
        }
    }

    fn apply_gravity(&mut self) {
        let natural_gravity = gravity_units(self.score.level());
        let soft_drop_is_faster =
            self.input.soft_drop && self.config.soft_drop_units > natural_gravity;
        let rate = if soft_drop_is_faster {
            self.config.soft_drop_units
        } else {
            natural_gravity
        };
        self.gravity_accumulator = self.gravity_accumulator.saturating_add(rate);

        while self.gravity_accumulator >= GRAVITY_UNIT {
            if !self.try_descend(soft_drop_is_faster) {
                self.gravity_accumulator = 0;
                break;
            }
            if soft_drop_is_faster {
                self.score.add_drop_points(1);
            }
            self.gravity_accumulator -= GRAVITY_UNIT;
        }
    }

    fn try_horizontal(&mut self, dx: i32) -> bool {
        let candidate = self.active.translated(dx, 0);
        if self.board.collides(candidate) {
            return false;
        }

        let was_grounded = self.grounded();
        self.active = candidate;
        self.last_action = None;
        self.consume_lock_reset(was_grounded);
        self.events.push(GameEvent::Moved {
            direction: if dx < 0 {
                MovementDirection::Left
            } else {
                MovementDirection::Right
            },
            column: self.sound_column(),
        });
        true
    }

    fn try_descend(&mut self, manual: bool) -> bool {
        let candidate = self.active.translated(0, 1);
        if self.board.collides(candidate) {
            return false;
        }

        self.active = candidate;
        if manual {
            self.last_action = None;
        }
        if self.active.origin.y > self.lowest_origin_y {
            self.lowest_origin_y = self.active.origin.y;
            self.lock_elapsed = 0;
            self.lock_resets = 0;
        }
        true
    }

    fn try_rotate(&mut self, clockwise: bool) -> bool {
        let from = self.active.rotation;
        let to = if clockwise {
            from.clockwise()
        } else {
            from.counterclockwise()
        };
        let was_grounded = self.grounded();

        for (kick_index, (dx, dy)) in kick_tests(self.active.kind, from, to)
            .into_iter()
            .enumerate()
        {
            let candidate = self.active.rotated(to, dx, dy);
            if self.board.collides(candidate) {
                continue;
            }

            self.active = candidate;
            self.last_action = Some(LastAction::Rotation { kick_index });
            if self.active.origin.y > self.lowest_origin_y {
                self.lowest_origin_y = self.active.origin.y;
                self.lock_elapsed = 0;
                self.lock_resets = 0;
            } else {
                self.consume_lock_reset(was_grounded);
            }
            self.events.push(GameEvent::Rotated {
                direction: if clockwise {
                    RotationDirection::Clockwise
                } else {
                    RotationDirection::Counterclockwise
                },
                column: self.sound_column(),
            });
            return true;
        }

        false
    }

    fn consume_lock_reset(&mut self, was_grounded: bool) {
        if was_grounded && self.lock_resets < self.config.max_lock_resets {
            self.lock_resets += 1;
            self.lock_elapsed = 0;
        }
    }

    fn grounded(&self) -> bool {
        self.board.collides(self.active.translated(0, 1))
    }

    fn try_hold(&mut self) {
        if self.hold_used {
            return;
        }

        let outgoing = self.active.kind;
        let incoming = if let Some(held) = self.hold.replace(outgoing) {
            held
        } else {
            self.take_next_piece()
        };
        self.hold_used = true;
        self.events.push(GameEvent::Held);
        self.spawn_kind(incoming);
    }

    fn hard_drop(&mut self) {
        let from = self.active.blocks();
        let mut distance = 0_u64;
        while self.try_descend(true) {
            distance += 1;
        }
        if distance > 0 {
            self.score.add_drop_points(distance.saturating_mul(2));
        }
        let to = self.active.blocks();
        self.events.push(GameEvent::HardDropped { from, to });
        self.lock_active();
    }

    fn lock_active(&mut self) {
        let spin = self.detect_t_spin();
        let full_lock_out = self
            .active
            .blocks()
            .into_iter()
            .all(|block| block.y < VISIBLE_TOP as i32);

        self.board.lock(self.active);
        self.events.push(GameEvent::PieceLocked {
            column: self.sound_column(),
        });
        let rows = self.board.clear_full_rows();
        let perfect_clear = !rows.is_empty() && self.board.is_empty();
        let previous_level = self.score.level();
        let result = self
            .score
            .apply_clear(rows.len() as u8, spin, perfect_clear);

        if !rows.is_empty() || spin != Spin::None {
            self.events.push(GameEvent::Cleared { rows, result });
        }
        if self.score.level() != previous_level {
            self.events
                .push(GameEvent::LevelChanged(self.score.level()));
        }

        if full_lock_out {
            self.set_game_over();
            return;
        }

        self.hold_used = false;
        let next = self.take_next_piece();
        self.spawn_kind(next);
    }

    fn detect_t_spin(&self) -> Spin {
        if self.active.kind != Tetromino::T {
            return Spin::None;
        }
        let Some(LastAction::Rotation { kick_index }) = self.last_action else {
            return Spin::None;
        };

        let pivot = self.active.origin.translated(1, 1);
        let corners = [
            Point::new(pivot.x - 1, pivot.y - 1),
            Point::new(pivot.x + 1, pivot.y - 1),
            Point::new(pivot.x - 1, pivot.y + 1),
            Point::new(pivot.x + 1, pivot.y + 1),
        ];
        let occupied = corners.map(|corner| self.board.point_is_occupied(corner));
        if occupied.into_iter().filter(|value| *value).count() < 3 {
            return Spin::None;
        }

        let front = match self.active.rotation {
            Rotation::Spawn => [occupied[0], occupied[1]],
            Rotation::Right => [occupied[1], occupied[3]],
            Rotation::Reverse => [occupied[2], occupied[3]],
            Rotation::Left => [occupied[0], occupied[2]],
        };

        if front.into_iter().all(|value| value) || kick_index == 4 {
            Spin::Full
        } else {
            Spin::Mini
        }
    }

    fn take_next_piece(&mut self) -> Tetromino {
        let piece = self
            .queue
            .pop_front()
            .expect("the next queue must be kept populated");
        self.queue.push_back(self.bag.next());
        piece
    }

    fn spawn_kind(&mut self, kind: Tetromino) {
        self.active = ActivePiece::spawn(kind);
        self.gravity_accumulator = 0;
        self.lock_elapsed = 0;
        self.lock_resets = 0;
        self.lowest_origin_y = self.active.origin.y;
        self.last_action = None;
        self.contact_reported = false;
        self.input.reset_repeats();

        if self.board.collides(self.active) {
            self.set_game_over();
        }
    }

    fn set_game_over(&mut self) {
        if !self.game_over {
            self.game_over = true;
            self.events.push(GameEvent::GameOver);
        }
    }

    fn sound_column(&self) -> i32 {
        self.active
            .blocks()
            .into_iter()
            .map(|block| block.x)
            .sum::<i32>()
            / 4
    }
}

impl Point {
    const fn translated(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

fn gravity_units(level: u32) -> u64 {
    if level > 100 {
        return MAX_GRAVITY_UNITS;
    }

    let exponent = level.saturating_sub(1) as i32;
    let base = 0.8 - f64::from(level.saturating_sub(1)) * 0.007;
    if base <= 0.0 {
        return MAX_GRAVITY_UNITS;
    }

    let seconds_per_row = base.powi(exponent);
    if !seconds_per_row.is_finite() || seconds_per_row <= 0.0 {
        return MAX_GRAVITY_UNITS;
    }

    let rows_per_tick = 1.0 / (seconds_per_row * 60.0);
    ((rows_per_tick * GRAVITY_UNIT as f64).round() as u64).min(MAX_GRAVITY_UNITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn occupied_cells(game: &Game) -> usize {
        (0..BOARD_HEIGHT)
            .flat_map(|y| (0..BOARD_WIDTH).map(move |x| (x, y)))
            .filter(|(x, y)| game.board.cell(*x, *y).is_some())
            .count()
    }

    #[test]
    fn game_starts_with_five_previews() {
        let game = Game::new(GameConfig::default(), 7);
        assert_eq!(game.next_pieces().count(), 5);
        assert!(!game.is_game_over());
    }

    #[test]
    fn hard_drop_locks_four_cells_and_spawns_the_next_piece() {
        let mut game = Game::new(GameConfig::default(), 11);
        let expected_next = game.next_pieces().next().unwrap();
        game.apply(Command::Press(Action::HardDrop));
        game.step();

        assert_eq!(occupied_cells(&game), 4);
        assert_eq!(game.active_kind(), expected_next);
        assert_eq!(game.score(), 38);
    }

    #[test]
    fn hold_is_available_only_once_per_active_piece() {
        let mut game = Game::new(GameConfig::default(), 9);
        let first = game.active_kind();
        game.apply(Command::Press(Action::Hold));
        game.step();
        let after_first_hold = game.active_kind();
        assert_eq!(game.held_piece(), Some(first));
        assert!(!game.hold_available());

        game.apply(Command::Press(Action::Hold));
        game.step();
        assert_eq!(game.active_kind(), after_first_hold);
        assert_eq!(game.held_piece(), Some(first));
    }

    #[test]
    fn seeded_command_sequence_is_deterministic() {
        let mut first = Game::new(GameConfig::default(), 123);
        let mut second = Game::new(GameConfig::default(), 123);
        let commands = [
            Command::Press(Action::Left),
            Command::Release(Action::Left),
            Command::Press(Action::RotateClockwise),
            Command::Press(Action::HardDrop),
            Command::Press(Action::Hold),
            Command::Press(Action::Right),
            Command::Release(Action::Right),
            Command::Press(Action::HardDrop),
        ];

        for command in commands {
            first.apply(command);
            second.apply(command);
            first.step();
            second.step();
        }

        assert_eq!(first.board, second.board);
        assert_eq!(first.active, second.active);
        assert_eq!(first.score, second.score);
        assert_eq!(first.queue, second.queue);
    }

    #[test]
    fn t_spin_uses_three_corners_and_front_corner_classification() {
        let mut game = Game::new(GameConfig::default(), 1);
        game.active = ActivePiece {
            kind: Tetromino::T,
            rotation: Rotation::Spawn,
            origin: Point::new(3, 30),
        };
        game.last_action = Some(LastAction::Rotation { kick_index: 0 });
        game.board.set_cell(3, 30, Some(Tetromino::J));
        game.board.set_cell(5, 30, Some(Tetromino::J));
        game.board.set_cell(3, 32, Some(Tetromino::J));

        assert_eq!(game.detect_t_spin(), Spin::Full);
        game.board.set_cell(5, 30, None);
        game.board.set_cell(5, 32, Some(Tetromino::J));
        assert_eq!(game.detect_t_spin(), Spin::Mini);
    }

    #[test]
    fn blocked_spawn_ends_the_game() {
        let mut game = Game::new(GameConfig::default(), 5);
        let next = Tetromino::O;
        for block in ActivePiece::spawn(next).blocks() {
            game.board
                .set_cell(block.x as usize, block.y as usize, Some(Tetromino::Z));
        }
        game.spawn_kind(next);
        assert!(game.is_game_over());
    }

    #[test]
    fn gravity_starts_at_one_row_per_second_and_caps_at_twenty_g() {
        assert!((16_666..=16_667).contains(&gravity_units(1)));
        assert_eq!(gravity_units(101), MAX_GRAVITY_UNITS);
    }

    #[test]
    fn grounded_piece_locks_after_thirty_ticks() {
        let mut game = Game::new(GameConfig::default(), 2);
        game.active = ActivePiece {
            kind: Tetromino::O,
            rotation: Rotation::Spawn,
            origin: Point::new(3, 38),
        };
        game.lowest_origin_y = game.active.origin.y;

        for _ in 0..29 {
            game.step();
        }
        assert_eq!(occupied_cells(&game), 0);
        game.step();
        assert_eq!(occupied_cells(&game), 4);
    }

    #[test]
    fn first_ground_contact_emits_once_per_piece() {
        let mut game = Game::new(GameConfig::default(), 2);
        game.active = ActivePiece {
            kind: Tetromino::O,
            rotation: Rotation::Spawn,
            origin: Point::new(3, 38),
        };
        game.lowest_origin_y = game.active.origin.y;

        game.step();
        let first = game.drain_events().collect::<Vec<_>>();
        game.step();
        let second = game.drain_events().collect::<Vec<_>>();

        assert_eq!(
            first
                .iter()
                .filter(|event| matches!(event, GameEvent::Grounded { .. }))
                .count(),
            1
        );
        assert!(
            !second
                .iter()
                .any(|event| matches!(event, GameEvent::Grounded { .. }))
        );
    }

    #[test]
    fn presentation_events_only_report_successful_actions() {
        let mut game = Game::new(GameConfig::default(), 12);
        game.active.origin.x = -1;
        game.apply(Command::Press(Action::Left));
        game.step();
        assert!(
            !game
                .drain_events()
                .any(|event| matches!(event, GameEvent::Moved { .. }))
        );

        game.apply(Command::Release(Action::Left));
        game.apply(Command::Press(Action::Right));
        game.step();
        assert!(game.drain_events().any(|event| matches!(
            event,
            GameEvent::Moved {
                direction: MovementDirection::Right,
                ..
            }
        )));
    }

    #[test]
    fn standard_wall_kick_moves_a_t_piece_inside_the_left_wall() {
        let mut game = Game::new(GameConfig::default(), 3);
        game.active = ActivePiece {
            kind: Tetromino::T,
            rotation: Rotation::Right,
            origin: Point::new(-1, 30),
        };
        game.lowest_origin_y = game.active.origin.y;
        game.apply(Command::Press(Action::RotateClockwise));
        game.step();

        assert_eq!(game.active.rotation, Rotation::Reverse);
        assert_eq!(game.active.origin.x, 0);
    }

    #[test]
    fn line_clear_updates_score_lines_and_perfect_clear() {
        let mut game = Game::new(GameConfig::default(), 4);
        for x in 0..BOARD_WIDTH {
            if !(3..=6).contains(&x) {
                game.board.set_cell(x, BOARD_HEIGHT - 1, Some(Tetromino::J));
            }
        }
        game.active = ActivePiece {
            kind: Tetromino::I,
            rotation: Rotation::Spawn,
            origin: Point::new(3, 38),
        };
        game.lowest_origin_y = game.active.origin.y;
        game.apply(Command::Press(Action::HardDrop));
        game.step();

        assert!(game.board.is_empty());
        assert_eq!(game.lines(), 1);
        assert_eq!(game.score(), 900);
    }

    #[test]
    fn commands_after_a_lock_do_not_move_the_new_piece() {
        let mut game = Game::new(GameConfig::default(), 6);
        game.apply(Command::Press(Action::HardDrop));
        game.apply(Command::Press(Action::Right));
        game.step();
        assert_eq!(game.active.origin.x, 3);

        for _ in 0..10 {
            game.step();
        }
        assert_eq!(game.active.origin.x, 4);
    }

    #[test]
    fn most_recent_horizontal_press_wins_and_release_falls_back() {
        let mut game = Game::new(GameConfig::default(), 8);
        game.apply(Command::Press(Action::Left));
        game.apply(Command::Press(Action::Right));
        game.step();
        assert_eq!(game.active.origin.x, 3);

        game.apply(Command::Release(Action::Right));
        for _ in 0..10 {
            game.step();
        }
        assert_eq!(game.active.origin.x, 2);
    }

    #[test]
    fn full_lock_out_ends_game_but_partial_lock_out_is_allowed() {
        let mut full = Game::new(GameConfig::default(), 10);
        full.active = ActivePiece {
            kind: Tetromino::I,
            rotation: Rotation::Spawn,
            origin: Point::new(3, 17),
        };
        for x in 3..=6 {
            full.board.set_cell(x, 19, Some(Tetromino::Z));
        }
        full.apply(Command::Press(Action::HardDrop));
        full.step();
        assert!(full.is_game_over());

        let mut partial = Game::new(GameConfig::default(), 10);
        partial.active = ActivePiece {
            kind: Tetromino::I,
            rotation: Rotation::Right,
            origin: Point::new(-2, 17),
        };
        partial.board.set_cell(0, 21, Some(Tetromino::Z));
        partial.apply(Command::Press(Action::HardDrop));
        partial.step();
        assert!(!partial.is_game_over());
    }
}
