//! Resize and too-small overlay snapshot tests.
//!
//! Uses ratatui `TestBackend` so no real terminal is needed.
//! Snapshots are managed by `insta`.

use std::time::Instant;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use blocktxt::clock::FakeClock;
use blocktxt::game::state::GameState;
use blocktxt::render::{self, Theme, MIN_HEIGHT, MIN_WIDTH};

fn fake_state() -> GameState {
    let clock = Box::new(FakeClock::new(Instant::now()));
    GameState::new(42, clock)
}

/// Render into a terminal that is smaller than the minimum size and
/// assert the too-small overlay is shown.
#[test]
fn snapshot_too_small_overlay() {
    let state = fake_state();
    let theme = Theme::monochrome();

    // Deliberately tiny — 20×10.
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render::render(f, &state, &theme))
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

    // The overlay text must appear somewhere in the output.
    let flat: String = lines.concat();
    assert!(
        flat.contains("small") || flat.contains("Resize") || flat.contains("resize"),
        "expected too-small message in overlay, got:\n{snapshot}"
    );

    insta::assert_snapshot!("too_small_overlay", snapshot);
}

/// Render at the minimum viable size — game should render without the overlay.
#[test]
fn snapshot_minimum_viable_size() {
    let state = fake_state();
    let theme = Theme::monochrome();

    let backend = TestBackend::new(MIN_WIDTH, MIN_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render::render(f, &state, &theme))
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

    // At minimum size the too-small overlay should NOT appear.
    let flat: String = lines.concat();
    assert!(
        !flat.contains("Terminal too small"),
        "should not show too-small overlay at minimum size:\n{snapshot}"
    );

    insta::assert_snapshot!("minimum_viable_size", snapshot);
}

/// Render at a comfortable normal size — game should render cleanly.
#[test]
fn snapshot_normal_size() {
    let state = fake_state();
    let theme = Theme::monochrome();

    // Normal terminal: 80×24.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render::render(f, &state, &theme))
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

    insta::assert_snapshot!("normal_size", snapshot);
}
