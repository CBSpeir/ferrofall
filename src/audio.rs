use std::time::Duration;

use crate::game::{GameEvent, MovementDirection, RotationDirection, Spin, VISIBLE_HEIGHT};

pub(crate) const DEFAULT_VOLUME: f32 = 0.70;
#[cfg(not(target_arch = "wasm32"))]
const MAX_VOICES: usize = 16;

#[cfg(target_arch = "wasm32")]
pub(crate) fn prepare_web_audio() {
    output::prepare();
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
    volume: f32,
    muted: bool,
    alternate_move: bool,
}

impl AudioSystem {
    pub(crate) fn new(volume: f32, muted: bool) -> Self {
        let volume = volume.clamp(0.0, 1.0);
        let mut output = output::Output::new();
        output.set_master_volume(if muted { 0.0 } else { volume });
        Self {
            output,
            volume,
            muted,
            alternate_move: false,
        }
    }

    pub(crate) fn is_available(&self) -> bool {
        self.output.is_available()
    }

    pub(crate) fn failure_reason(&self) -> Option<&str> {
        self.output.failure_reason()
    }

    pub(crate) const fn volume(&self) -> f32 {
        self.volume
    }

    pub(crate) const fn is_muted(&self) -> bool {
        self.muted
    }

    pub(crate) fn activate(&mut self) {
        self.output.activate();
        self.apply_master_volume();
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_master_volume();
    }

    pub(crate) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.apply_master_volume();
    }

    pub(crate) fn toggle_muted(&mut self) {
        self.set_muted(!self.muted);
        if !self.muted {
            self.play_request(CueRequest::new(Cue::Resume, -7.0));
        }
    }

    pub(crate) fn stop_all(&mut self) {
        self.output.stop_all();
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
            self.stop_all();
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

    fn play_request(&mut self, request: CueRequest) {
        if self.muted || self.volume <= 0.0 {
            return;
        }
        self.output.play(request);
    }

    fn apply_master_volume(&mut self) {
        self.output
            .set_master_volume(if self.muted { 0.0 } else { self.volume });
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

    use kira::sound::PlaybackState;
    use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
    use kira::{AudioManager, Decibels, DefaultBackend, StartTime, Tween};

    use super::{Cue, CueRequest, MAX_VOICES};

    pub(super) struct Output {
        bank: Option<HashMap<Cue, StaticSoundData>>,
        manager: Option<AudioManager<DefaultBackend>>,
        handles: Vec<StaticSoundHandle>,
        master_volume: f32,
        failure_reason: Option<String>,
    }

    impl Output {
        pub(super) fn new() -> Self {
            match load_bank() {
                Ok(bank) => Self {
                    bank: Some(bank),
                    manager: None,
                    handles: Vec::new(),
                    master_volume: 1.0,
                    failure_reason: None,
                },
                Err(reason) => Self {
                    bank: None,
                    manager: None,
                    handles: Vec::new(),
                    master_volume: 1.0,
                    failure_reason: Some(reason),
                },
            }
        }

        pub(super) fn is_available(&self) -> bool {
            self.failure_reason.is_none()
        }

        pub(super) fn failure_reason(&self) -> Option<&str> {
            self.failure_reason.as_deref()
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
                    manager
                        .main_track()
                        .set_volume(linear_to_decibels(self.master_volume), Tween::default());
                    self.manager = Some(manager);
                }
                Err(error) => {
                    self.failure_reason = Some(format!("Audio output is unavailable: {error}"));
                }
            }
        }

        pub(super) fn set_master_volume(&mut self, volume: f32) {
            self.master_volume = volume.clamp(0.0, 1.0);
            if let Some(manager) = self.manager.as_mut() {
                manager.main_track().set_volume(
                    linear_to_decibels(self.master_volume),
                    Tween {
                        duration: std::time::Duration::from_millis(40),
                        ..Default::default()
                    },
                );
            }
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
                .volume(request.gain_db)
                .playback_rate(request.rate as f64)
                .panning(request.pan)
                .start_time(StartTime::Delayed(request.delay));
            if let Ok(handle) = manager.play(sound) {
                self.handles.push(handle);
            }
        }

        pub(super) fn stop_all(&mut self) {
            for handle in &mut self.handles {
                handle.stop(Tween {
                    duration: std::time::Duration::from_millis(15),
                    ..Default::default()
                });
            }
            self.handles.clear();
        }
    }

    fn linear_to_decibels(volume: f32) -> Decibels {
        if volume <= 0.0 {
            Decibels::SILENCE
        } else {
            Decibels(20.0 * volume.log10())
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

    use super::CueRequest;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioAvailable)]
        fn web_audio_available() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioPrepare)]
        fn web_audio_prepare();
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioActivate)]
        fn web_audio_activate() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioSetMasterVolume)]
        fn web_audio_set_master_volume(volume: f32);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioPlay)]
        fn web_audio_play(name: &str, gain_db: f32, rate: f32, pan: f32, delay_seconds: f64);
        #[wasm_bindgen(js_namespace = window, js_name = oxidefallAudioStopAll)]
        fn web_audio_stop_all();
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

        pub(super) fn activate(&mut self) {
            if !self.failed {
                self.failed = !web_audio_activate();
            }
        }

        pub(super) fn set_master_volume(&mut self, volume: f32) {
            if !self.failed {
                web_audio_set_master_volume(volume);
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
}
