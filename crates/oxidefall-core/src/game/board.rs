use super::piece::{ActivePiece, Point, Tetromino};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 40;
pub const VISIBLE_HEIGHT: usize = 20;
pub const VISIBLE_TOP: usize = BOARD_HEIGHT - VISIBLE_HEIGHT;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board {
    cells: [[Option<Tetromino>; BOARD_WIDTH]; BOARD_HEIGHT],
}

impl Default for Board {
    fn default() -> Self {
        Self {
            cells: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
        }
    }
}

impl Board {
    pub fn cell(&self, x: usize, y: usize) -> Option<Tetromino> {
        self.cells[y][x]
    }

    pub fn collides(&self, piece: ActivePiece) -> bool {
        piece
            .blocks()
            .into_iter()
            .any(|block| self.point_is_occupied(block))
    }

    pub fn point_is_occupied(&self, point: Point) -> bool {
        if point.x < 0
            || point.x >= BOARD_WIDTH as i32
            || point.y < 0
            || point.y >= BOARD_HEIGHT as i32
        {
            return true;
        }

        self.cells[point.y as usize][point.x as usize].is_some()
    }

    pub fn lock(&mut self, piece: ActivePiece) {
        for block in piece.blocks() {
            debug_assert!(block.x >= 0 && block.x < BOARD_WIDTH as i32);
            debug_assert!(block.y >= 0 && block.y < BOARD_HEIGHT as i32);
            self.cells[block.y as usize][block.x as usize] = Some(piece.kind);
        }
    }

    pub fn clear_full_rows(&mut self) -> Vec<usize> {
        let full_rows: Vec<_> = self
            .cells
            .iter()
            .enumerate()
            .filter_map(|(y, row)| row.iter().all(Option::is_some).then_some(y))
            .collect();

        if full_rows.is_empty() {
            return full_rows;
        }

        let mut write_y = BOARD_HEIGHT as isize - 1;
        for read_y in (0..BOARD_HEIGHT).rev() {
            if !full_rows.contains(&read_y) {
                self.cells[write_y as usize] = self.cells[read_y];
                write_y -= 1;
            }
        }

        while write_y >= 0 {
            self.cells[write_y as usize] = [None; BOARD_WIDTH];
            write_y -= 1;
        }

        full_rows
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().flatten().all(Option::is_none)
    }

    #[cfg(test)]
    pub fn set_cell(&mut self, x: usize, y: usize, value: Option<Tetromino>) {
        self.cells[y][x] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_rows_compacts_everything_above() {
        let mut board = Board::default();
        for x in 0..BOARD_WIDTH {
            board.set_cell(x, BOARD_HEIGHT - 1, Some(Tetromino::I));
        }
        board.set_cell(2, BOARD_HEIGHT - 2, Some(Tetromino::T));

        assert_eq!(board.clear_full_rows(), vec![BOARD_HEIGHT - 1]);
        assert_eq!(board.cell(2, BOARD_HEIGHT - 1), Some(Tetromino::T));
        assert!(
            board.cells[..BOARD_HEIGHT - 1]
                .iter()
                .flatten()
                .all(Option::is_none)
        );
    }

    #[test]
    fn boundaries_count_as_collisions() {
        let board = Board::default();
        let piece = ActivePiece::spawn(Tetromino::I).translated(-4, 0);
        assert!(board.collides(piece));
    }
}
