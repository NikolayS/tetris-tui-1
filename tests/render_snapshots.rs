//! Snapshot tests using ratatui TestBackend + insta.
//!
//! For first run: `INSTA_UPDATE=always cargo test --test render_snapshots`
//! Then review with `cargo insta review` or accept inline.

use std::time::Instant;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use blocktxt::clock::FakeClock;
use blocktxt::game::state::GameState;
use blocktxt::render::{hud, Theme};
use blocktxt::Input;

fn fake_state() -> GameState {
    let clock = Box::new(FakeClock::new(Instant::now()));
    GameState::new(42, clock)
}

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
    let snapshot = lines.join("\n");
    insta::assert_snapshot!("hud_empty_state", snapshot);
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
    let snapshot = lines.join("\n");
    insta::assert_snapshot!("hud_paused", snapshot);
}

/// Render the full board_view on an empty board.
#[test]
fn snapshot_board_view_empty() {
    use blocktxt::render::board_view;

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
    let snapshot = lines.join("\n");
    insta::assert_snapshot!("board_view_empty", snapshot);
}
