use blocktxt::game::bag::Bag;
use blocktxt::game::piece::PieceKind;
use proptest::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

const ALL_KINDS: [PieceKind; 7] = [
    PieceKind::I,
    PieceKind::O,
    PieceKind::T,
    PieceKind::S,
    PieceKind::Z,
    PieceKind::J,
    PieceKind::L,
];

fn draw_n(bag: &mut Bag<ChaCha8Rng>, n: usize) -> Vec<PieceKind> {
    (0..n).map(|_| bag.next().unwrap()).collect()
}

// -----------------------------------------------------------------------
// aligned_bags_are_permutations
// Proptest: 100 cases, 1000 pieces each.
// -----------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn aligned_bags_are_permutations(seed in 0u64..u64::MAX) {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let mut bag = Bag::new(rng);
        let pieces = draw_n(&mut bag, 1000);

        // Aligned windows: [0..7], [7..14], [14..21], …
        for chunk in pieces.chunks_exact(7) {
            let mut counts = [0u8; 7];
            for &p in chunk {
                let idx = kind_index(p);
                counts[idx] += 1;
            }
            // Every kind appears exactly once.
            prop_assert_eq!(
                counts,
                [1u8; 7],
                "chunk {:?} is not a permutation of all 7 kinds",
                chunk
            );
        }
    }
}

// -----------------------------------------------------------------------
// max_gap_between_same_piece_is_at_most_12
// Proptest: 50 cases, 10000 pieces each.
// -----------------------------------------------------------------------
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn max_gap_between_same_piece_is_at_most_12(seed in 0u64..u64::MAX) {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let mut bag = Bag::new(rng);
        let pieces = draw_n(&mut bag, 10_000);

        for &kind in &ALL_KINDS {
            let mut last_seen: Option<usize> = None;
            for (i, &p) in pieces.iter().enumerate() {
                if p == kind {
                    if let Some(prev) = last_seen {
                        let gap = i - prev - 1;
                        prop_assert!(
                            gap <= 12,
                            "kind {:?}: gap {} > 12 between indices {} and {}",
                            kind,
                            gap,
                            prev,
                            i
                        );
                    }
                    last_seen = Some(i);
                }
            }
        }
    }
}

// -----------------------------------------------------------------------
// sliding_window_uniqueness_is_NOT_a_property
//
// Documents the SPEC §4 round-1 decision: the invariant is aligned-bag
// permutations, NOT sliding-window uniqueness.
//
// We build a concrete 14-piece sequence from two known bags and show that
// the sliding window starting at index 1 (window [1..8]) can contain a
// repeated piece — if the same kind happens to be at position 6 of bag 1
// and position 0 of bag 2.
// -----------------------------------------------------------------------
#[test]
fn sliding_window_uniqueness_is_not_a_property() {
    // Construct two bags whose boundaries produce a cross-boundary repeat.
    // Bag 1 ends with kind X at position 6; bag 2 starts with kind X at
    // position 0.  We search over seeds until we find such a pair.
    let mut found_duplicate_in_sliding_window = false;

    'outer: for seed in 0u64..10_000 {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let mut bag = Bag::new(rng);
        let pieces = draw_n(&mut bag, 14);

        // Aligned bags are guaranteed permutations.
        // Check sliding windows [i..i+7] for i in 1..8.
        for start in 1..8usize {
            let window = &pieces[start..start + 7];
            let mut counts = [0u8; 7];
            for &p in window {
                counts[kind_index(p)] += 1;
            }
            if counts.iter().any(|&c| c > 1) {
                found_duplicate_in_sliding_window = true;
                break 'outer;
            }
        }
    }

    assert!(
        found_duplicate_in_sliding_window,
        "expected to find at least one seed where a non-aligned \
         sliding window contains a repeated piece kind — \
         this proves the invariant is aligned-bag, not sliding-window"
    );
}

// -----------------------------------------------------------------------
// seeded_bag_is_deterministic
// -----------------------------------------------------------------------
#[test]
fn seeded_bag_is_deterministic() {
    let mut bag1 = Bag::new(ChaCha8Rng::seed_from_u64(42));
    let mut bag2 = Bag::new(ChaCha8Rng::seed_from_u64(42));

    let seq1 = draw_n(&mut bag1, 100);
    let seq2 = draw_n(&mut bag2, 100);

    assert_eq!(seq1, seq2, "same seed must yield identical sequences");
}

// -----------------------------------------------------------------------
// different_seeds_produce_different_sequences
// -----------------------------------------------------------------------
#[test]
fn different_seeds_produce_different_sequences() {
    let mut bag1 = Bag::new(ChaCha8Rng::seed_from_u64(1));
    let mut bag2 = Bag::new(ChaCha8Rng::seed_from_u64(2));

    let seq1 = draw_n(&mut bag1, 100);
    let seq2 = draw_n(&mut bag2, 100);

    assert_ne!(
        seq1, seq2,
        "different seeds should (with high probability) differ"
    );
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn kind_index(kind: PieceKind) -> usize {
    match kind {
        PieceKind::I => 0,
        PieceKind::O => 1,
        PieceKind::T => 2,
        PieceKind::S => 3,
        PieceKind::Z => 4,
        PieceKind::J => 5,
        PieceKind::L => 6,
    }
}
