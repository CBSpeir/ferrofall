use std::time::Duration;

use web_time::Instant;

use crate::game::{GameEvent, MovementDirection, RotationDirection, Spin, VISIBLE_HEIGHT};

pub(crate) const DEFAULT_EFFECTS_VOLUME: f32 = 0.70;
pub(crate) const DEFAULT_MUSIC_VOLUME: f32 = 0.35;
const DANGER_START_ROW: usize = crate::game::VISIBLE_TOP + 7;
const DANGER_CLEAR_ROW: usize = crate::game::VISIBLE_TOP + 10;
const MUSIC_DUCK_DURATION: Duration = Duration::from_millis(300);
#[cfg(not(target_arch = "wasm32"))]
const MUSIC_BAR_SECONDS: f64 = 240.0 / 132.0;
#[cfg(not(target_arch = "wasm32"))]
const MAX_VOICES: usize = 16;

#[cfg(target_arch = "wasm32")]
pub(crate) fn prepare_web_audio() {
    output::prepare();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MusicTier {
    Base,
    Drive,
    Pressure,
}

impl MusicTier {
    const fn layer_count(self) -> usize {
        match self {
            Self::Base => 1,
            Self::Drive => 2,
            Self::Pressure => 3,
        }
    }

    const fn advance(self) -> Self {
        match self {
            Self::Base => Self::Drive,
            Self::Drive | Self::Pressure => Self::Pressure,
        }
    }

    #[cfg(target_arch = "wasm32")]
    const fn web_value(self) -> u8 {
        self.layer_count() as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Cue {
    UiActivate,
    GameStart,
    Pause,
    Resume,
    MoveA,
    MoveB,
    Rotate,
    Hold,
    Contact,
    HardDrop,
    Lock,
    ClearSingle,
    ClearDouble,
    ClearTriple,
    ClearFour,
    AccentTSpin,
    AccentCombo,
    AccentBackToBack,
    AccentPerfect,
    LevelUp,
    GameOver,
    NewBest,
}

impl Cue {
    #[cfg(feature = "audio-lab")]
    pub(crate) const ALL: [Self; 22] = [
        Self::UiActivate,
        Self::GameStart,
        Self::Pause,
        Self::Resume,
        Self::MoveA,
        Self::MoveB,
        Self::Rotate,
        Self::Hold,
        Self::Contact,
        Self::HardDrop,
        Self::Lock,
        Self::ClearSingle,
        Self::ClearDouble,
        Self::ClearTriple,
        Self::ClearFour,
        Self::AccentTSpin,
        Self::AccentCombo,
        Self::AccentBackToBack,
        Self::AccentPerfect,
        Self::LevelUp,
        Self::GameOver,
        Self::NewBest,
    ];

    pub(crate) const fn file_stem(self) -> &'static str {
        match self {
            Self::UiActivate => "ui_activate",
            Self::GameStart => "game_start",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::MoveA => "move_a",
            Self::MoveB => "move_b",
            Self::Rotate => "rotate",
            Self::Hold => "hold",
            Self::Contact => "contact",
            Self::HardDrop => "hard_drop",
            Self::Lock => "lock",
            Self::ClearSingle => "clear_single",
            Self::ClearDouble => "clear_double",
            Self::ClearTriple => "clear_triple",
            Self::ClearFour => "clear_four",
            Self::AccentTSpin => "accent_tspin",
            Self::AccentCombo => "accent_combo",
            Self::AccentBackToBack => "accent_back_to_back",
            Self::AccentPerfect => "accent_perfect",
            Self::LevelUp => "level_up",
            Self::GameOver => "game_over",
            Self::NewBest => "new_best",
        }
    }

    #[cfg(feature = "audio-lab")]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::UiActivate => "UI ACTIVATE",
            Self::GameStart => "GAME START",
            Self::Pause => "PAUSE",
            Self::Resume => "RESUME",
            Self::MoveA => "MOVE A",
            Self::MoveB => "MOVE B",
            Self::Rotate => "ROTATE",
            Self::Hold => "HOLD",
            Self::Contact => "CONTACT",
            Self::HardDrop => "HARD DROP",
            Self::Lock => "LOCK",
            Self::ClearSingle => "SINGLE",
            Self::ClearDouble => "DOUBLE",
            Self::ClearTriple => "TRIPLE",
            Self::ClearFour => "FOUR LINES",
            Self::AccentTSpin => "T-SPIN ACCENT",
            Self::AccentCombo => "COMBO ACCENT",
            Self::AccentBackToBack => "BACK-TO-BACK",
            Self::AccentPerfect => "PERFECT CLEAR",
            Self::LevelUp => "LEVEL UP",
            Self::GameOver => "GAME OVER",
            Self::NewBest => "NEW BEST",
        }
    }
}

#[cfg(feature = "audio-lab")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CompoundPreview {
    TSpinCombo,
    PerfectFour,
    NewBest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CueRequest {
    cue: Cue,
    gain_db: f32,
    rate: f32,
    pan: f32,
    delay: Duration,
}

impl CueRequest {
    const fn new(cue: Cue, gain_db: f32) -> Self {
        Self {
            cue,
            gain_db,
            rate: 1.0,
            pan: 0.0,
            delay: Duration::ZERO,
        }
    }

    const fn rate(mut self, rate: f32) -> Self {
        self.rate = rate;
        self
    }

    fn pan(mut self, pan: f32) -> Self {
        self.pan = pan.clamp(-0.35, 0.35);
        self
    }

    const fn delay_ms(mut self, milliseconds: u64) -> Self {
        self.delay = Duration::from_millis(milliseconds);
        self
    }
}

pub(crate) struct AudioSystem {
    output: output::Output,
    effects_volume: f32,
    music_volume: f32,
    muted: bool,
    alternate_move: bool,
    music_tier: MusicTier,
    danger_active: bool,
    music_ducked_until: Option<Instant>,
}

impl AudioSystem {
    pub(crate) fn new(effects_volume: f32, music_volume: f32, muted: bool) -> Self {
        let effects_volume = effects_volume.clamp(0.0, 1.0);
        let music_volume = music_volume.clamp(0.0, 1.0);
        let mut output = output::Output::new();
        output.set_effects_volume(perceptual_gain(effects_volume));
        output.set_music_volume(perceptual_gain(music_volume));
        output.set_muted(muted);
        Self {
            output,
            effects_volume,
            music_volume,
            muted,
            alternate_move: false,
            music_tier: MusicTier::Base,
            danger_active: false,
            music_ducked_until: None,
        }
    }

    pub(crate) fn is_available(&self) -> bool {
        self.output.is_available()
    }

    pub(crate) fn failure_reason(&self) -> Option<&str> {
        self.output.failure_reason()
    }

    pub(crate) fn is_music_available(&self) -> bool {
        self.output.music_available()
    }

    pub(crate) fn music_failure_reason(&self) -> Option<&str> {
        self.output.music_failure_reason()
    }

    pub(crate) const fn effects_volume(&self) -> f32 {
        self.effects_volume
    }

    pub(crate) const fn music_volume(&self) -> f32 {
        self.music_volume
    }

    pub(crate) const fn is_muted(&self) -> bool {
        self.muted
    }

    pub(crate) fn activate(&mut self) {
        self.output.activate();
        self.apply_output_levels();
    }

    pub(crate) fn set_effects_volume(&mut self, volume: f32) {
        self.effects_volume = volume.clamp(0.0, 1.0);
        self.output
            .set_effects_volume(perceptual_gain(self.effects_volume));
    }

    pub(crate) fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        self.output
            .set_music_volume(perceptual_gain(self.music_volume));
    }

    pub(crate) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.output.set_muted(muted);
    }

    pub(crate) fn toggle_muted(&mut self) {
        self.set_muted(!self.muted);
        if !self.muted {
            self.play_request(CueRequest::new(Cue::Resume, -7.0));
        }
    }

    pub(crate) fn stop_all(&mut self) {
        self.output.stop_all();
        self.music_ducked_until = None;
    }

    pub(crate) fn stop_effects(&mut self) {
        self.output.stop_effects();
    }

    pub(crate) fn start_music(&mut self) {
        self.music_tier = MusicTier::Base;
        self.danger_active = false;
        self.music_ducked_until = None;
        self.output.start_music(self.music_tier);
    }

    pub(crate) fn pause_music(&mut self) {
        self.output.pause_music();
    }

    pub(crate) fn resume_music(&mut self) {
        self.output.resume_music();
    }

    pub(crate) fn stop_music(&mut self) {
        self.output.stop_music();
        self.music_ducked_until = None;
    }

    pub(crate) fn update_music(
        &mut self,
        level: u32,
        highest_locked_row: Option<usize>,
        now: Instant,
    ) {
        self.danger_active = if self.danger_active {
            highest_locked_row.is_some_and(|row| row < DANGER_CLEAR_ROW)
        } else {
            highest_locked_row.is_some_and(|row| row <= DANGER_START_ROW)
        };
        let baseline = baseline_music_tier(level);
        let target = if self.danger_active {
            baseline.advance()
        } else {
            baseline
        };
        if target != self.music_tier {
            self.music_tier = target;
            self.output.set_music_tier(target);
        }
        self.tick(now);
    }

    pub(crate) fn tick(&mut self, now: Instant) {
        if self
            .music_ducked_until
            .is_some_and(|deadline| now >= deadline)
        {
            self.music_ducked_until = None;
            self.output.set_music_ducked(false);
        }
    }

    pub(crate) fn play_ui(&mut self, cue: Cue) {
        let gain = match cue {
            Cue::UiActivate => -9.0,
            Cue::Pause | Cue::Resume => -5.0,
            Cue::GameStart => -2.5,
            _ => -4.0,
        };
        self.play_request(CueRequest::new(cue, gain));
    }

    pub(crate) fn observe_game_events(&mut self, events: &[GameEvent], new_best: bool) {
        if events
            .iter()
            .any(|event| matches!(event, GameEvent::GameOver))
        {
            self.stop_music();
            self.stop_effects();
        } else if events.iter().any(|event| {
            matches!(
                event,
                GameEvent::Cleared { .. } | GameEvent::LevelChanged(_)
            )
        }) {
            self.output.set_music_ducked(true);
            self.music_ducked_until = Some(Instant::now() + MUSIC_DUCK_DURATION);
        }
        for request in plan_game_audio(events, new_best, &mut self.alternate_move) {
            self.play_request(request);
        }
    }

    #[cfg(feature = "audio-lab")]
    pub(crate) fn preview(&mut self, cue: Cue, rate: f32, pan: f32) {
        self.activate();
        self.play_request(CueRequest::new(cue, -3.0).rate(rate).pan(pan));
    }

    #[cfg(feature = "audio-lab")]
    pub(crate) fn preview_compound(&mut self, compound: CompoundPreview, rate: f32, pan: f32) {
        self.activate();
        self.stop_all();
        let requests = match compound {
            CompoundPreview::TSpinCombo => vec![
                CueRequest::new(Cue::HardDrop, -5.0),
                CueRequest::new(Cue::ClearDouble, -2.5),
                CueRequest::new(Cue::AccentTSpin, -4.0).delay_ms(20),
                CueRequest::new(Cue::AccentCombo, -8.0)
                    .rate(1.10)
                    .delay_ms(45),
                CueRequest::new(Cue::AccentBackToBack, -6.0).delay_ms(65),
            ],
            CompoundPreview::PerfectFour => vec![
                CueRequest::new(Cue::ClearFour, -2.5),
                CueRequest::new(Cue::AccentPerfect, -2.0).delay_ms(80),
                CueRequest::new(Cue::LevelUp, -5.0).rate(1.12).delay_ms(120),
            ],
            CompoundPreview::NewBest => vec![
                CueRequest::new(Cue::GameOver, -3.0),
                CueRequest::new(Cue::NewBest, -1.5).delay_ms(240),
            ],
        };
        for mut request in requests {
            request.rate *= rate;
            request.pan = (request.pan + pan).clamp(-0.35, 0.35);
            self.play_request(request);
        }
    }

    #[cfg(feature = "audio-lab")]
    pub(crate) fn preview_music(&mut self, tier: MusicTier) {
        self.activate();
        self.stop_music();
        self.music_tier = tier;
        self.output.start_music(tier);
    }

    #[cfg(feature = "audio-lab")]
    pub(crate) fn preview_music_duck(&mut self) {
        self.output.set_music_ducked(true);
        self.music_ducked_until = Some(Instant::now() + MUSIC_DUCK_DURATION);
    }

    fn play_request(&mut self, request: CueRequest) {
        if self.muted || self.effects_volume <= 0.0 {
            return;
        }
        self.output.play(request);
    }

    fn apply_output_levels(&mut self) {
        self.output
            .set_effects_volume(perceptual_gain(self.effects_volume));
        self.output
            .set_music_volume(perceptual_gain(self.music_volume));
        self.output.set_muted(self.muted);
    }
}

fn perceptual_gain(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0).powi(2)
}

fn baseline_music_tier(level: u32) -> MusicTier {
    match level {
        0..=4 => MusicTier::Base,
        5..=9 => MusicTier::Drive,
        _ => MusicTier::Pressure,
    }
}

fn plan_game_audio(
    events: &[GameEvent],
    new_best: bool,
    alternate_move: &mut bool,
) -> Vec<CueRequest> {
    if events
        .iter()
        .any(|event| matches!(event, GameEvent::GameOver))
    {
        let mut requests = vec![CueRequest::new(Cue::GameOver, -3.0)];
        if new_best {
            requests.push(CueRequest::new(Cue::NewBest, -1.5).delay_ms(240));
        }
        return requests;
    }

    let has_clear = events
        .iter()
        .any(|event| matches!(event, GameEvent::Cleared { .. }));
    let mut requests = Vec::new();

    for event in events {
        match event {
            GameEvent::Moved { direction, column } => {
                *alternate_move = !*alternate_move;
                let cue = if *alternate_move {
                    Cue::MoveA
                } else {
                    Cue::MoveB
                };
                let rate = match direction {
                    MovementDirection::Left => 0.96,
                    MovementDirection::Right => 1.04,
                };
                requests.push(
                    CueRequest::new(cue, -13.0)
                        .rate(rate)
                        .pan(column_pan(*column)),
                );
            }
            GameEvent::Rotated { direction, column } => {
                let rate = match direction {
                    RotationDirection::Clockwise => 1.04,
                    RotationDirection::Counterclockwise => 0.96,
                };
                requests.push(
                    CueRequest::new(Cue::Rotate, -10.0)
                        .rate(rate)
                        .pan(column_pan(*column)),
                );
            }
            GameEvent::Held => requests.push(CueRequest::new(Cue::Hold, -6.0)),
            GameEvent::Grounded { column } => {
                requests.push(CueRequest::new(Cue::Contact, -15.0).pan(column_pan(*column)))
            }
            GameEvent::HardDropped { from, to } => {
                let distance = (to[0].y - from[0].y).max(0) as f32;
                let gain = -7.0 + distance.min(VISIBLE_HEIGHT as f32) * 0.18;
                let column = to.iter().map(|point| point.x).sum::<i32>() / 4;
                requests.push(
                    CueRequest::new(Cue::HardDrop, gain.min(-3.0))
                        .rate(0.94 + distance.min(20.0) * 0.003)
                        .pan(column_pan(column)),
                );
            }
            GameEvent::PieceLocked { column } if !has_clear => {
                requests.push(CueRequest::new(Cue::Lock, -5.0).pan(column_pan(*column)))
            }
            GameEvent::Cleared { result, .. } => {
                let clear_cue = match result.lines {
                    0 | 1 => Cue::ClearSingle,
                    2 => Cue::ClearDouble,
                    3 => Cue::ClearTriple,
                    _ => Cue::ClearFour,
                };
                requests.push(CueRequest::new(clear_cue, -2.5));
                if result.spin != Spin::None {
                    let rate = if result.spin == Spin::Mini { 0.92 } else { 1.0 };
                    requests.push(
                        CueRequest::new(Cue::AccentTSpin, -4.0)
                            .rate(rate)
                            .delay_ms(20),
                    );
                }
                if result.combo.is_some_and(|combo| combo > 0) {
                    let combo = result.combo.unwrap_or(0).min(8) as f32;
                    requests.push(
                        CueRequest::new(Cue::AccentCombo, -8.0)
                            .rate(1.0 + combo * 0.035)
                            .delay_ms(45),
                    );
                }
                if result.back_to_back_bonus {
                    requests.push(CueRequest::new(Cue::AccentBackToBack, -6.0).delay_ms(65));
                }
                if result.perfect_clear {
                    requests.push(CueRequest::new(Cue::AccentPerfect, -2.0).delay_ms(80));
                }
            }
            GameEvent::LevelChanged(level) => {
                let rate = 1.0 + level.saturating_sub(1).min(10) as f32 * 0.018;
                requests.push(CueRequest::new(Cue::LevelUp, -3.0).rate(rate));
                if level % 5 == 0 {
                    requests.push(
                        CueRequest::new(Cue::LevelUp, -10.0)
                            .rate(rate * 1.5)
                            .delay_ms(30),
                    );
                }
            }
            GameEvent::PieceLocked { .. } | GameEvent::GameOver => {}
        }
    }

    requests
}

fn column_pan(column: i32) -> f32 {
    let visible_column = column.clamp(0, 9) as f32;
    ((visible_column - 4.5) / 4.5) * 0.35
}

#[cfg(not(target_arch = "wasm32"))]
mod output {
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::time::Duration;

    use kira::sound::FromFileError;
    use kira::sound::PlaybackState;
    use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
    use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
    use kira::{AudioManager, Decibels, DefaultBackend, StartTime, Tween};

    use super::{Cue, CueRequest, MAX_VOICES, MUSIC_BAR_SECONDS, MusicTier};

    pub(super) struct Output {
        bank: Option<HashMap<Cue, StaticSoundData>>,
        music_assets: [&'static [u8]; 3],
        manager: Option<AudioManager<DefaultBackend>>,
        handles: Vec<StaticSoundHandle>,
        music_handles: Vec<StreamingSoundHandle<FromFileError>>,
        effects_volume: f32,
        music_volume: f32,
        music_tier: MusicTier,
        music_ducked: bool,
        muted: bool,
        failure_reason: Option<String>,
        music_failure_reason: Option<String>,
    }

    impl Output {
        pub(super) fn new() -> Self {
            let (bank, failure_reason) = match load_bank() {
                Ok(bank) => (Some(bank), None),
                Err(reason) => (None, Some(reason)),
            };
            Self {
                bank,
                music_assets: [
                    include_bytes!("../assets/audio/music_base.ogg"),
                    include_bytes!("../assets/audio/music_drive.ogg"),
                    include_bytes!("../assets/audio/music_pressure.ogg"),
                ],
                manager: None,
                handles: Vec::new(),
                music_handles: Vec::new(),
                effects_volume: 1.0,
                music_volume: 1.0,
                music_tier: MusicTier::Base,
                music_ducked: false,
                muted: false,
                failure_reason,
                music_failure_reason: None,
            }
        }

        pub(super) fn is_available(&self) -> bool {
            self.failure_reason.is_none()
        }

        pub(super) fn failure_reason(&self) -> Option<&str> {
            self.failure_reason.as_deref()
        }

        pub(super) fn music_available(&self) -> bool {
            self.failure_reason.is_none() && self.music_failure_reason.is_none()
        }

        pub(super) fn music_failure_reason(&self) -> Option<&str> {
            self.failure_reason
                .as_deref()
                .or(self.music_failure_reason.as_deref())
        }

        pub(super) fn activate(&mut self) {
            if self.manager.is_some() || self.failure_reason.is_some() {
                return;
            }

            #[cfg(test)]
            return;

            #[cfg(not(test))]
            match AudioManager::<DefaultBackend>::new(kira::AudioManagerSettings::default()) {
                Ok(mut manager) => {
                    manager.main_track().set_volume(
                        linear_to_decibels(if self.muted { 0.0 } else { 1.0 }),
                        Tween::default(),
                    );
                    self.manager = Some(manager);
                }
                Err(error) => {
                    self.failure_reason = Some(format!("Audio output is unavailable: {error}"));
                }
            }
        }

        pub(super) fn set_muted(&mut self, muted: bool) {
            self.muted = muted;
            if let Some(manager) = self.manager.as_mut() {
                manager.main_track().set_volume(
                    linear_to_decibels(if muted { 0.0 } else { 1.0 }),
                    Tween {
                        duration: std::time::Duration::from_millis(40),
                        ..Default::default()
                    },
                );
            }
        }

        pub(super) fn set_effects_volume(&mut self, volume: f32) {
            self.effects_volume = volume.clamp(0.0, 1.0);
        }

        pub(super) fn set_music_volume(&mut self, volume: f32) {
            self.music_volume = volume.clamp(0.0, 1.0);
            self.update_music_gains(Tween {
                duration: Duration::from_millis(40),
                ..Default::default()
            });
        }

        pub(super) fn play(&mut self, request: CueRequest) {
            let (Some(bank), Some(manager)) = (self.bank.as_ref(), self.manager.as_mut()) else {
                return;
            };
            self.handles
                .retain(|handle| handle.state() != PlaybackState::Stopped);
            if self.handles.len() >= MAX_VOICES {
                let mut oldest = self.handles.remove(0);
                oldest.stop(Tween::default());
            }

            let Some(data) = bank.get(&request.cue) else {
                return;
            };
            let sound = data
                .volume(request.gain_db + linear_gain_db(self.effects_volume))
                .playback_rate(request.rate as f64)
                .panning(request.pan)
                .start_time(StartTime::Delayed(request.delay));
            if let Ok(handle) = manager.play(sound) {
                self.handles.push(handle);
            }
        }

        pub(super) fn stop_effects(&mut self) {
            for handle in &mut self.handles {
                handle.stop(Tween {
                    duration: std::time::Duration::from_millis(15),
                    ..Default::default()
                });
            }
            self.handles.clear();
        }

        pub(super) fn start_music(&mut self, tier: MusicTier) {
            self.stop_music();
            self.music_tier = tier;
            self.music_ducked = false;
            let layer_count = self.music_tier.layer_count();
            let music_volume = self.music_volume;
            let Some(manager) = self.manager.as_mut() else {
                return;
            };
            let start_time = StartTime::Delayed(Duration::from_millis(60));
            let mut handles = Vec::with_capacity(self.music_assets.len());
            let mut failure_reason = None;
            for (index, bytes) in self.music_assets.iter().enumerate() {
                let gain = if index < layer_count {
                    music_volume
                } else {
                    0.0
                };
                let sound = match StreamingSoundData::from_cursor(Cursor::new(*bytes)) {
                    Ok(sound) => sound,
                    Err(error) => {
                        failure_reason = Some(format!("Could not decode music stem: {error}"));
                        break;
                    }
                }
                .loop_region(..)
                .volume(linear_to_decibels(gain))
                .panning(match index {
                    1 => -0.08,
                    2 => 0.08,
                    _ => 0.0,
                })
                .start_time(start_time);
                match manager.play(sound) {
                    Ok(handle) => handles.push(handle),
                    Err(error) => {
                        failure_reason = Some(format!("Could not play music stem: {error}"));
                        break;
                    }
                }
            }
            self.music_handles = handles;
            if let Some(reason) = failure_reason {
                self.music_failure_reason = Some(reason);
                self.stop_music();
            }
        }

        pub(super) fn set_music_tier(&mut self, tier: MusicTier) {
            self.music_tier = tier;
            let delay = self
                .music_handles
                .first()
                .map(StreamingSoundHandle::position)
                .map(|position| {
                    let remainder = position.rem_euclid(MUSIC_BAR_SECONDS);
                    if remainder <= 0.02 {
                        Duration::ZERO
                    } else {
                        Duration::from_secs_f64(MUSIC_BAR_SECONDS - remainder)
                    }
                })
                .unwrap_or(Duration::ZERO);
            self.update_music_gains(Tween {
                start_time: StartTime::Delayed(delay),
                duration: Duration::from_millis(250),
                ..Default::default()
            });
        }

        pub(super) fn pause_music(&mut self) {
            for handle in &mut self.music_handles {
                handle.pause(Tween {
                    duration: Duration::from_millis(15),
                    ..Default::default()
                });
            }
        }

        pub(super) fn resume_music(&mut self) {
            if self.music_handles.is_empty() {
                self.start_music(self.music_tier);
                return;
            }
            for handle in &mut self.music_handles {
                handle.resume(Tween {
                    duration: Duration::from_millis(40),
                    ..Default::default()
                });
            }
        }

        pub(super) fn stop_music(&mut self) {
            for handle in &mut self.music_handles {
                handle.stop(Tween {
                    duration: Duration::from_millis(150),
                    ..Default::default()
                });
            }
            self.music_handles.clear();
        }

        pub(super) fn set_music_ducked(&mut self, ducked: bool) {
            self.music_ducked = ducked;
            self.update_music_gains(Tween {
                duration: if ducked {
                    Duration::from_millis(25)
                } else {
                    Duration::from_millis(180)
                },
                ..Default::default()
            });
        }

        pub(super) fn stop_all(&mut self) {
            self.stop_effects();
            self.stop_music();
        }

        fn update_music_gains(&mut self, tween: Tween) {
            let layer_count = self.music_tier.layer_count();
            let duck_gain = if self.music_ducked { 0.707_945_76 } else { 1.0 };
            for (index, handle) in self.music_handles.iter_mut().enumerate() {
                let gain = if index < layer_count {
                    self.music_volume * duck_gain
                } else {
                    0.0
                };
                handle.set_volume(linear_to_decibels(gain), tween);
            }
        }
    }

    fn linear_to_decibels(volume: f32) -> Decibels {
        if volume <= 0.0 {
            Decibels::SILENCE
        } else {
            Decibels(20.0 * volume.log10())
        }
    }

    fn linear_gain_db(volume: f32) -> f32 {
        if volume <= 0.0 {
            -120.0
        } else {
            20.0 * volume.log10()
        }
    }

    fn load_bank() -> Result<HashMap<Cue, StaticSoundData>, String> {
        let assets: [(Cue, &'static [u8]); 22] = [
            (
                Cue::UiActivate,
                include_bytes!("../assets/audio/ui_activate.wav"),
            ),
            (
                Cue::GameStart,
                include_bytes!("../assets/audio/game_start.wav"),
            ),
            (Cue::Pause, include_bytes!("../assets/audio/pause.wav")),
            (Cue::Resume, include_bytes!("../assets/audio/resume.wav")),
            (Cue::MoveA, include_bytes!("../assets/audio/move_a.wav")),
            (Cue::MoveB, include_bytes!("../assets/audio/move_b.wav")),
            (Cue::Rotate, include_bytes!("../assets/audio/rotate.wav")),
            (Cue::Hold, include_bytes!("../assets/audio/hold.wav")),
            (Cue::Contact, include_bytes!("../assets/audio/contact.wav")),
            (
                Cue::HardDrop,
                include_bytes!("../assets/audio/hard_drop.wav"),
            ),
            (Cue::Lock, include_bytes!("../assets/audio/lock.wav")),
            (
                Cue::ClearSingle,
                include_bytes!("../assets/audio/clear_single.wav"),
            ),
            (
                Cue::ClearDouble,
                include_bytes!("../assets/audio/clear_double.wav"),
            ),
            (
                Cue::ClearTriple,
                include_bytes!("../assets/audio/clear_triple.wav"),
            ),
            (
                Cue::ClearFour,
                include_bytes!("../assets/audio/clear_four.wav"),
            ),
            (
                Cue::AccentTSpin,
                include_bytes!("../assets/audio/accent_tspin.wav"),
            ),
            (
                Cue::AccentCombo,
                include_bytes!("../assets/audio/accent_combo.wav"),
            ),
            (
                Cue::AccentBackToBack,
                include_bytes!("../assets/audio/accent_back_to_back.wav"),
            ),
            (
                Cue::AccentPerfect,
                include_bytes!("../assets/audio/accent_perfect.wav"),
            ),
            (Cue::LevelUp, include_bytes!("../assets/audio/level_up.wav")),
            (
                Cue::GameOver,
                include_bytes!("../assets/audio/game_over.wav"),
            ),
            (Cue::NewBest, include_bytes!("../assets/audio/new_best.wav")),
        ];
        assets
            .into_iter()
            .map(|(cue, bytes)| {
                StaticSoundData::from_cursor(Cursor::new(bytes))
                    .map(|sound| (cue, sound))
                    .map_err(|error| format!("Could not decode {}.wav: {error}", cue.file_stem()))
            })
            .collect()
    }
}

#[cfg(target_arch = "wasm32")]
mod output {
    use wasm_bindgen::prelude::wasm_bindgen;

    use super::{CueRequest, MusicTier};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioAvailable)]
        fn web_audio_available() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioPrepare)]
        fn web_audio_prepare();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioActivate)]
        fn web_audio_activate() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioSetMuted)]
        fn web_audio_set_muted(muted: bool);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioSetEffectsVolume)]
        fn web_audio_set_effects_volume(volume: f32);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioSetMusicVolume)]
        fn web_audio_set_music_volume(volume: f32);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioPlay)]
        fn web_audio_play(name: &str, gain_db: f32, rate: f32, pan: f32, delay_seconds: f64);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioStopEffects)]
        fn web_audio_stop_effects();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioStopAll)]
        fn web_audio_stop_all();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicAvailable)]
        fn web_music_available() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicStart)]
        fn web_music_start(tier: u8);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicSetTier)]
        fn web_music_set_tier(tier: u8);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicPause)]
        fn web_music_pause();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicResume)]
        fn web_music_resume();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicStop)]
        fn web_music_stop();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallMusicSetDucked)]
        fn web_music_set_ducked(ducked: bool);
    }

    pub(super) fn prepare() {
        web_audio_prepare();
    }

    pub(super) struct Output {
        failed: bool,
    }

    impl Output {
        pub(super) fn new() -> Self {
            Self {
                failed: !web_audio_available(),
            }
        }

        pub(super) fn is_available(&self) -> bool {
            !self.failed && web_audio_available()
        }

        pub(super) fn failure_reason(&self) -> Option<&str> {
            self.failed
                .then_some("Web Audio is unavailable in this browser.")
        }

        pub(super) fn music_available(&self) -> bool {
            !self.failed && web_music_available()
        }

        pub(super) fn music_failure_reason(&self) -> Option<&str> {
            (!self.music_available()).then_some("Music assets are unavailable.")
        }

        pub(super) fn activate(&mut self) {
            if !self.failed {
                self.failed = !web_audio_activate();
            }
        }

        pub(super) fn set_muted(&mut self, muted: bool) {
            if !self.failed {
                web_audio_set_muted(muted);
            }
        }

        pub(super) fn set_effects_volume(&mut self, volume: f32) {
            if !self.failed {
                web_audio_set_effects_volume(volume);
            }
        }

        pub(super) fn set_music_volume(&mut self, volume: f32) {
            if !self.failed {
                web_audio_set_music_volume(volume);
            }
        }

        pub(super) fn play(&mut self, request: CueRequest) {
            if !self.failed {
                web_audio_play(
                    request.cue.file_stem(),
                    request.gain_db,
                    request.rate,
                    request.pan,
                    request.delay.as_secs_f64(),
                );
            }
        }

        pub(super) fn stop_effects(&mut self) {
            if !self.failed {
                web_audio_stop_effects();
            }
        }

        pub(super) fn start_music(&mut self, tier: MusicTier) {
            if !self.failed {
                web_music_start(tier.web_value());
            }
        }

        pub(super) fn set_music_tier(&mut self, tier: MusicTier) {
            if !self.failed {
                web_music_set_tier(tier.web_value());
            }
        }

        pub(super) fn pause_music(&mut self) {
            if !self.failed {
                web_music_pause();
            }
        }

        pub(super) fn resume_music(&mut self) {
            if !self.failed {
                web_music_resume();
            }
        }

        pub(super) fn stop_music(&mut self) {
            if !self.failed {
                web_music_stop();
            }
        }

        pub(super) fn set_music_ducked(&mut self, ducked: bool) {
            if !self.failed {
                web_music_set_ducked(ducked);
            }
        }

        pub(super) fn stop_all(&mut self) {
            if !self.failed {
                web_audio_stop_all();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{ClearResult, Point};

    fn clear_result(lines: u8) -> ClearResult {
        ClearResult {
            lines,
            spin: Spin::None,
            perfect_clear: false,
            difficult: lines == 4,
            score_delta: 100,
            combo: Some(0),
            back_to_back: lines == 4,
            back_to_back_bonus: false,
        }
    }

    #[test]
    fn clear_replaces_plain_lock_but_keeps_hard_drop_layer() {
        let point = Point::new(3, crate::game::VISIBLE_TOP as i32);
        let events = [
            GameEvent::HardDropped {
                from: [point; 4],
                to: [Point::new(3, point.y + 12); 4],
            },
            GameEvent::PieceLocked { column: 3 },
            GameEvent::Cleared {
                rows: vec![39],
                result: clear_result(1),
            },
        ];
        let requests = plan_game_audio(&events, false, &mut false);

        assert!(requests.iter().any(|request| request.cue == Cue::HardDrop));
        assert!(
            requests
                .iter()
                .any(|request| request.cue == Cue::ClearSingle)
        );
        assert!(!requests.iter().any(|request| request.cue == Cue::Lock));
    }

    #[test]
    fn special_clear_accents_layer_without_replacing_base_clear() {
        let mut result = clear_result(4);
        result.spin = Spin::Full;
        result.combo = Some(3);
        result.back_to_back_bonus = true;
        result.perfect_clear = true;
        let events = [GameEvent::Cleared {
            rows: vec![36, 37, 38, 39],
            result,
        }];
        let requests = plan_game_audio(&events, false, &mut false);
        let cues = requests
            .iter()
            .map(|request| request.cue)
            .collect::<Vec<_>>();

        assert_eq!(cues[0], Cue::ClearFour);
        assert!(cues.contains(&Cue::AccentTSpin));
        assert!(cues.contains(&Cue::AccentCombo));
        assert!(cues.contains(&Cue::AccentBackToBack));
        assert!(cues.contains(&Cue::AccentPerfect));
    }

    #[test]
    fn game_over_has_priority_and_new_best_adds_flourish() {
        let events = [
            GameEvent::Moved {
                direction: MovementDirection::Left,
                column: 3,
            },
            GameEvent::GameOver,
        ];
        let requests = plan_game_audio(&events, true, &mut false);

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].cue, Cue::GameOver);
        assert_eq!(requests[1].cue, Cue::NewBest);
        assert!(requests[1].delay > Duration::ZERO);
    }

    #[test]
    fn board_panning_is_narrow_and_centered() {
        assert_eq!(column_pan(4), -column_pan(5));
        assert_eq!(column_pan(-20), -0.35);
        assert_eq!(column_pan(20), 0.35);
    }

    #[test]
    fn music_levels_map_to_three_permanent_tiers() {
        assert_eq!(baseline_music_tier(1), MusicTier::Base);
        assert_eq!(baseline_music_tier(4), MusicTier::Base);
        assert_eq!(baseline_music_tier(5), MusicTier::Drive);
        assert_eq!(baseline_music_tier(9), MusicTier::Drive);
        assert_eq!(baseline_music_tier(10), MusicTier::Pressure);
        assert_eq!(baseline_music_tier(100), MusicTier::Pressure);
    }

    #[test]
    fn board_danger_advances_one_tier_with_hysteresis() {
        let now = Instant::now();
        let mut audio = AudioSystem::new(0.7, 0.35, false);

        audio.update_music(1, None, now);
        assert_eq!(audio.music_tier, MusicTier::Base);
        assert!(!audio.danger_active);

        audio.update_music(1, Some(DANGER_START_ROW), now);
        assert_eq!(audio.music_tier, MusicTier::Drive);
        assert!(audio.danger_active);

        audio.update_music(1, Some(DANGER_CLEAR_ROW - 1), now);
        assert_eq!(audio.music_tier, MusicTier::Drive);
        assert!(audio.danger_active);

        audio.update_music(1, Some(DANGER_CLEAR_ROW), now);
        assert_eq!(audio.music_tier, MusicTier::Base);
        assert!(!audio.danger_active);

        audio.update_music(5, Some(DANGER_START_ROW), now);
        assert_eq!(audio.music_tier, MusicTier::Pressure);
    }

    #[test]
    fn volume_percentages_use_a_perceptual_gain_curve() {
        assert_eq!(perceptual_gain(0.0), 0.0);
        assert!((perceptual_gain(0.35) - 0.1225).abs() < f32::EPSILON);
        assert!((perceptual_gain(0.70) - 0.49).abs() < f32::EPSILON);
        assert_eq!(perceptual_gain(1.0), 1.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn checked_in_music_stems_open_as_streaming_vorbis() {
        use std::io::Cursor;

        use kira::sound::streaming::StreamingSoundData;

        for bytes in [
            include_bytes!("../assets/audio/music_base.ogg").as_slice(),
            include_bytes!("../assets/audio/music_drive.ogg").as_slice(),
            include_bytes!("../assets/audio/music_pressure.ogg").as_slice(),
        ] {
            assert!(StreamingSoundData::from_cursor(Cursor::new(bytes)).is_ok());
        }
    }
}
