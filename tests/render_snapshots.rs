//! Snapshot tests using ratatui TestBackend + insta.
//!
//! For first run: `INSTA_UPDATE=always cargo test --test render_snapshots`
//! Then review with `cargo insta review` or accept inline.

use std::time::{Duration, Instant};

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use blocktxt::clock::{Clock, FakeClock};
use blocktxt::game::piece::{Piece, PieceKind, Rotation};
use blocktxt::game::state::GameState;
use blocktxt::persistence::{HighScore, HighScoreStore};
use blocktxt::render::theme::Palette;
use blocktxt::render::{board_view, hud, Theme};
use blocktxt::Input;

// ── helpers ───────────────────────────────────────────────────────────────────

fn fake_state() -> GameState {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut gs = GameState::new(42, clock);
    // Transition to Playing so render tests see a normal in-game state.
    gs.step(std::time::Duration::ZERO, &[blocktxt::Input::StartGame]);
    gs
}

/// Render a terminal buffer to a multiline string (one char per cell).
fn buf_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer().clone();
    let lines: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| {
                    let cell = &buf[(x, y)];
                    cell.symbol().chars().next().unwrap_or(' ')
                })
                .collect()
        })
        .collect();
    lines.join("\n")
}

// ── existing snapshots (preserved, not churned) ───────────────────────────────

/// Render the HUD panel to a TestBackend and snapshot it.
#[test]
fn snapshot_hud_empty_state() {
    let state = fake_state();
    let theme = Theme::monochrome();

    let backend = TestBackend::new(20, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            hud::draw(f, area, &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_empty_state", buf_to_string(&terminal));
}

/// Render the HUD in paused state and snapshot it.
#[test]
fn snapshot_hud_paused() {
    let mut state = fake_state();
    // Pause the game.
    state.step(std::time::Duration::ZERO, &[Input::Pause]);

    let theme = Theme::monochrome();

    let backend = TestBackend::new(20, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            hud::draw(f, area, &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_paused", buf_to_string(&terminal));
}

/// Render the full board_view on an empty board.
#[test]
fn snapshot_board_view_empty() {
    let state = fake_state();
    let theme = Theme::monochrome();

    // 22 wide (board), 22 tall (20 visible + 2 border).
    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            board_view::draw(f, area, &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_empty", buf_to_string(&terminal));
}

// ── new snapshots (#30) ───────────────────────────────────────────────────────

/// HUD with a non-trivial score, level, and line count.
#[test]
fn snapshot_hud_with_score() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    // Inject stats directly via public fields.
    state.score = 100_000;
    // Also set the rollup display to the same value so the snapshot is stable.
    state.score_display.current = 100_000;
    state.score_display.target = 100_000;
    state.level = 5;
    state.lines_cleared = 40;

    let theme = Theme::monochrome();
    let backend = TestBackend::new(20, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_with_score", buf_to_string(&terminal));
}

/// Board view with a locked piece stack and the active piece + ghost overlay.
#[test]
fn snapshot_board_view_with_active_piece_and_ghost() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);

    // Fill the bottom 3 rows of the visible area (board rows 37..40)
    // with O pieces to give the ghost something to land on.
    for row in 37..40usize {
        for col in 0..10usize {
            state.board.set(col, row, PieceKind::O);
        }
    }

    // Place the active piece near the top-middle of the visible area.
    state.active = Some(Piece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        origin: (3, 22),
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!(
        "board_view_with_active_piece_and_ghost",
        buf_to_string(&terminal)
    );
}

/// Game-over overlay with a regular (non-best) score.
#[test]
fn snapshot_game_over_overlay_regular() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    state.score = 500;

    // Force game-over phase via hard-drops until game ends.
    for _ in 0..300 {
        state.step(Duration::ZERO, &[Input::HardDrop]);
        if matches!(state.phase, blocktxt::Phase::GameOver { .. }) {
            break;
        }
    }

    // Clear the zoom animation so the snapshot captures the final full-size
    // overlay (not the animated intermediate state).
    state.gameover_zoom = None;

    // Build a store where the existing best (10_000) beats our score (500).
    let mut store = HighScoreStore::new();
    store.insert(HighScore {
        name: "prev".into(),
        score: 10_000,
        level: 3,
        lines: 20,
        ts: 0,
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(20, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw_with_scores(f, f.area(), &state, &theme, Some(&store));
        })
        .unwrap();

    insta::assert_snapshot!("game_over_overlay_regular", buf_to_string(&terminal));
}

/// Game-over overlay with a new-best score: highlighted + "NEW BEST!" banner.
#[test]
fn snapshot_game_over_overlay_new_best() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);

    // Force game-over.
    for _ in 0..300 {
        state.step(Duration::ZERO, &[Input::HardDrop]);
        if matches!(state.phase, blocktxt::Phase::GameOver { .. }) {
            break;
        }
    }

    // Clear the zoom animation so the snapshot captures the final full-size
    // overlay (not the animated intermediate state).
    state.gameover_zoom = None;

    // Build an empty store so every score is a new best.
    let store = HighScoreStore::new();

    let theme = Theme::monochrome();
    let backend = TestBackend::new(22, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw_with_scores(f, f.area(), &state, &theme, Some(&store));
        })
        .unwrap();

    insta::assert_snapshot!("game_over_overlay_new_best", buf_to_string(&terminal));
}

/// HUD rendered in monochrome mode via NO_COLOR env var (no_color flag path).
///
/// Uses `Theme::detect(true, ...)` to simulate `--no-color` flag so the test
/// is env-var-free and therefore safe for parallel execution.
#[test]
fn snapshot_hud_no_color_mode() {
    let state = fake_state();
    // Simulate NO_COLOR by passing no_color_flag=true; avoids env mutation.
    let theme = Theme::detect(true, Palette::default());

    let backend = TestBackend::new(20, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_no_color_mode", buf_to_string(&terminal));
}

/// Board view with a locked stack at the bottom and no active piece.
///
/// Represents the board state just before the game-over overlay appears:
/// the stack has reached the top of the visible area.
#[test]
fn snapshot_board_view_with_locked_stack() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);

    // Fill the bottom 5 visible rows (board rows 35..40) with locked pieces
    // using alternating kinds for visual variety.
    for row in 35..40usize {
        for col in 0..10usize {
            let kind = if col % 2 == 0 {
                PieceKind::I
            } else {
                PieceKind::O
            };
            state.board.set(col, row, kind);
        }
    }

    // No active piece — game-over state just before the overlay.
    state.active = None;

    let theme = Theme::monochrome();
    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_with_locked_stack", buf_to_string(&terminal));
}

/// Board view during phase 1 (flash) of the line-clear animation.
#[test]
fn snapshot_line_clear_flash_frame() {
    use blocktxt::{LineClearAnim, LineClearPhase};

    let clock = FakeClock::new(Instant::now());
    let mut state = GameState::new(42, Box::new(clock.clone()));

    // Fill all 10 columns of board row 39 (bottom of visible area).
    for col in 0..10usize {
        state.board.set(col, 39, PieceKind::I);
    }

    // Inject a LineClearAnim in Flash phase (t=0, well within 100ms).
    state.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: state.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("line_clear_flash_frame", buf_to_string(&terminal));
}

/// Board view rendered with Catppuccin Mocha palette via explicit Palette arg.
///
/// Pins the non-default path so a regression in palette routing is caught.
#[test]
fn board_view_catppuccin_via_arg() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    // Fill the bottom 3 rows to give something coloured to render.
    for row in 37..40usize {
        for col in 0..10usize {
            state.board.set(col, row, PieceKind::S);
        }
    }

    // Explicitly select Catppuccin Mocha (non-default palette).
    let theme = Theme::truecolor(Palette::CatppuccinMocha);

    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_catppuccin_via_arg", buf_to_string(&terminal));
}

// ── new palette snapshots (#012) ──────────────────────────────────────────────

/// Board view rendered with Gruvbox Dark palette.
#[test]
fn board_view_gruvbox() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in 37..40usize {
        for col in 0..10usize {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::GruvboxDark);

    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_gruvbox", buf_to_string(&terminal));
}

/// Board view rendered with Nord palette.
#[test]
fn board_view_nord() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in 37..40usize {
        for col in 0..10usize {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::Nord);

    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_nord", buf_to_string(&terminal));
}

/// Board view rendered with Dracula palette.
#[test]
fn board_view_dracula() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in 37..40usize {
        for col in 0..10usize {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::Dracula);

    let backend = TestBackend::new(22, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_dracula", buf_to_string(&terminal));
}
