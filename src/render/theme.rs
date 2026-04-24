//! Color and glyph theme for the renderer.
//!
//! `Theme::detect()` reads the environment to select the best available
//! output mode:
//!
//!   1. If `NO_COLOR` is set (non-empty) OR `--no-color` was passed, use
//!      monochrome ASCII glyphs — one distinct letter per piece kind.
//!   2. Else if `COLORTERM=truecolor`, use full RGB colors + block glyph.
//!   3. Else use 256-color palette + distinctive glyphs as a safe fallback.
//!
//! Index into `colors` and `glyphs` arrays using `PieceKind as usize`:
//!   I=0, O=1, T=2, S=3, Z=4, J=5, L=6

use ratatui::style::Color;

use crate::game::piece::PieceKind;

// ── Catppuccin Mocha palette ──────────────────────────────────────────────────
// https://github.com/catppuccin/catppuccin

/// Background — 1e1e2e
pub const CM_BASE: Color = Color::Rgb(30, 30, 46);
/// Secondary background — 181825
pub const CM_MANTLE: Color = Color::Rgb(24, 24, 37);
/// Darkest background — 11111b
pub const CM_CRUST: Color = Color::Rgb(17, 17, 27);
/// Primary text — cdd6f4
pub const CM_TEXT: Color = Color::Rgb(205, 214, 244);
/// Secondary text — a6adc8
pub const CM_SUBTEXT: Color = Color::Rgb(166, 173, 200);
/// Dim / border color — 6c7086
pub const CM_OVERLAY: Color = Color::Rgb(108, 112, 134);

/// I piece — sky
pub const CM_I: Color = Color::Rgb(137, 220, 235);
/// O piece — yellow
pub const CM_O: Color = Color::Rgb(249, 226, 175);
/// T piece — mauve
pub const CM_T: Color = Color::Rgb(203, 166, 247);
/// S piece — green
pub const CM_S: Color = Color::Rgb(166, 227, 161);
/// Z piece — pink
pub const CM_Z: Color = Color::Rgb(243, 139, 168);
/// J piece — blue
pub const CM_J: Color = Color::Rgb(137, 180, 250);
/// L piece — peach
pub const CM_L: Color = Color::Rgb(250, 179, 135);

/// Ghost piece fill — surface1
pub const CM_GHOST: Color = Color::Rgb(69, 71, 90);
/// "NEW BEST!" highlight — yellow accent
pub const CM_NEW_BEST: Color = Color::Rgb(249, 226, 175);

// ── Tokyo Night palette ───────────────────────────────────────────────────────
// https://github.com/tokyo-night/tokyo-night-vscode-theme

/// Background — #1a1b26
pub const TN_BASE: Color = Color::Rgb(26, 27, 38);
/// Secondary background — #16161e
pub const TN_MANTLE: Color = Color::Rgb(22, 22, 30);
/// Darkest background — #0f0f14
pub const TN_CRUST: Color = Color::Rgb(15, 15, 20);
/// Primary text — #c0caf5
pub const TN_TEXT: Color = Color::Rgb(192, 202, 245);
/// Secondary text — #a9b1d6
pub const TN_SUBTEXT: Color = Color::Rgb(169, 177, 214);
/// Dim / border color — #565f89
pub const TN_OVERLAY: Color = Color::Rgb(86, 95, 137);

/// I piece — cyan #7dcfff
pub const TN_I: Color = Color::Rgb(125, 207, 255);
/// O piece — gold #e0af68
pub const TN_O: Color = Color::Rgb(224, 175, 104);
/// T piece — purple #bb9af7
pub const TN_T: Color = Color::Rgb(187, 154, 247);
/// S piece — lime #9ece6a
pub const TN_S: Color = Color::Rgb(158, 206, 106);
/// Z piece — red #f7768e
pub const TN_Z: Color = Color::Rgb(247, 118, 142);
/// J piece — blue #7aa2f7
pub const TN_J: Color = Color::Rgb(122, 162, 247);
/// L piece — orange #ff9e64
pub const TN_L: Color = Color::Rgb(255, 158, 100);

/// Ghost piece fill — #414868
pub const TN_GHOST: Color = Color::Rgb(65, 72, 104);
/// "NEW BEST!" highlight — gold accent
pub const TN_NEW_BEST: Color = Color::Rgb(224, 175, 104);

// ── Legacy public aliases (Catppuccin Mocha, kept for existing render code) ──

/// Background (Catppuccin Mocha alias).
pub const BASE: Color = CM_BASE;
/// Secondary background (Catppuccin Mocha alias).
pub const MANTLE: Color = CM_MANTLE;
/// Darkest background (Catppuccin Mocha alias).
pub const CRUST: Color = CM_CRUST;
/// Primary text (Catppuccin Mocha alias).
pub const TEXT: Color = CM_TEXT;
/// Secondary text (Catppuccin Mocha alias).
pub const SUBTEXT: Color = CM_SUBTEXT;
/// Dim / border color (Catppuccin Mocha alias).
pub const OVERLAY: Color = CM_OVERLAY;

/// I piece color (Catppuccin Mocha alias).
pub const I_COLOR: Color = CM_I;
/// O piece color (Catppuccin Mocha alias).
pub const O_COLOR: Color = CM_O;
/// T piece color (Catppuccin Mocha alias).
pub const T_COLOR: Color = CM_T;
/// S piece color (Catppuccin Mocha alias).
pub const S_COLOR: Color = CM_S;
/// Z piece color (Catppuccin Mocha alias).
pub const Z_COLOR: Color = CM_Z;
/// J piece color (Catppuccin Mocha alias).
pub const J_COLOR: Color = CM_J;
/// L piece color (Catppuccin Mocha alias).
pub const L_COLOR: Color = CM_L;

/// Ghost piece fill (Catppuccin Mocha alias).
pub const GHOST_MOD: Color = CM_GHOST;
/// "NEW BEST!" highlight (Catppuccin Mocha alias).
pub const NEW_BEST: Color = CM_NEW_BEST;

// ── Gruvbox Dark palette ──────────────────────────────────────────────────────
// https://github.com/morhetz/gruvbox

/// Background — #282828
pub const GV_BASE: Color = Color::Rgb(40, 40, 40);
/// Secondary background — #1d2021
pub const GV_MANTLE: Color = Color::Rgb(29, 32, 33);
/// Darkest background — #16181a
pub const GV_CRUST: Color = Color::Rgb(22, 24, 26);
/// Primary text — #ebdbb2 fg1
pub const GV_TEXT: Color = Color::Rgb(235, 219, 178);
/// Secondary text — #bdae93 fg3
pub const GV_SUBTEXT: Color = Color::Rgb(189, 174, 147);
/// Dim / border color — #7c6f64 bg4
pub const GV_OVERLAY: Color = Color::Rgb(124, 111, 100);

/// I piece — #83a598 aqua
pub const GV_I: Color = Color::Rgb(131, 165, 152);
/// O piece — #fabd2f yellow
pub const GV_O: Color = Color::Rgb(250, 189, 47);
/// T piece — #d3869b purple
pub const GV_T: Color = Color::Rgb(211, 134, 155);
/// S piece — #b8bb26 green
pub const GV_S: Color = Color::Rgb(184, 187, 38);
/// Z piece — #fb4934 red
pub const GV_Z: Color = Color::Rgb(251, 73, 52);
/// J piece — #458588 blue
pub const GV_J: Color = Color::Rgb(69, 133, 136);
/// L piece — #fe8019 orange
pub const GV_L: Color = Color::Rgb(254, 128, 25);

/// Ghost piece fill — #504945
pub const GV_GHOST: Color = Color::Rgb(80, 73, 69);
/// "NEW BEST!" highlight — yellow accent
pub const GV_NEW_BEST: Color = GV_O;

// ── Nord palette ──────────────────────────────────────────────────────────────
// https://www.nordtheme.com/

/// Background — #2e3440 nord0
pub const NO_BASE: Color = Color::Rgb(46, 52, 64);
/// Secondary background — #252a34
pub const NO_MANTLE: Color = Color::Rgb(37, 42, 52);
/// Darkest background — #1d2129
pub const NO_CRUST: Color = Color::Rgb(29, 33, 41);
/// Primary text — #d8dee9 nord4
pub const NO_TEXT: Color = Color::Rgb(216, 222, 233);
/// Secondary text — #b4bcce
pub const NO_SUBTEXT: Color = Color::Rgb(180, 188, 206);
/// Dim / border color — #4c566a nord3
pub const NO_OVERLAY: Color = Color::Rgb(76, 86, 106);

/// I piece — #88c0d0 nord8
pub const NO_I: Color = Color::Rgb(136, 192, 208);
/// O piece — #ebcb8b nord13
pub const NO_O: Color = Color::Rgb(235, 203, 139);
/// T piece — #b48ead nord15
pub const NO_T: Color = Color::Rgb(180, 142, 173);
/// S piece — #a3be8c nord14
pub const NO_S: Color = Color::Rgb(163, 190, 140);
/// Z piece — #bf616a nord11
pub const NO_Z: Color = Color::Rgb(191, 97, 106);
/// J piece — #81a1c1 nord9
pub const NO_J: Color = Color::Rgb(129, 161, 193);
/// L piece — #d08770 nord12
pub const NO_L: Color = Color::Rgb(208, 135, 112);

/// Ghost piece fill — #3b4252 nord1
pub const NO_GHOST: Color = Color::Rgb(59, 66, 82);
/// "NEW BEST!" highlight — yellow accent
pub const NO_NEW_BEST: Color = NO_O;

// ── Dracula palette ───────────────────────────────────────────────────────────
// https://draculatheme.com/

/// Background — #282a36
pub const DR_BASE: Color = Color::Rgb(40, 42, 54);
/// Secondary background — #22242e
pub const DR_MANTLE: Color = Color::Rgb(34, 36, 46);
/// Darkest background — #1a1c24
pub const DR_CRUST: Color = Color::Rgb(26, 28, 36);
/// Primary text — #f8f8f2 foreground
pub const DR_TEXT: Color = Color::Rgb(248, 248, 242);
/// Secondary text — #d0d2e0
pub const DR_SUBTEXT: Color = Color::Rgb(208, 210, 224);
/// Dim / border color — #6272a4 comment
pub const DR_OVERLAY: Color = Color::Rgb(98, 114, 164);

/// I piece — #8be9fd cyan
pub const DR_I: Color = Color::Rgb(139, 233, 253);
/// O piece — #f1fa8c yellow
pub const DR_O: Color = Color::Rgb(241, 250, 140);
/// T piece — #bd93f9 purple
pub const DR_T: Color = Color::Rgb(189, 147, 249);
/// S piece — #50fa7b green
pub const DR_S: Color = Color::Rgb(80, 250, 123);
/// Z piece — #ff5555 red
pub const DR_Z: Color = Color::Rgb(255, 85, 85);
/// J piece — #6272a4 soft blue
pub const DR_J: Color = Color::Rgb(98, 114, 164);
/// L piece — #ffb86c orange
pub const DR_L: Color = Color::Rgb(255, 184, 108);

/// Ghost piece fill — #44475a current-line
pub const DR_GHOST: Color = Color::Rgb(68, 71, 90);
/// "NEW BEST!" highlight — yellow accent
pub const DR_NEW_BEST: Color = DR_O;

// ── Palette enum ──────────────────────────────────────────────────────────────

/// Named color palettes available via `--theme`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Palette {
    /// Tokyo Night — higher saturation + brighter text (default).
    #[default]
    TokyoNight,
    /// Catppuccin Mocha — softer, muted tones.
    CatppuccinMocha,
    /// Gruvbox Dark — warm, earthy tones.
    GruvboxDark,
    /// Nord — cool arctic blues.
    Nord,
    /// Dracula — vibrant purple-based dark theme.
    Dracula,
}

impl std::str::FromStr for Palette {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tokyo-night" | "tokyo_night" | "tn" => Ok(Self::TokyoNight),
            "catppuccin-mocha" | "catppuccin" | "cm" => Ok(Self::CatppuccinMocha),
            "gruvbox-dark" | "gruvbox_dark" | "gruvbox" | "gv" => Ok(Self::GruvboxDark),
            "nord" | "nord-dark" => Ok(Self::Nord),
            "dracula" | "dr" => Ok(Self::Dracula),
            other => Err(format!(
                "unknown theme '{}'. valid: \
                 tokyo-night, catppuccin-mocha, gruvbox-dark, nord, dracula",
                other
            )),
        }
    }
}

// ── Theme struct ──────────────────────────────────────────────────────────────

/// Rendering theme: one color and one glyph per piece kind.
#[derive(Debug, Clone)]
pub struct Theme {
    /// ANSI colors indexed by `PieceKind as usize`.
    pub colors: [Color; 7],
    /// Glyphs indexed by `PieceKind as usize`.
    pub glyphs: [char; 7],
    /// True when no color should be applied (monochrome mode).
    pub monochrome: bool,
    /// Background fill color.
    pub base: Color,
    /// Secondary / inner background color.
    pub mantle: Color,
    /// Dim / border color.
    pub overlay: Color,
    /// Primary text color.
    pub text: Color,
    /// Secondary / label text color.
    pub subtext: Color,
    /// Ghost piece color.
    pub ghost: Color,
    /// "NEW BEST!" highlight color.
    pub new_best: Color,
}

impl Theme {
    /// Detect the best available color mode and return an appropriate theme.
    ///
    /// `no_color_flag` should be `true` when the user passed `--no-color`.
    /// `palette` selects which named palette to use in color modes.
    pub fn detect(no_color_flag: bool, palette: Palette) -> Self {
        let no_color_env = std::env::var("NO_COLOR")
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        if no_color_flag || no_color_env {
            return Self::monochrome();
        }

        // Check for truecolor support.
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if colorterm == "truecolor" || colorterm == "24bit" {
            return Self::truecolor(palette);
        }

        // Default: 256-color palette with distinctive glyphs.
        Self::color256(palette)
    }

    /// Monochrome theme: distinct ASCII letters, no color attributes.
    ///
    /// Letters match piece names so they're meaningful without color:
    ///   I, O, T, S, Z, J, L — same as the piece names in the spec.
    pub fn monochrome() -> Self {
        Self {
            colors: [Color::Reset; 7],
            // One unique letter per piece kind (I, O, T, S, Z, J, L).
            glyphs: ['I', 'O', 'T', 'S', 'Z', 'J', 'L'],
            monochrome: true,
            // Monochrome UI uses Tokyo Night backgrounds (no color shown).
            base: TN_BASE,
            mantle: TN_MANTLE,
            overlay: TN_OVERLAY,
            text: TN_TEXT,
            subtext: TN_SUBTEXT,
            ghost: TN_GHOST,
            new_best: TN_NEW_BEST,
        }
    }

    /// Full RGB truecolor theme using the given palette.
    pub fn truecolor(palette: Palette) -> Self {
        match palette {
            Palette::TokyoNight => Self {
                colors: [
                    TN_I, // I — cyan
                    TN_O, // O — gold
                    TN_T, // T — purple
                    TN_S, // S — lime
                    TN_Z, // Z — red
                    TN_J, // J — blue
                    TN_L, // L — orange
                ],
                glyphs: ['█', '█', '█', '█', '█', '█', '█'],
                monochrome: false,
                base: TN_BASE,
                mantle: TN_MANTLE,
                overlay: TN_OVERLAY,
                text: TN_TEXT,
                subtext: TN_SUBTEXT,
                ghost: TN_GHOST,
                new_best: TN_NEW_BEST,
            },
            Palette::CatppuccinMocha => Self {
                colors: [
                    CM_I, // I — sky
                    CM_O, // O — yellow
                    CM_T, // T — mauve
                    CM_S, // S — green
                    CM_Z, // Z — pink
                    CM_J, // J — blue
                    CM_L, // L — peach
                ],
                glyphs: ['█', '█', '█', '█', '█', '█', '█'],
                monochrome: false,
                base: CM_BASE,
                mantle: CM_MANTLE,
                overlay: CM_OVERLAY,
                text: CM_TEXT,
                subtext: CM_SUBTEXT,
                ghost: CM_GHOST,
                new_best: CM_NEW_BEST,
            },
            Palette::GruvboxDark => Self {
                colors: [
                    GV_I, // I — aqua
                    GV_O, // O — yellow
                    GV_T, // T — purple
                    GV_S, // S — green
                    GV_Z, // Z — red
                    GV_J, // J — blue
                    GV_L, // L — orange
                ],
                glyphs: ['█', '█', '█', '█', '█', '█', '█'],
                monochrome: false,
                base: GV_BASE,
                mantle: GV_MANTLE,
                overlay: GV_OVERLAY,
                text: GV_TEXT,
                subtext: GV_SUBTEXT,
                ghost: GV_GHOST,
                new_best: GV_NEW_BEST,
            },
            Palette::Nord => Self {
                colors: [
                    NO_I, // I — nord8 cyan
                    NO_O, // O — nord13 yellow
                    NO_T, // T — nord15 purple
                    NO_S, // S — nord14 green
                    NO_Z, // Z — nord11 red
                    NO_J, // J — nord9 blue
                    NO_L, // L — nord12 orange
                ],
                glyphs: ['█', '█', '█', '█', '█', '█', '█'],
                monochrome: false,
                base: NO_BASE,
                mantle: NO_MANTLE,
                overlay: NO_OVERLAY,
                text: NO_TEXT,
                subtext: NO_SUBTEXT,
                ghost: NO_GHOST,
                new_best: NO_NEW_BEST,
            },
            Palette::Dracula => Self {
                colors: [
                    DR_I, // I — cyan
                    DR_O, // O — yellow
                    DR_T, // T — purple
                    DR_S, // S — green
                    DR_Z, // Z — red
                    DR_J, // J — soft blue
                    DR_L, // L — orange
                ],
                glyphs: ['█', '█', '█', '█', '█', '█', '█'],
                monochrome: false,
                base: DR_BASE,
                mantle: DR_MANTLE,
                overlay: DR_OVERLAY,
                text: DR_TEXT,
                subtext: DR_SUBTEXT,
                ghost: DR_GHOST,
                new_best: DR_NEW_BEST,
            },
        }
    }

    /// 256-color palette theme with distinctive single-char glyphs.
    ///
    /// Palette parameter is accepted for API consistency but the 16-color
    /// ANSI set is palette-agnostic by nature.
    pub fn color256(_palette: Palette) -> Self {
        Self {
            colors: [
                Color::Cyan,    // I
                Color::Yellow,  // O
                Color::Magenta, // T
                Color::Green,   // S
                Color::Red,     // Z
                Color::Blue,    // J
                Color::White,   // L (orange unavailable in 16-color)
            ],
            // Visually distinct glyphs for accessibility and monochrome
            // terminals that claim 256-color support.
            glyphs: ['▓', '▒', '░', '■', '▪', '▫', '▬'],
            monochrome: false,
            base: TN_BASE,
            mantle: TN_MANTLE,
            overlay: TN_OVERLAY,
            text: TN_TEXT,
            subtext: TN_SUBTEXT,
            ghost: TN_GHOST,
            new_best: TN_NEW_BEST,
        }
    }

    /// Return the color for a given piece kind.
    #[inline]
    pub fn color(&self, kind: PieceKind) -> Color {
        self.colors[kind as usize]
    }

    /// Return the glyph for a given piece kind.
    #[inline]
    pub fn glyph(&self, kind: PieceKind) -> char {
        self.glyphs[kind as usize]
    }
}
