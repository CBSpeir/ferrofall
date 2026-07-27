use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use eframe::egui::{self, Event, Key, Pos2, Rect, TouchId, TouchPhase};
use web_time::Instant;

use crate::audio::{AudioSystem, Cue, DEFAULT_VOLUME};
use crate::game::{Action, Command, Game, GameConfig};
use crate::platform;
use crate::ui::{
    AudioUiState, Screen, TouchControlAction, UiAction, UiOutput, UiState, VisualEffects,
};

const SIMULATION_STEP: Duration = Duration::from_nanos(1_000_000_000 / 60);
const MAX_CATCH_UP: Duration = Duration::from_millis(250);
const AUDIO_NOTICE_DURATION: Duration = Duration::from_millis(1_200);
const AUDIO_VOLUME_KEY: &str = "ferrofall.audio-volume.v1";
const AUDIO_MUTED_KEY: &str = "ferrofall.audio-muted.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewportOrientation {
    Portrait,
    Landscape,
}

impl ViewportOrientation {
    fn from_rect(rect: Rect) -> Self {
        if rect.height() >= rect.width() {
            Self::Portrait
        } else {
            Self::Landscape
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TouchBinding {
    Held(Option<TouchControlAction>),
    Immediate(TouchControlAction),
    ReleaseAction {
        action: TouchControlAction,
        armed: bool,
    },
    Ignored,
}

impl TouchBinding {
    fn active_action(self) -> Option<TouchControlAction> {
        match self {
            Self::Held(action) => action,
            Self::Immediate(action) => Some(action),
            Self::ReleaseAction {
                action,
                armed: true,
            } => Some(action),
            Self::ReleaseAction { armed: false, .. } | Self::Ignored => None,
        }
    }
}

pub(crate) struct FerrofallApp {
    screen: Screen,
    game: Option<Game>,
    session_best: u64,
    last_frame: Instant,
    accumulator: Duration,
    pressed_keys: BTreeSet<Key>,
    touch_contacts: BTreeMap<TouchId, TouchBinding>,
    touch_mode: bool,
    viewport_orientation: Option<ViewportOrientation>,
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
            touch_contacts: BTreeMap::new(),
            touch_mode: platform::prefers_touch_controls(),
            viewport_orientation: None,
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
        self.touch_contacts.clear();
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
            self.touch_contacts.clear();
            self.audio.play_ui(Cue::Resume);
        }
    }

    fn return_to_title(&mut self) {
        self.update_session_best();
        self.screen = Screen::Title;
        self.game = None;
        self.accumulator = Duration::ZERO;
        self.pressed_keys.clear();
        self.touch_contacts.clear();
        self.effects.clear();
        self.audio.stop_all();
    }

    fn clear_gameplay_input(&mut self) {
        self.pressed_keys.clear();
        self.touch_contacts.clear();
        if let Some(game) = self.game.as_mut() {
            game.clear_input();
        }
    }

    fn clear_touch_input(&mut self) {
        let mut held_actions = Vec::new();
        for binding in self.touch_contacts.values() {
            if let TouchBinding::Held(Some(control)) = binding {
                let action = touch_to_game_action(*control);
                if !held_actions.contains(&action) {
                    held_actions.push(action);
                }
            }
        }
        self.touch_contacts.clear();
        held_actions.retain(|action| !self.any_pressed_key_maps_to(*action));
        if let Some(game) = self.game.as_mut() {
            for action in held_actions {
                game.apply(Command::Release(action));
            }
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
                && !self.touch_holds_action(action)
                && let Some(game) = self.game.as_mut()
            {
                game.apply(Command::Press(action));
            }
        } else if !self.any_pressed_key_maps_to(action)
            && !self.touch_holds_action(action)
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

    fn touch_controls_enabled(&self) -> bool {
        self.touch_mode || platform::prefers_touch_controls()
    }

    fn touch_holds_action(&self, action: Action) -> bool {
        self.touch_contacts.values().any(|binding| {
            matches!(binding, TouchBinding::Held(Some(control)) if touch_to_game_action(*control) == action)
        })
    }

    fn other_touch_holds_action(&self, id: TouchId, action: Action) -> bool {
        self.touch_contacts.iter().any(|(touch_id, binding)| {
            *touch_id != id
                && matches!(binding, TouchBinding::Held(Some(control)) if touch_to_game_action(*control) == action)
        })
    }

    fn set_touch_held(&mut self, id: TouchId, next: Option<TouchControlAction>) {
        let current = self
            .touch_contacts
            .get(&id)
            .and_then(|binding| match binding {
                TouchBinding::Held(action) => *action,
                TouchBinding::Immediate(_)
                | TouchBinding::ReleaseAction { .. }
                | TouchBinding::Ignored => None,
            });
        if current == next {
            return;
        }

        if let Some(current) = current {
            let action = touch_to_game_action(current);
            if !self.other_touch_holds_action(id, action)
                && !self.any_pressed_key_maps_to(action)
                && let Some(game) = self.game.as_mut()
            {
                game.apply(Command::Release(action));
            }
        }
        if let Some(next) = next {
            let action = touch_to_game_action(next);
            if !self.other_touch_holds_action(id, action)
                && !self.any_pressed_key_maps_to(action)
                && let Some(game) = self.game.as_mut()
            {
                game.apply(Command::Press(action));
            }
        }
        self.touch_contacts.insert(id, TouchBinding::Held(next));
    }

    fn finish_touch(&mut self, id: TouchId, phase: TouchPhase, pos: Pos2, viewport: Rect) {
        let Some(binding) = self.touch_contacts.remove(&id) else {
            return;
        };
        match binding {
            TouchBinding::Held(Some(control)) => {
                let action = touch_to_game_action(control);
                if !self.touch_holds_action(action)
                    && !self.any_pressed_key_maps_to(action)
                    && let Some(game) = self.game.as_mut()
                {
                    game.apply(Command::Release(action));
                }
            }
            TouchBinding::ReleaseAction { action, armed }
                if phase == TouchPhase::End
                    && armed
                    && crate::ui::touch_control_layout(
                        viewport,
                        crate::ui::layout_mode(viewport, true),
                    )
                    .is_some_and(|layout| layout.contains(action, pos)) =>
            {
                if let Some(game) = self.game.as_mut() {
                    game.apply(Command::Press(touch_to_game_action(action)));
                }
            }
            TouchBinding::Held(None)
            | TouchBinding::Immediate(_)
            | TouchBinding::ReleaseAction { .. }
            | TouchBinding::Ignored => {}
        }
    }

    fn handle_touch_event(&mut self, id: TouchId, phase: TouchPhase, pos: Pos2, viewport: Rect) {
        self.touch_mode = true;

        if self.screen != Screen::Playing
            || platform::browser_support_issue(viewport.size()).is_some()
        {
            if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
                self.finish_touch(id, phase, pos, viewport);
            }
            return;
        }

        let Some(layout) =
            crate::ui::touch_control_layout(viewport, crate::ui::layout_mode(viewport, true))
        else {
            self.clear_gameplay_input();
            return;
        };

        match phase {
            TouchPhase::Start => {
                if self.touch_contacts.contains_key(&id) {
                    self.finish_touch(id, TouchPhase::Cancel, pos, viewport);
                }
                match layout.action_at(pos) {
                    Some(action) if action.is_held() => {
                        self.touch_contacts.insert(id, TouchBinding::Held(None));
                        self.set_touch_held(id, Some(action));
                    }
                    Some(action @ TouchControlAction::HardDrop) => {
                        self.touch_contacts.insert(
                            id,
                            TouchBinding::ReleaseAction {
                                action,
                                armed: true,
                            },
                        );
                    }
                    Some(action) => {
                        if let Some(game) = self.game.as_mut() {
                            game.apply(Command::Press(touch_to_game_action(action)));
                        }
                        self.touch_contacts
                            .insert(id, TouchBinding::Immediate(action));
                    }
                    None => {
                        self.touch_contacts.insert(id, TouchBinding::Ignored);
                    }
                }
            }
            TouchPhase::Move => match self.touch_contacts.get(&id).copied() {
                Some(TouchBinding::Held(_)) => {
                    self.set_touch_held(id, layout.held_action_at(pos));
                }
                Some(TouchBinding::ReleaseAction { action, armed }) => {
                    self.touch_contacts.insert(
                        id,
                        TouchBinding::ReleaseAction {
                            action,
                            armed: armed && layout.contains(action, pos),
                        },
                    );
                }
                Some(TouchBinding::Immediate(_) | TouchBinding::Ignored) | None => {}
            },
            TouchPhase::End | TouchPhase::Cancel => {
                self.finish_touch(id, phase, pos, viewport);
            }
        }
    }

    fn active_touch_controls(&self) -> Vec<TouchControlAction> {
        self.touch_contacts
            .values()
            .filter_map(|binding| binding.active_action())
            .collect()
    }

    fn handle_viewport_orientation(&mut self, viewport: Rect) {
        let orientation = ViewportOrientation::from_rect(viewport);
        if self.touch_controls_enabled()
            && self
                .viewport_orientation
                .is_some_and(|previous| previous != orientation)
        {
            self.pause(false);
        }
        self.viewport_orientation = Some(orientation);
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
        let resolution_changed = platform::sync_canvas_resolution();
        if resolution_changed {
            self.clear_touch_input();
            context.request_repaint();
        }
        let viewport = context.viewport_rect();
        self.handle_viewport_orientation(viewport);

        #[cfg(feature = "qa-screenshot")]
        let focused = true;
        #[cfg(not(feature = "qa-screenshot"))]
        let focused = context.input(|input| {
            input.viewport().focused.unwrap_or(true) && !input.viewport().occluded.unwrap_or(false)
        });
        self.handle_focus(focused);

        let input_events = context.input(|input| {
            input
                .events
                .iter()
                .filter(|event| matches!(event, Event::Key { .. } | Event::Touch { .. }))
                .cloned()
                .collect::<Vec<_>>()
        });
        for event in input_events {
            match event {
                Event::Key {
                    key,
                    pressed,
                    repeat,
                    ..
                } => self.handle_key_event(key, pressed, repeat),
                Event::Touch { id, phase, pos, .. } if !resolution_changed => {
                    self.handle_touch_event(id, phase, pos, viewport)
                }
                _ => {}
            }
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
                platform::BrowserSupportIssue::ViewportTooSmall => (
                    "viewport-too-small",
                    "Ferrofall needs a safe viewport of at least 320 by 500 pixels, or 500 by 320 in landscape.",
                ),
            };
            platform::set_canvas_layout("unsupported", false);
            platform::set_canvas_touch_metadata("", "");
            self.set_accessible_status(screen, status.to_owned());
            crate::ui::show_browser_support_issue(ui, issue);
            return;
        }

        let touch_controls = self.touch_controls_enabled();
        let layout_mode = crate::ui::layout_mode(ui.max_rect(), touch_controls);
        let control_layout = (touch_controls && self.screen == Screen::Playing)
            .then(|| crate::ui::touch_control_layout(ui.max_rect(), layout_mode))
            .flatten();
        let touch_controls_visible = control_layout.is_some();
        let active_touch_controls = self.active_touch_controls();
        let active_touch_metadata = active_touch_controls
            .iter()
            .map(|control| control.data_label())
            .collect::<Vec<_>>()
            .join(",");
        let touch_region_metadata = control_layout
            .as_ref()
            .map(crate::ui::TouchControlLayout::metadata)
            .unwrap_or_default();
        platform::set_canvas_layout(layout_mode.label(), touch_controls_visible);
        platform::set_canvas_touch_metadata(&touch_region_metadata, &active_touch_metadata);
        let now = Instant::now();
        let audio_notice = self.active_audio_notice(now).map(str::to_owned);
        let (screen, mut status) = match self.screen {
            Screen::Title => (
                "title",
                if touch_controls {
                    "Ferrofall title screen. Tap Play to begin with two-thumb controls.".to_owned()
                } else {
                    "Ferrofall title screen. Press Enter or choose Play to begin.".to_owned()
                },
            ),
            Screen::Playing => (
                "playing",
                if touch_controls {
                    "Ferrofall game in progress. Use the labeled touch controls or tap Pause."
                        .to_owned()
                } else {
                    "Ferrofall game in progress. Press Escape to pause.".to_owned()
                },
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

        let UiOutput { action } = crate::ui::show(
            ui,
            self.screen,
            UiState {
                game: self.game.as_ref(),
                session_best: self.session_best,
                effects: &self.effects,
                now,
                audio: AudioUiState {
                    volume: self.audio.volume(),
                    muted: self.audio.is_muted(),
                    available: self.audio.is_available(),
                    controls_open: self.audio_controls_open,
                    notice: audio_notice.as_deref(),
                    failure_reason: self.audio.failure_reason(),
                },
                touch_controls,
                active_touch_controls: &active_touch_controls,
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

fn touch_to_game_action(action: TouchControlAction) -> Action {
    match action {
        TouchControlAction::Left => Action::Left,
        TouchControlAction::SoftDrop => Action::SoftDrop,
        TouchControlAction::Right => Action::Right,
        TouchControlAction::Hold => Action::Hold,
        TouchControlAction::RotateCounterclockwise => Action::RotateCounterclockwise,
        TouchControlAction::RotateClockwise => Action::RotateClockwise,
        TouchControlAction::HardDrop => Action::HardDrop,
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

    #[test]
    fn touch_movement_can_slide_between_directions() {
        let mut app = FerrofallApp::initial_state();
        app.start_game();
        let viewport = Rect::from_min_size(Pos2::ZERO, egui::vec2(360.0, 640.0));
        let controls =
            crate::ui::touch_control_layout(viewport, crate::ui::LayoutMode::CompactPortrait)
                .unwrap();
        let before = app
            .game
            .as_ref()
            .unwrap()
            .active_blocks()
            .into_iter()
            .map(|block| block.x)
            .min()
            .unwrap();

        let touch = TouchId(7);
        app.handle_touch_event(
            touch,
            TouchPhase::Start,
            controls.rect_for(TouchControlAction::Left).center(),
            viewport,
        );
        app.game.as_mut().unwrap().step();
        let after_left = app
            .game
            .as_ref()
            .unwrap()
            .active_blocks()
            .into_iter()
            .map(|block| block.x)
            .min()
            .unwrap();
        assert_eq!(after_left, before - 1);

        app.handle_touch_event(
            touch,
            TouchPhase::Move,
            controls.rect_for(TouchControlAction::Right).center(),
            viewport,
        );
        app.game.as_mut().unwrap().step();
        let after_right = app
            .game
            .as_ref()
            .unwrap()
            .active_blocks()
            .into_iter()
            .map(|block| block.x)
            .min()
            .unwrap();
        assert_eq!(after_right, before);

        app.handle_touch_event(
            touch,
            TouchPhase::End,
            controls.rect_for(TouchControlAction::Right).center(),
            viewport,
        );
        assert!(app.touch_contacts.is_empty());
    }

    #[test]
    fn hard_drop_requires_release_inside_its_control() {
        let mut app = FerrofallApp::initial_state();
        app.start_game();
        let viewport = Rect::from_min_size(Pos2::ZERO, egui::vec2(360.0, 640.0));
        let controls =
            crate::ui::touch_control_layout(viewport, crate::ui::LayoutMode::CompactPortrait)
                .unwrap();
        let hard_drop = controls.rect_for(TouchControlAction::HardDrop).center();
        let outside = viewport.center();

        app.handle_touch_event(TouchId(1), TouchPhase::Start, hard_drop, viewport);
        app.handle_touch_event(TouchId(1), TouchPhase::Move, outside, viewport);
        app.handle_touch_event(TouchId(1), TouchPhase::Move, hard_drop, viewport);
        app.handle_touch_event(TouchId(1), TouchPhase::End, hard_drop, viewport);
        app.game.as_mut().unwrap().step();
        assert_eq!(locked_cell_count(app.game.as_ref().unwrap()), 0);

        app.handle_touch_event(TouchId(2), TouchPhase::Start, hard_drop, viewport);
        app.handle_touch_event(TouchId(2), TouchPhase::End, hard_drop, viewport);
        app.game.as_mut().unwrap().step();
        assert_eq!(locked_cell_count(app.game.as_ref().unwrap()), 4);
    }

    #[test]
    fn rotation_fires_on_touch_start_without_repeating() {
        let mut app = FerrofallApp::initial_state();
        app.start_game();
        let viewport = Rect::from_min_size(Pos2::ZERO, egui::vec2(360.0, 640.0));
        let controls =
            crate::ui::touch_control_layout(viewport, crate::ui::LayoutMode::CompactPortrait)
                .unwrap();
        let clockwise = controls
            .rect_for(TouchControlAction::RotateClockwise)
            .center();
        let counterclockwise = controls
            .rect_for(TouchControlAction::RotateCounterclockwise)
            .center();
        let outside = viewport.center();
        app.game.as_mut().unwrap().drain_events().for_each(drop);

        app.handle_touch_event(TouchId(1), TouchPhase::Start, clockwise, viewport);
        app.game.as_mut().unwrap().step();
        assert!(
            app.game
                .as_mut()
                .unwrap()
                .drain_events()
                .any(|event| { matches!(event, crate::game::GameEvent::Rotated { .. }) })
        );
        app.handle_touch_event(TouchId(1), TouchPhase::Move, outside, viewport);
        app.handle_touch_event(TouchId(1), TouchPhase::Move, clockwise, viewport);
        app.handle_touch_event(TouchId(1), TouchPhase::End, clockwise, viewport);
        assert!(
            !app.game
                .as_mut()
                .unwrap()
                .drain_events()
                .any(|event| { matches!(event, crate::game::GameEvent::Rotated { .. }) })
        );
        app.handle_touch_event(TouchId(2), TouchPhase::Start, counterclockwise, viewport);
        app.game.as_mut().unwrap().step();
        assert!(app.game.as_mut().unwrap().drain_events().any(|event| {
            matches!(
                event,
                crate::game::GameEvent::Rotated {
                    direction: crate::game::RotationDirection::Counterclockwise,
                    ..
                }
            )
        }));
        app.handle_touch_event(TouchId(2), TouchPhase::End, counterclockwise, viewport);

        app.handle_touch_event(TouchId(3), TouchPhase::Start, outside, viewport);
        app.handle_touch_event(TouchId(3), TouchPhase::Move, clockwise, viewport);
        app.handle_touch_event(TouchId(3), TouchPhase::End, clockwise, viewport);
        app.game.as_mut().unwrap().step();
        assert!(
            !app.game
                .as_mut()
                .unwrap()
                .drain_events()
                .any(|event| { matches!(event, crate::game::GameEvent::Rotated { .. }) })
        );
    }

    #[test]
    fn touch_orientation_change_pauses_and_clears_input() {
        let mut app = FerrofallApp::initial_state();
        app.touch_mode = true;
        app.start_game();
        app.handle_viewport_orientation(Rect::from_min_size(Pos2::ZERO, egui::vec2(360.0, 640.0)));
        assert_eq!(app.screen, Screen::Playing);

        app.handle_viewport_orientation(Rect::from_min_size(Pos2::ZERO, egui::vec2(640.0, 360.0)));
        assert_eq!(app.screen, Screen::Paused);
        assert!(app.touch_contacts.is_empty());
    }

    fn locked_cell_count(game: &Game) -> usize {
        (0..crate::game::BOARD_HEIGHT)
            .flat_map(|y| (0..crate::game::BOARD_WIDTH).map(move |x| (x, y)))
            .filter(|(x, y)| game.board().cell(*x, *y).is_some())
            .count()
    }
}
