#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Spin {
    None,
    Mini,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClearResult {
    pub(crate) lines: u8,
    pub(crate) spin: Spin,
    pub(crate) perfect_clear: bool,
    pub(crate) difficult: bool,
    pub(crate) score_delta: u64,
    pub(crate) combo: Option<u32>,
    pub(crate) back_to_back: bool,
    pub(crate) back_to_back_bonus: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScoreState {
    score: u64,
    lines: u32,
    level: u32,
    combo: i32,
    back_to_back: bool,
}

impl Default for ScoreState {
    fn default() -> Self {
        Self {
            score: 0,
            lines: 0,
            level: 1,
            combo: -1,
            back_to_back: false,
        }
    }
}

impl ScoreState {
    pub(crate) fn score(&self) -> u64 {
        self.score
    }

    pub(crate) fn lines(&self) -> u32 {
        self.lines
    }

    pub(crate) fn level(&self) -> u32 {
        self.level
    }

    pub(crate) fn combo(&self) -> Option<u32> {
        (self.combo >= 0).then_some(self.combo as u32)
    }

    pub(crate) fn back_to_back(&self) -> bool {
        self.back_to_back
    }

    pub(crate) fn add_drop_points(&mut self, points: u64) {
        self.score = self.score.saturating_add(points);
    }

    pub(crate) fn apply_clear(
        &mut self,
        lines: u8,
        spin: Spin,
        perfect_clear: bool,
    ) -> ClearResult {
        let scoring_level = self.level as u64;
        let difficult = lines == 4 || (spin != Spin::None && lines > 0);
        let was_back_to_back = self.back_to_back;
        let mut base = base_points(lines, spin);

        if difficult && was_back_to_back {
            base = base.saturating_mul(3) / 2;
        }

        if lines > 0 {
            self.combo += 1;
            self.back_to_back = difficult;
        } else {
            self.combo = -1;
        }

        let combo_points = if lines > 0 {
            50_u64.saturating_mul(self.combo.max(0) as u64)
        } else {
            0
        };
        let perfect_clear_points = if perfect_clear {
            match lines {
                1 => 800,
                2 => 1_200,
                3 => 1_800,
                4 if was_back_to_back => 3_200,
                4 => 2_000,
                _ => 0,
            }
        } else {
            0
        };
        let score_delta = base
            .saturating_add(combo_points)
            .saturating_add(perfect_clear_points)
            .saturating_mul(scoring_level);

        self.score = self.score.saturating_add(score_delta);
        self.lines = self.lines.saturating_add(lines as u32);
        self.level = self.lines / 10 + 1;

        ClearResult {
            lines,
            spin,
            perfect_clear,
            difficult,
            score_delta,
            combo: self.combo(),
            back_to_back: self.back_to_back,
            back_to_back_bonus: difficult && was_back_to_back,
        }
    }
}

fn base_points(lines: u8, spin: Spin) -> u64 {
    match (spin, lines) {
        (Spin::None, 0) => 0,
        (Spin::None, 1) => 100,
        (Spin::None, 2) => 300,
        (Spin::None, 3) => 500,
        (Spin::None, 4) => 800,
        (Spin::Mini, 0) => 100,
        (Spin::Mini, 1) => 200,
        (Spin::Mini, 2) => 400,
        (Spin::Full, 0) => 400,
        (Spin::Full, 1) => 800,
        (Spin::Full, 2) => 1_200,
        (Spin::Full, 3) => 1_600,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combo_and_back_to_back_follow_guideline_rules() {
        let mut score = ScoreState::default();
        let first = score.apply_clear(4, Spin::None, false);
        let second = score.apply_clear(4, Spin::None, false);
        let no_clear = score.apply_clear(0, Spin::None, false);

        assert_eq!(first.score_delta, 800);
        assert_eq!(second.score_delta, 1_250);
        assert!(!first.back_to_back_bonus);
        assert!(second.back_to_back_bonus);
        assert!(second.back_to_back);
        assert_eq!(no_clear.combo, None);
        assert!(no_clear.back_to_back);
    }

    #[test]
    fn clear_uses_old_level_then_advances() {
        let mut score = ScoreState {
            lines: 9,
            ..Default::default()
        };
        let clear = score.apply_clear(1, Spin::None, false);
        assert_eq!(clear.score_delta, 100);
        assert_eq!(score.level(), 2);
    }

    #[test]
    fn perfect_clear_bonus_is_applied() {
        let mut score = ScoreState::default();
        let clear = score.apply_clear(4, Spin::None, true);
        assert_eq!(clear.score_delta, 2_800);
    }
}
