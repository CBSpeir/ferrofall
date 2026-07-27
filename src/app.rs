use std::collections::BTreeSet;
use std::time::Duration;

use eframe::egui::{self, Event, Key};
use web_time::Instant;

use crate::audio::{AudioSystem, Cue, DEFAULT_VOLUME};
use crate::game::{Action, Command, Game, GameConfig};
use crate::platform;
use crate::ui::{AudioUiState, Screen, UiAction, VisualEffects};

const SIMULATION_STEP: Duration = Duration::from_nanos(1_000_000_000 / 60);
const MAX_CATCH_UP: Duration = Duration::from_millis(250);
const AUDIO_NOTICE_DURATION: Duration = Duration::from_millis(1_200);
const AUDIO_VOLUME_KEY: &str = "ferrofall.audio-volume.v1";
const AUDIO_MUTED_KEY: &str = "ferrofall.audio-muted.v1";

pub(crate) struct FerrofallApp {
    screen: Screen,
    game: Option<Game>,
    session_best: u64,
    last_frame: Instant,
    accumulator: Duration,
    pressed_keys: BTreeSet<Key>,
    effects: VisualEffects,
    audio: AudioSystem,
    audio_controls_open: bool,
    audio_notice: Option<(String, Instant)>,
    #[cfg(feature = "audio-lab")]
    audio_lab_enabled: bool,
    #[cfg(feature = "audio-lab")]
    audio_lab_rate: f32,
    #[cfg(feature = "audio-lab")]
    audio_lab_pan: f32,
    accessible_status: String,
}

impl FerrofallApp {
    pub(crate) fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_egui(&context.egui_ctx);
        let volume = context
            .storage
            .and_then(|storage| storage.get_string(AUDIO_VOLUME_KEY))
            .and_then(|volume| volume.parse::<f32>().ok())
            .filter(|volume| volume.is_finite())
            .unwrap_or(DEFAULT_VOLUME)
            .clamp(0.0, 1.0);
        let muted = context
            .storage
            .and_then(|storage| storage.get_string(AUDIO_MUTED_KEY))
            .is_some_and(|muted| muted == "true");
        let app = Self::initial_state_with_audio(volume, muted);

        #[cfg(feature = "qa-screenshot")]
        {
            let mut app = app;
            if std::env::var_os("FERROFALL_QA_TITLE").is_none() {
                app.prepare_screenshot_state();
            }
            app
        }

        #[cfg(not(feature = "qa-screenshot"))]
        app
    }

    #[cfg(test)]
    fn initial_state() -> Self {
        Self::initial_state_with_audio(DEFAULT_VOLUME, false)
    }

    fn initial_state_with_audio(volume: f32, muted: bool) -> Self {
        Self {
            screen: Screen::Title,
            game: None,
            session_best: platform::load_best_score(),
            last_frame: Instant::now(),
            accumulator: Duration::ZERO,
            pressed_keys: BTreeSet::new(),
            effects: VisualEffects::default(),
            audio: AudioSystem::new(volume, muted),
            audio_controls_open: false,
            audio_notice: None,
            #[cfg(feature = "audio-lab")]
            audio_lab_enabled: true,
            #[cfg(feature = "audio-lab")]
            audio_lab_rate: 1.0,
            #[cfg(feature = "audio-lab")]
            audio_lab_pan: 0.0,
            accessible_status: String::new(),
        }
    }

    #[cfg(feature = "qa-screenshot")]
    fn prepare_screenshot_state(&mut self) {
        let mut game = Game::new(GameConfig::default(), 0xF3_22_0F_A1);
        game.apply(Command::Press(Action::Hold));
        game.step();

        let placements = [-4_i32, 3, -2, 2, 0, -3, 3, -1, 1];
        for (index, offset) in placements.into_iter().enumerate() {
            if index % 3 == 1 {
                game.apply(Command::Press(Action::RotateClockwise));
                game.step();
            }
            let direction = if offset < 0 {
                Action::Left
            } else {
                Action::Right
            };
            for _ in 0..offset.unsigned_abs() {
                game.apply(Command::Press(direction));
                game.step();
                game.apply(Command::Release(direction));
                game.step();
            }
            game.apply(Command::Press(Action::HardDrop));
            game.step();
            if game.is_game_over() {
                break;
            }
        }

        game.drain_events().for_each(drop);
        self.session_best = game.score().saturating_add(8_240);
        self.game = Some(game);
        self.screen = Screen::Playing;
    }

    fn start_game(&mut self) {
        self.audio.activate();
        self.audio.stop_all();
        self.audio.play_ui(Cue::GameStart);
        self.audio_controls_open = false;
        let seed = rand::random::<u64>();
        self.game = Some(Game::new(GameConfig::default(), seed));
        self.screen = Screen::Playing;
        self.last_frame = Instant::now();
        self.accumulator = Duration::ZERO;
        self.pressed_keys.clear();
        self.effects.clear();
    }

    fn pause(&mut self, intentional: bool) {
        if self.screen == Screen::Playing {
            self.screen = Screen::Paused;
            self.clear_gameplay_input();
            self.audio.stop_all();
            if intentional {
                self.audio.play_ui(Cue::Pause);
            }
        }
    }

    fn handle_focus(&mut self, focused: bool) {
        if !focused && self.screen == Screen::Playing {
            self.pause(false);
        }
    }

    fn resume(&mut self) {
        if self.screen == Screen::Paused {
            self.screen = Screen::Playing;
            self.last_frame = Instant::now();
            self.accumulator = Duration::ZERO;
            self.pressed_keys.clear();
            self.audio.play_ui(Cue::Resume);
        }
    }

    fn return_to_title(&mut self) {
        self.update_session_best();
        self.screen = Screen::Title;
        self.game = None;
        self.accumulator = Duration::ZERO;
        self.pressed_keys.clear();
        self.effects.clear();
        self.audio.stop_all();
    }

    fn clear_gameplay_input(&mut self) {
        self.pressed_keys.clear();
        if let Some(game) = self.game.as_mut() {
            game.clear_input();
        }
    }

    fn update_session_best(&mut self) {
        if let Some(score) = self.game.as_ref().map(Game::score) {
            self.record_best_score(score);
        }
    }

    fn record_best_score(&mut self, score: u64) {
        if score > self.session_best {
            self.session_best = score;
            platform::save_best_score(score);
        }
    }

    fn set_accessible_status(&mut self, screen: &str, message: String) {
        if self.accessible_status != message {
            platform::set_accessible_status(screen, &message);
            self.accessible_status = message;
        }
    }

    fn handle_key_event(&mut self, key: Key, pressed: bool, repeat: bool) {
        if repeat {
            return;
        }

        if pressed {
            if !self.pressed_keys.insert(key) {
                return;
            }
            if self.handle_screen_key(key) {
                return;
            }
        } else if !self.pressed_keys.remove(&key) {
            return;
        }

        if self.screen != Screen::Playing {
            return;
        }
        let Some(action) = key_to_action(key) else {
            return;
        };

        if pressed {
            if !self.other_pressed_key_maps_to(key, action)
                && let Some(game) = self.game.as_mut()
            {
                game.apply(Command::Press(action));
            }
        } else if !self.any_pressed_key_maps_to(action)
            && let Some(game) = self.game.as_mut()
        {
            game.apply(Command::Release(action));
        }
    }

    fn handle_screen_key(&mut self, key: Key) -> bool {
        if key == Key::M {
            self.toggle_mute();
            return true;
        }
        match (self.screen, key) {
            (Screen::Title, Key::Enter) => {
                self.start_game();
                true
            }
            (Screen::Playing, Key::Escape) => {
                self.pause(true);
                true
            }
            (Screen::Paused, Key::Escape) => {
                self.resume();
                true
            }
            (Screen::Paused | Screen::GameOver, Key::R) => {
                self.start_game();
                true
            }
            _ => false,
        }
    }

    fn other_pressed_key_maps_to(&self, key: Key, action: Action) -> bool {
        self.pressed_keys
            .iter()
            .any(|pressed| *pressed != key && key_to_action(*pressed) == Some(action))
    }

    fn any_pressed_key_maps_to(&self, action: Action) -> bool {
        self.pressed_keys
            .iter()
            .any(|pressed| key_to_action(*pressed) == Some(action))
    }

    fn advance_game(&mut self, now: Instant) {
        if self.screen != Screen::Playing {
            self.last_frame = now;
            self.accumulator = Duration::ZERO;
            return;
        }

        let delta = now
            .saturating_duration_since(self.last_frame)
            .min(MAX_CATCH_UP);
        self.last_frame = now;
        self.accumulator += delta;

        while self.accumulator >= SIMULATION_STEP {
            let (events, game_over, score) = {
                let Some(game) = self.game.as_mut() else {
                    break;
                };
                game.step();
                let events = game.drain_events().collect::<Vec<_>>();
                (events, game.is_game_over(), game.score())
            };
            self.accumulator -= SIMULATION_STEP;

            let new_best = game_over && score > self.session_best;
            self.audio.observe_game_events(&events, new_best);
            for event in &events {
                self.effects.observe(event, now);
            }

            if game_over {
                self.record_best_score(score);
                self.screen = Screen::GameOver;
                self.clear_gameplay_input();
                self.accumulator = Duration::ZERO;
                break;
            }
        }
    }

    fn handle_ui_action(&mut self, action: UiAction, context: &egui::Context) {
        match action {
            UiAction::None => {}
            UiAction::Play | UiAction::Restart => self.start_game(),
            UiAction::Pause => self.pause(true),
            UiAction::Resume => self.resume(),
            UiAction::MainMenu => {
                self.audio.play_ui(Cue::UiActivate);
                self.return_to_title();
            }
            UiAction::Quit => {
                self.audio.play_ui(Cue::UiActivate);
                context.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            UiAction::Fullscreen => {
                self.audio.activate();
                self.audio.play_ui(Cue::UiActivate);
                platform::toggle_fullscreen(context);
            }
            UiAction::ToggleAudioControls => {
                self.audio.activate();
                self.audio.play_ui(Cue::UiActivate);
                self.audio_controls_open = !self.audio_controls_open;
            }
            UiAction::ToggleMute => self.toggle_mute(),
            UiAction::SetAudioVolume(volume) => {
                self.audio.activate();
                self.audio.set_volume(volume);
                self.set_audio_notice(format!(
                    "SOUND {}%",
                    (self.audio.volume() * 100.0).round() as u32
                ));
            }
        }
        context.request_repaint();
    }

    fn toggle_mute(&mut self) {
        self.audio.activate();
        self.audio.toggle_muted();
        self.set_audio_notice(if self.audio.is_muted() {
            "SOUND MUTED".to_owned()
        } else {
            "SOUND ON".to_owned()
        });
    }

    fn set_audio_notice(&mut self, message: String) {
        self.audio_notice = Some((message, Instant::now()));
    }

    fn active_audio_notice(&mut self, now: Instant) -> Option<&str> {
        if self
            .audio_notice
            .as_ref()
            .is_some_and(|(_, started)| now.duration_since(*started) >= AUDIO_NOTICE_DURATION)
        {
            self.audio_notice = None;
        }
        self.audio_notice
            .as_ref()
            .map(|(message, _)| message.as_str())
    }

    #[cfg(feature = "audio-lab")]
    fn handle_audio_lab_action(&mut self, action: crate::ui::AudioLabAction) {
        use crate::ui::AudioLabAction;

        match action {
            AudioLabAction::None => {}
            AudioLabAction::Preview(cue) => {
                self.audio
                    .preview(cue, self.audio_lab_rate, self.audio_lab_pan);
            }
            AudioLabAction::PreviewCompound(compound) => {
                self.audio
                    .preview_compound(compound, self.audio_lab_rate, self.audio_lab_pan);
            }
            AudioLabAction::Stop => self.audio.stop_all(),
            AudioLabAction::ToggleMute => self.toggle_mute(),
            AudioLabAction::SetVolume(volume) => self.audio.set_volume(volume),
            AudioLabAction::SetRate(rate) => self.audio_lab_rate = rate,
            AudioLabAction::SetPan(pan) => self.audio_lab_pan = pan,
        }
    }
}

impl eframe::App for FerrofallApp {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();

        #[cfg(feature = "qa-screenshot")]
        let focused = true;
        #[cfg(not(feature = "qa-screenshot"))]
        let focused = context.input(|input| input.viewport().focused.unwrap_or(true));
        self.handle_focus(focused);

        let key_events = context.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Key {
                        key,
                        pressed,
                        repeat,
                        ..
                    } => Some((*key, *pressed, *repeat)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        for (key, pressed, repeat) in key_events {
            self.handle_key_event(key, pressed, repeat);
        }

        self.advance_game(now);
        self.effects.retain_active(now);
        let audio_notice_active = self.active_audio_notice(now).is_some();

        if self.screen == Screen::Playing || self.effects.is_active() || audio_notice_active {
            context.request_repaint_after(SIMULATION_STEP);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(feature = "audio-lab")]
        if self.audio_lab_enabled {
            let action = crate::ui::show_audio_lab(
                ui,
                AudioUiState {
                    volume: self.audio.volume(),
                    muted: self.audio.is_muted(),
                    available: self.audio.is_available(),
                    controls_open: false,
                    notice: None,
                    failure_reason: self.audio.failure_reason(),
                },
                self.audio_lab_rate,
                self.audio_lab_pan,
            );
            self.handle_audio_lab_action(action);
            return;
        }

        if let Some(issue) = platform::browser_support_issue(ui.max_rect().size()) {
            self.pause(false);
            let (screen, status) = match issue {
                platform::BrowserSupportIssue::TouchOnly => (
                    "unsupported-device",
                    "Ferrofall requires a desktop or laptop browser with a keyboard.",
                ),
                platform::BrowserSupportIssue::ViewportTooSmall => (
                    "viewport-too-small",
                    "Ferrofall needs a browser viewport of at least 720 by 560 pixels.",
                ),
            };
            self.set_accessible_status(screen, status.to_owned());
            crate::ui::show_browser_support_issue(ui, issue);
            return;
        }

        let now = Instant::now();
        let audio_notice = self.active_audio_notice(now).map(str::to_owned);
        let (screen, mut status) = match self.screen {
            Screen::Title => (
                "title",
                "Ferrofall title screen. Press Enter or choose Play to begin.".to_owned(),
            ),
            Screen::Playing => (
                "playing",
                "Ferrofall game in progress. Press Escape to pause.".to_owned(),
            ),
            Screen::Paused => (
                "paused",
                "Ferrofall paused. Press Escape to resume.".to_owned(),
            ),
            Screen::GameOver => {
                let score = self.game.as_ref().map_or(0, Game::score);
                (
                    "game-over",
                    format!("Ferrofall game over. Final score: {score}."),
                )
            }
        };
        if let Some(notice) = &audio_notice {
            status.push(' ');
            status.push_str(notice);
            status.push('.');
        }
        if !self.audio.is_available() {
            status.push_str(" Sound is unavailable; gameplay continues silently.");
        }
        self.set_accessible_status(screen, status);

        let action = crate::ui::show(
            ui,
            self.screen,
            self.game.as_ref(),
            self.session_best,
            &self.effects,
            now,
            AudioUiState {
                volume: self.audio.volume(),
                muted: self.audio.is_muted(),
                available: self.audio.is_available(),
                controls_open: self.audio_controls_open,
                notice: audio_notice.as_deref(),
                failure_reason: self.audio.failure_reason(),
            },
        );
        self.handle_ui_action(action, ui.ctx());
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(7, 16, 24).to_normalized_gamma_f32()
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(AUDIO_VOLUME_KEY, self.audio.volume().to_string());
        storage.set_string(AUDIO_MUTED_KEY, self.audio.is_muted().to_string());
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

fn key_to_action(key: Key) -> Option<Action> {
    match key {
        Key::ArrowLeft => Some(Action::Left),
        Key::ArrowRight => Some(Action::Right),
        Key::ArrowDown => Some(Action::SoftDrop),
        Key::ArrowUp | Key::X => Some(Action::RotateClockwise),
        Key::Z => Some(Action::RotateCounterclockwise),
        Key::C | Key::ShiftLeft => Some(Action::Hold),
        Key::Space => Some(Action::HardDrop),
        _ => None,
    }
}

fn configure_egui(context: &egui::Context) {
    context.set_theme(egui::Theme::Dark);
    let mut style = (*context.style_of(egui::Theme::Dark)).clone();
    style.animation_time = 0.12;
    style.spacing.button_padding = egui::vec2(22.0, 10.0);
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = egui::Color32::from_rgb(7, 16, 24);
    style.visuals.window_fill = egui::Color32::from_rgb(13, 26, 36);
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(210, 220, 226));
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(207, 142, 35);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(2);
    context.set_style_of(egui::Theme::Dark, style);
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage as _;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryStorage(HashMap<String, String>);

    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }

        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_owned(), value);
        }

        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }

        fn flush(&mut self) {}
    }

    #[test]
    fn title_play_movement_focus_pause_and_resume_form_a_complete_path() {
        let mut app = FerrofallApp::initial_state();
        assert_eq!(app.screen, Screen::Title);

        assert!(app.handle_screen_key(Key::Enter));
        assert_eq!(app.screen, Screen::Playing);
        let before = app
            .game
            .as_ref()
            .unwrap()
            .active_blocks()
            .into_iter()
            .map(|block| block.x)
            .min()
            .unwrap();

        app.handle_key_event(Key::ArrowLeft, true, false);
        app.game.as_mut().unwrap().step();
        let after = app
            .game
            .as_ref()
            .unwrap()
            .active_blocks()
            .into_iter()
            .map(|block| block.x)
            .min()
            .unwrap();
        assert_eq!(after, before - 1);

        app.handle_focus(false);
        assert_eq!(app.screen, Screen::Paused);
        assert!(app.pressed_keys.is_empty());

        assert!(app.handle_screen_key(Key::Escape));
        assert_eq!(app.screen, Screen::Playing);
    }

    #[test]
    fn all_documented_gameplay_keys_map_to_engine_actions() {
        assert_eq!(key_to_action(Key::ArrowLeft), Some(Action::Left));
        assert_eq!(key_to_action(Key::ArrowRight), Some(Action::Right));
        assert_eq!(key_to_action(Key::ArrowDown), Some(Action::SoftDrop));
        assert_eq!(key_to_action(Key::ArrowUp), Some(Action::RotateClockwise));
        assert_eq!(key_to_action(Key::X), Some(Action::RotateClockwise));
        assert_eq!(key_to_action(Key::Z), Some(Action::RotateCounterclockwise));
        assert_eq!(key_to_action(Key::C), Some(Action::Hold));
        assert_eq!(key_to_action(Key::ShiftLeft), Some(Action::Hold));
        assert_eq!(key_to_action(Key::Space), Some(Action::HardDrop));
    }

    #[test]
    fn recording_a_higher_score_updates_session_best() {
        let mut app = FerrofallApp::initial_state();
        app.record_best_score(12_345);
        assert_eq!(app.session_best, 12_345);

        app.record_best_score(1_000);
        assert_eq!(app.session_best, 12_345);
    }

    #[test]
    fn mute_key_is_global_and_never_maps_to_gameplay() {
        let mut app = FerrofallApp::initial_state();
        assert!(!app.audio.is_muted());

        assert!(app.handle_screen_key(Key::M));
        assert!(app.audio.is_muted());
        assert_eq!(key_to_action(Key::M), None);

        assert!(app.handle_screen_key(Key::M));
        assert!(!app.audio.is_muted());
    }

    #[test]
    fn audio_preferences_restore_and_save_through_eframe_storage() {
        let mut stored = MemoryStorage::default();
        stored.set_string(AUDIO_VOLUME_KEY, "0.42".to_owned());
        stored.set_string(AUDIO_MUTED_KEY, "true".to_owned());
        let mut creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        creation.storage = Some(&stored);
        let mut app = FerrofallApp::new(&creation);

        assert!((app.audio.volume() - 0.42).abs() < f32::EPSILON);
        assert!(app.audio.is_muted());

        app.audio.set_volume(0.81);
        app.audio.set_muted(false);
        let mut saved = MemoryStorage::default();
        eframe::App::save(&mut app, &mut saved);
        assert_eq!(saved.get_string(AUDIO_VOLUME_KEY).as_deref(), Some("0.81"));
        assert_eq!(saved.get_string(AUDIO_MUTED_KEY).as_deref(), Some("false"));
    }
}
