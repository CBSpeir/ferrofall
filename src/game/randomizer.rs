use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

use super::piece::Tetromino;

pub(crate) struct SevenBag {
    rng: ChaCha8Rng,
    bag: [Tetromino; 7],
    cursor: usize,
}

impl SevenBag {
    pub(crate) fn new(seed: u64) -> Self {
        let mut bag = Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            bag: Tetromino::ALL,
            cursor: Tetromino::ALL.len(),
        };
        bag.refill();
        bag
    }

    pub(crate) fn next(&mut self) -> Tetromino {
        if self.cursor == self.bag.len() {
            self.refill();
        }

        let piece = self.bag[self.cursor];
        self.cursor += 1;
        piece
    }

    fn refill(&mut self) {
        self.bag = Tetromino::ALL;
        self.bag.shuffle(&mut self.rng);
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bag_contains_each_piece_exactly_once() {
        let mut bag = SevenBag::new(42);
        for _ in 0..8 {
            let mut pieces = (0..7).map(|_| bag.next()).collect::<Vec<_>>();
            pieces.sort_by_key(|piece| Tetromino::ALL.iter().position(|p| p == piece));
            assert_eq!(pieces, Tetromino::ALL);
        }
    }

    #[test]
    fn equal_seeds_produce_equal_sequences() {
        let mut first = SevenBag::new(1234);
        let mut second = SevenBag::new(1234);
        for _ in 0..100 {
            assert_eq!(first.next(), second.next());
        }
    }
}
