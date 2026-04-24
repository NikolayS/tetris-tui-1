//! Tests for `render::theme` — color/glyph detection.

use blocktxt::render::theme::{
    Palette, Theme, CM_I, CM_J, CM_L, CM_O, CM_S, CM_T, TN_I, TN_J, TN_L, TN_O, TN_S, TN_T, TN_Z,
};
use ratatui::style::Color;
use serial_test::serial;

#[test]
fn theme_monochrome_when_no_color_flag() {
    let theme = Theme::detect(true /* no_color_flag */, Palette::default());
    assert!(
        theme.monochrome,
        "no_color flag should yield monochrome theme"
    );
}

#[test]
#[serial]
fn theme_monochrome_when_no_color_env() {
    // Temporarily set NO_COLOR in the environment.
    // `#[serial]` prevents data races with other tests that touch env vars.
    std::env::set_var("NO_COLOR", "1");
    let theme = Theme::detect(false, Palette::default());
    std::env::remove_var("NO_COLOR");
    assert!(
        theme.monochrome,
        "NO_COLOR env should yield monochrome theme"
    );
}

#[test]
#[serial]
fn theme_uses_256_color_when_colorterm_truecolor() {
    std::env::remove_var("NO_COLOR");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::default());
    std::env::remove_var("COLORTERM");
    assert!(
        !theme.monochrome,
        "truecolor COLORTERM should yield color theme"
    );
}

#[test]
fn theme_glyphs_are_all_distinct_monochrome() {
    let theme = Theme::monochrome();
    let glyphs = &theme.glyphs;
    // All 7 glyphs must be different characters.
    let unique: std::collections::HashSet<char> = glyphs.iter().copied().collect();
    assert_eq!(
        unique.len(),
        7,
        "monochrome glyphs must all be distinct: {:?}",
        glyphs
    );
}

#[test]
#[serial]
fn theme_detect_no_color_env_empty_string_is_not_set() {
    // Per NO_COLOR spec: the variable must be non-empty to activate.
    // Empty string should NOT activate monochrome.
    std::env::set_var("NO_COLOR", "");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::default());
    std::env::remove_var("NO_COLOR");
    std::env::remove_var("COLORTERM");
    assert!(
        !theme.monochrome,
        "empty NO_COLOR should not activate monochrome"
    );
}

// ── new palette tests (#50) ───────────────────────────────────────────────────

#[test]
fn palette_default_is_tokyo_night() {
    assert_eq!(
        Palette::default(),
        Palette::TokyoNight,
        "default palette must be TokyoNight"
    );
}

#[test]
#[serial]
fn theme_tokyo_night_colors_match_spec() {
    std::env::remove_var("NO_COLOR");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::TokyoNight);
    std::env::remove_var("COLORTERM");

    use blocktxt::game::piece::PieceKind;
    assert_eq!(theme.color(PieceKind::I), TN_I, "I piece should be TN cyan");
    assert_eq!(theme.color(PieceKind::O), TN_O, "O piece should be TN gold");
    assert_eq!(theme.color(PieceKind::Z), TN_Z, "Z piece should be TN red");
    // Spot-check background colors.
    assert_eq!(theme.base, Color::Rgb(26, 27, 38), "base should be #1a1b26");
}

#[test]
#[serial]
fn theme_catppuccin_still_works() {
    std::env::remove_var("NO_COLOR");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::CatppuccinMocha);
    std::env::remove_var("COLORTERM");

    use blocktxt::game::piece::PieceKind;
    assert_eq!(theme.color(PieceKind::I), CM_I, "I piece should be CM sky");
    assert_eq!(
        theme.color(PieceKind::O),
        CM_O,
        "O piece should be CM yellow"
    );
    assert_eq!(
        theme.color(PieceKind::T),
        CM_T,
        "T piece should be CM mauve"
    );
}

#[test]
fn cli_theme_flag_parses_valid() {
    for input in &["tokyo-night", "tn", "catppuccin-mocha"] {
        let result: Result<Palette, _> = input.parse();
        assert!(result.is_ok(), "expected Ok for theme '{input}'");
    }
}

#[test]
fn cli_theme_flag_rejects_invalid() {
    let result: Result<Palette, String> = "purple".parse();
    assert!(result.is_err(), "expected Err for unknown theme 'purple'");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("tokyo-night") && msg.contains("catppuccin-mocha"),
        "error message should list valid themes, got: {msg}"
    );
}

#[test]
#[serial]
fn no_color_overrides_palette() {
    // Even with TokyoNight or CatppuccinMocha, NO_COLOR must give monochrome.
    std::env::set_var("NO_COLOR", "1");
    let tn = Theme::detect(false, Palette::TokyoNight);
    let cm = Theme::detect(false, Palette::CatppuccinMocha);
    std::env::remove_var("NO_COLOR");

    assert!(tn.monochrome, "TokyoNight: NO_COLOR must yield monochrome");
    assert!(
        cm.monochrome,
        "CatppuccinMocha: NO_COLOR must yield monochrome"
    );
}

// Keep these imports used (suppress unused-import lint).
const _: Color = TN_J;
const _: Color = TN_L;
const _: Color = TN_S;
const _: Color = TN_T;
const _: Color = CM_J;
const _: Color = CM_L;
const _: Color = CM_S;
