//! Renderer entry point.
//!
//! `render::render(frame, &state, &theme)` is the single public drawing
//! entry point. It splits the terminal area into:
//!
//!   - Playfield: 20 cols wide (10 board cols × 2 char each), 20 rows tall.
//!   - HUD: everything to the right of the playfield.
//!
//! The renderer only reads `&GameState` and never mutates game state.

pub mod board_view;
pub mod helpers;
pub mod hud;
pub mod theme;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::game::state::GameState;
use crate::persistence::HighScoreStore;
pub use theme::Theme;

/// Minimum terminal width required to display the game.
///
/// Per SPEC §3 "SIGWINCH / resize": minimum is 44×24 characters.
pub const MIN_WIDTH: u16 = 44;

/// Minimum terminal height required to display the game.
///
/// Per SPEC §3 "SIGWINCH / resize": minimum is 44×24 characters.
pub const MIN_HEIGHT: u16 = 24;

/// Draw one full frame: board + HUD.
///
/// If the terminal is smaller than `MIN_WIDTH × MIN_HEIGHT`, draws the
/// too-small overlay instead of the game.
pub fn render(frame: &mut Frame, state: &GameState, theme: &Theme) {
    render_with_scores(frame, state, theme, None);
}

/// Like `render`, but routes an optional `HighScoreStore` to the HUD so
/// the new-best banner on the GameOver overlay can light up end-to-end.
///
/// Preferred entry point from `main.rs` once persistence is wired.
pub fn render_with_scores(
    frame: &mut Frame,
    state: &GameState,
    theme: &Theme,
    high_scores: Option<&HighScoreStore>,
) {
    let area = frame.area();

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small_overlay(frame, area);
        return;
    }

    // Split horizontally: 22 cols for the playfield, rest for HUD.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // playfield (20 cols + 2 border chars)
            Constraint::Min(16),    // HUD (min 16 chars)
        ])
        .split(area);

    board_view::draw(frame, chunks[0], state, theme);
    hud::draw_with_scores(frame, chunks[1], state, theme, high_scores);
}

/// Draw a centered "terminal too small" overlay.
///
/// Replaces the entire frame with a plain message so the user knows
/// to resize.  Does not crash or leave corrupted output.
pub fn draw_too_small_overlay(frame: &mut Frame, area: Rect) {
    let msg = format!(
        "Terminal too small\nPlease resize to at least\n{}x{}",
        MIN_WIDTH, MIN_HEIGHT
    );
    let overlay_w = 32u16.min(area.width.max(1));
    let overlay_h = 5u16.min(area.height.max(1));
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Terminal too small",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw(format!(
                "Resize to at least {}x{}",
                MIN_WIDTH, MIN_HEIGHT
            ))),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center),
        overlay_area,
    );
    let _ = msg; // used in the Paragraph above
}
