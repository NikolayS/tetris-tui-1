mod cli;
mod signals;
mod terminal;

use std::io;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use terminal::{install_panic_hook, TerminalGuard};

use blocktxt::clock::RealClock;
use blocktxt::game::state::GameState;
use blocktxt::input::{InputTranslator, KittySupport};
use blocktxt::persistence::{self, HighScore, HighScoreStore};
use blocktxt::render::theme::Palette;
use blocktxt::render::{self, Theme};
use blocktxt::{Event as GameEvent, Input};

fn main() -> anyhow::Result<()> {
    // Step 1: Parse CLI args (clap derive).
    let args = cli::parse();

    // Step 2: Handle --reset-scores in cooked mode before any raw mode.
    if args.reset_scores {
        cli::handle_reset_scores(&args)?;
        return Ok(());
    }

    // Parse --theme early so invalid values exit before entering raw mode.
    let palette: Palette = match args.theme.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("blocktxt: {e}");
            std::process::exit(2);
        }
    };

    // Step 3: Load persistence BEFORE entering raw mode so any warning goes
    //         to stderr while the terminal is still in cooked mode.
    let persist_dir_result = persistence::init_data_dir();
    // Keep a copy of the path (if Ok) for use in the game loop.
    let persist_dir_path = persist_dir_result.as_ref().ok().cloned();
    let (mut store, persist_err) = HighScoreStore::new_with_fallback(persist_dir_result);

    if let Some(ref err) = persist_err {
        eprintln!(
            "blocktxt: warning: persistence unavailable ({err}); \
             scores will not be saved this session."
        );
    }

    // Step 4: Install signal flags BEFORE guard enter so Ctrl-C works if
    //         guard setup fails partway through.
    // Step 5: Install the panic hook BEFORE guard setup so the terminal is
    //         restored even if setup panics.
    install_panic_hook();

    #[cfg(unix)]
    let flags = signals::unix::install()?;

    // Step 6: Probe kitty protocol (quick 50 ms probe before entering raw mode).
    //
    // Risk note (P-4): if the process panics between writing the probe query
    // bytes (`CSI > 1u`) and a terminal that does not understand them, some
    // stray characters could in principle remain in the terminal.  In
    // practice the panic hook installed above calls `restore_raw()` which
    // writes a known ANSI reset sequence to fd 2, so the terminal ends up
    // cleanly restored regardless of probe response state.
    let kitty = InputTranslator::probe_kitty(Duration::from_millis(50));

    // Step 7: Enter TUI (raw mode + alternate screen + hide cursor).
    let mut guard = TerminalGuard::enter()?;

    // Step 8: --crash-for-test panics after guard entry (used by PTY tests).
    if args.crash_for_test {
        panic!("--crash-for-test: intentional panic after guard entry");
    }

    // Step 9: Run game loop.
    run_loop(
        &mut guard,
        &flags,
        &args,
        &mut store,
        persist_dir_path.as_deref(),
        kitty,
        palette,
    )?;

    Ok(())
}

#[cfg(unix)]
fn run_loop(
    guard: &mut TerminalGuard,
    flags: &signals::unix::Flags,
    args: &cli::Args,
    store: &mut HighScoreStore,
    persist_dir: Option<&std::path::Path>,
    kitty: KittySupport,
    palette: Palette,
) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};

    // Build ratatui terminal on top of the existing crossterm raw-mode/alt-screen
    // that TerminalGuard already set up.
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Game state + theme.
    let seed = args.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(42)
    });
    let clock = Box::new(RealClock);
    let mut state = GameState::new(seed, clock);
    let theme = Theme::detect(args.no_color, palette);

    // DAS/ARR input translator.
    let mut translator = InputTranslator::new(kitty);

    // Frame cadence: 16 ms ceiling (≈ 60 fps).
    const FRAME_DT: Duration = Duration::from_millis(16);
    // Event poll: 8 ms per SPEC §4.
    const POLL_DT: Duration = Duration::from_millis(8);

    let mut last_frame = Instant::now();

    loop {
        // --- 1. Signal flags (ordered per SPEC §3) ---

        if flags.shutdown.swap(false, Ordering::Relaxed) {
            break;
        }

        if flags.tstp_pending.swap(false, Ordering::Relaxed) {
            guard.restore();
            let _ = signal::raise(Signal::SIGSTOP);
            guard.re_enter()?;
            flags.cont_pending.store(false, Ordering::Relaxed);
            flags.winch_pending.store(true, Ordering::Relaxed);
            terminal.clear()?;
            last_frame = Instant::now();
            continue;
        }

        if flags.cont_pending.swap(false, Ordering::Relaxed) {
            guard.re_enter()?;
            flags.winch_pending.store(true, Ordering::Relaxed);
        }

        if flags.winch_pending.swap(false, Ordering::Relaxed) {
            terminal.autoresize()?;
        }

        // --- 2. Collect inputs ---
        let now = Instant::now();
        let mut inputs: Vec<Input> = Vec::new();
        let mut quit_requested = false;

        // Poll for events (8 ms timeout).
        if event::poll(POLL_DT)? {
            let ev = event::read()?;
            let (evs, quit) = translator.translate_event(&ev, now);
            inputs.extend(evs);
            if quit {
                quit_requested = true;
            }
        }

        // Emit any DAS/ARR ticks.
        translator.tick(now, &mut inputs);

        if quit_requested {
            break;
        }

        // --- 3. Step game ---
        let dt = now.duration_since(last_frame).min(FRAME_DT * 2);
        let game_events = state.step(dt, &inputs);
        last_frame = now;

        // Handle emitted events — save score on game-over.
        for ev in game_events {
            if let GameEvent::GameOver(_) = ev {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let hs = HighScore {
                    name: "player".to_owned(),
                    score: state.score,
                    level: state.level,
                    lines: state.lines_cleared,
                    ts,
                };
                let new_best = store.insert(hs);
                if new_best {
                    eprintln!(
                        "blocktxt: new personal best: {} points at level {}.",
                        state.score, state.level
                    );
                }
                if let Some(dir) = persist_dir {
                    if let Err(e) = persistence::save(store, dir) {
                        eprintln!("blocktxt: warning: could not save score: {e}");
                    }
                }
            }
        }

        // --- 4. Draw ---
        // Pass the high-score store to the renderer so the GameOver overlay
        // can light up the NEW BEST banner from PR #39.
        let store_ref: &HighScoreStore = store;
        terminal.draw(|f| render::render_with_scores(f, &state, &theme, Some(store_ref)))?;

        // --- 5. Sleep remainder of frame budget ---
        let elapsed = last_frame.elapsed();
        if elapsed < FRAME_DT {
            std::thread::sleep(FRAME_DT - elapsed);
        }
    }

    Ok(())
}

// Non-unix stub so the crate still compiles on Windows (WSL takes the unix path).
#[cfg(not(unix))]
fn run_loop(
    _guard: &mut TerminalGuard,
    _flags: &(),
    _args: &cli::Args,
    _store: &mut HighScoreStore,
    _persist_dir: Option<&std::path::Path>,
    _kitty: KittySupport,
    _palette: Palette,
) -> anyhow::Result<()> {
    Ok(())
}
