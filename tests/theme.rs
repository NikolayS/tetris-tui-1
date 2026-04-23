//! Tests for `render::theme` — color/glyph detection.

use blocktxt::render::theme::Theme;
use serial_test::serial;

#[test]
fn theme_monochrome_when_no_color_flag() {
    let theme = Theme::detect(true /* no_color_flag */);
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
    let theme = Theme::detect(false);
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
    let theme = Theme::detect(false);
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
    let theme = Theme::detect(false);
    std::env::remove_var("NO_COLOR");
    std::env::remove_var("COLORTERM");
    assert!(
        !theme.monochrome,
        "empty NO_COLOR should not activate monochrome"
    );
}
