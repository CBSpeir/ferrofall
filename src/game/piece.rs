#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Tetromino {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Tetromino {
    pub(crate) const ALL: [Self; 7] = [
        Self::I,
        Self::O,
        Self::T,
        Self::S,
        Self::Z,
        Self::J,
        Self::L,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Rotation {
    Spawn,
    Right,
    Reverse,
    Left,
}

impl Rotation {
    pub(crate) fn clockwise(self) -> Self {
        match self {
            Self::Spawn => Self::Right,
            Self::Right => Self::Reverse,
            Self::Reverse => Self::Left,
            Self::Left => Self::Spawn,
        }
    }

    pub(crate) fn counterclockwise(self) -> Self {
        match self {
            Self::Spawn => Self::Left,
            Self::Right => Self::Spawn,
            Self::Reverse => Self::Right,
            Self::Left => Self::Reverse,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl Point {
    pub(crate) const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ActivePiece {
    pub(crate) kind: Tetromino,
    pub(crate) rotation: Rotation,
    pub(crate) origin: Point,
}

impl ActivePiece {
    pub(crate) const fn spawn(kind: Tetromino) -> Self {
        Self {
            kind,
            rotation: Rotation::Spawn,
            origin: Point::new(3, 19),
        }
    }

    pub(crate) fn blocks(self) -> [Point; 4] {
        offsets(self.kind, self.rotation)
            .map(|offset| Point::new(self.origin.x + offset.x, self.origin.y + offset.y))
    }

    pub(crate) fn translated(self, dx: i32, dy: i32) -> Self {
        Self {
            origin: Point::new(self.origin.x + dx, self.origin.y + dy),
            ..self
        }
    }

    pub(crate) fn rotated(self, rotation: Rotation, dx: i32, dy: i32) -> Self {
        Self {
            rotation,
            origin: Point::new(self.origin.x + dx, self.origin.y + dy),
            ..self
        }
    }
}

pub(crate) fn preview_offsets(kind: Tetromino) -> [Point; 4] {
    offsets(kind, Rotation::Spawn)
}

fn offsets(kind: Tetromino, rotation: Rotation) -> [Point; 4] {
    use Point as P;
    use Rotation::{Left, Reverse, Right, Spawn};
    use Tetromino::{I, J, L, O, S, T, Z};

    match (kind, rotation) {
        (I, Spawn) => [P::new(0, 1), P::new(1, 1), P::new(2, 1), P::new(3, 1)],
        (I, Right) => [P::new(2, 0), P::new(2, 1), P::new(2, 2), P::new(2, 3)],
        (I, Reverse) => [P::new(0, 2), P::new(1, 2), P::new(2, 2), P::new(3, 2)],
        (I, Left) => [P::new(1, 0), P::new(1, 1), P::new(1, 2), P::new(1, 3)],
        (O, _) => [P::new(1, 0), P::new(2, 0), P::new(1, 1), P::new(2, 1)],
        (T, Spawn) => [P::new(1, 0), P::new(0, 1), P::new(1, 1), P::new(2, 1)],
        (T, Right) => [P::new(1, 0), P::new(1, 1), P::new(2, 1), P::new(1, 2)],
        (T, Reverse) => [P::new(0, 1), P::new(1, 1), P::new(2, 1), P::new(1, 2)],
        (T, Left) => [P::new(1, 0), P::new(0, 1), P::new(1, 1), P::new(1, 2)],
        (S, Spawn) => [P::new(1, 0), P::new(2, 0), P::new(0, 1), P::new(1, 1)],
        (S, Right) => [P::new(1, 0), P::new(1, 1), P::new(2, 1), P::new(2, 2)],
        (S, Reverse) => [P::new(1, 1), P::new(2, 1), P::new(0, 2), P::new(1, 2)],
        (S, Left) => [P::new(0, 0), P::new(0, 1), P::new(1, 1), P::new(1, 2)],
        (Z, Spawn) => [P::new(0, 0), P::new(1, 0), P::new(1, 1), P::new(2, 1)],
        (Z, Right) => [P::new(2, 0), P::new(1, 1), P::new(2, 1), P::new(1, 2)],
        (Z, Reverse) => [P::new(0, 1), P::new(1, 1), P::new(1, 2), P::new(2, 2)],
        (Z, Left) => [P::new(1, 0), P::new(0, 1), P::new(1, 1), P::new(0, 2)],
        (J, Spawn) => [P::new(0, 0), P::new(0, 1), P::new(1, 1), P::new(2, 1)],
        (J, Right) => [P::new(1, 0), P::new(2, 0), P::new(1, 1), P::new(1, 2)],
        (J, Reverse) => [P::new(0, 1), P::new(1, 1), P::new(2, 1), P::new(2, 2)],
        (J, Left) => [P::new(1, 0), P::new(1, 1), P::new(0, 2), P::new(1, 2)],
        (L, Spawn) => [P::new(2, 0), P::new(0, 1), P::new(1, 1), P::new(2, 1)],
        (L, Right) => [P::new(1, 0), P::new(1, 1), P::new(1, 2), P::new(2, 2)],
        (L, Reverse) => [P::new(0, 1), P::new(1, 1), P::new(2, 1), P::new(0, 2)],
        (L, Left) => [P::new(0, 0), P::new(1, 0), P::new(1, 1), P::new(1, 2)],
    }
}

pub(crate) fn kick_tests(kind: Tetromino, from: Rotation, to: Rotation) -> [(i32, i32); 5] {
    if kind == Tetromino::O {
        return [(0, 0); 5];
    }

    if kind == Tetromino::I {
        return i_kicks(from, to);
    }

    jltsz_kicks(from, to)
}

fn jltsz_kicks(from: Rotation, to: Rotation) -> [(i32, i32); 5] {
    use Rotation::{Left as L, Reverse as Two, Right as R, Spawn as Zero};

    match (from, to) {
        (Zero, R) | (Two, R) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
        (R, Zero) | (R, Two) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        (Two, L) | (Zero, L) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        (L, Two) | (L, Zero) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        _ => [(0, 0); 5],
    }
}

fn i_kicks(from: Rotation, to: Rotation) -> [(i32, i32); 5] {
    use Rotation::{Left as L, Reverse as Two, Right as R, Spawn as Zero};

    match (from, to) {
        (Zero, R) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
        (R, Zero) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
        (R, Two) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
        (Two, R) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
        (Two, L) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
        (L, Two) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
        (L, Zero) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
        (Zero, L) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
        _ => [(0, 0); 5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_piece_has_four_unique_blocks_in_every_rotation() {
        for kind in Tetromino::ALL {
            for rotation in [
                Rotation::Spawn,
                Rotation::Right,
                Rotation::Reverse,
                Rotation::Left,
            ] {
                let blocks = ActivePiece {
                    kind,
                    rotation,
                    origin: Point::default(),
                }
                .blocks();

                for (index, block) in blocks.iter().enumerate() {
                    assert!(!blocks[index + 1..].contains(block));
                }
            }
        }
    }

    #[test]
    fn four_clockwise_rotations_return_to_spawn() {
        let rotation = Rotation::Spawn
            .clockwise()
            .clockwise()
            .clockwise()
            .clockwise();
        assert_eq!(rotation, Rotation::Spawn);
    }
}
