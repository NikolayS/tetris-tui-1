use rand::seq::SliceRandom;

use crate::game::piece::PieceKind;

const ALL_PIECES: [PieceKind; 7] = [
    PieceKind::I,
    PieceKind::O,
    PieceKind::T,
    PieceKind::S,
    PieceKind::Z,
    PieceKind::J,
    PieceKind::L,
];

/// 7-bag randomizer.
///
/// Produces pieces in aligned 7-piece windows that are permutations of
/// [I, O, T, S, Z, J, L].  The invariant is on ALIGNED windows, not
/// sliding windows — see SPEC §4 round-1 decision.
pub struct Bag<R: rand::RngCore> {
    rng: R,
    pending: Vec<PieceKind>,
}

impl<R: rand::RngCore> Bag<R> {
    pub fn new(rng: R) -> Self {
        Bag {
            rng,
            pending: Vec::new(),
        }
    }

    /// Refill the pending buffer with a freshly shuffled batch.
    fn refill(&mut self) {
        let mut batch = ALL_PIECES;
        batch.shuffle(&mut self.rng);
        self.pending.extend_from_slice(&batch);
    }
}

impl<R: rand::RngCore> Iterator for Bag<R> {
    type Item = PieceKind;

    /// Return the next piece, refilling from a freshly shuffled bag when
    /// the pending buffer is empty.
    ///
    /// The bag never runs out, so this always returns `Some`.
    fn next(&mut self) -> Option<PieceKind> {
        if self.pending.is_empty() {
            self.refill();
        }
        // pending is non-empty; pop from the front so pieces emerge in
        // the shuffled order.
        Some(self.pending.remove(0))
    }
}
