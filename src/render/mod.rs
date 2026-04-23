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

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::game::state::GameState;
pub use theme::Theme;

/// Draw one full frame: board + HUD.
pub fn render(frame: &mut Frame, state: &GameState, theme: &Theme) {
    let area = frame.area();

    // Split horizontally: 20 columns for the playfield, rest for HUD.
    // Playfield: 10 cols × 2 chars/col = 20 chars. Add 2 for border.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // playfield (20 cols + 2 border chars)
            Constraint::Min(16),    // HUD (min 16 chars)
        ])
        .split(area);

    board_view::draw(frame, chunks[0], state, theme);
    hud::draw(frame, chunks[1], state, theme);
}
