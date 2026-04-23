mod cli;
mod signals;
mod terminal;

use std::io;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crossterm::event;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use terminal::{install_panic_hook, TerminalGuard};

use blocktxt::clock::RealClock;
use blocktxt::game::state::GameState;
use blocktxt::render::{self, Theme};
use blocktxt::{Event as GameEvent, Input, Phase};

fn main() -> anyhow::Result<()> {
    // Step 1: Parse CLI args (clap derive).
    let args = cli::parse();

    // Step 2: Handle --reset-scores in cooked mode before any raw mode.
    if args.reset_scores {
        cli::handle_reset_scores(&args)?;
        return Ok(());
    }

    // Step 3: Install signal flags BEFORE guard enter so Ctrl-C works if
    //         guard setup fails partway through.
    // Step 4: Install the panic hook BEFORE guard setup so the terminal is
    //         restored even if setup panics.
    install_panic_hook();

    #[cfg(unix)]
    let flags = signals::unix::install()?;

    // Step 5: Enter TUI (raw mode + alternate screen + hide cursor).
    let mut guard = TerminalGuard::enter()?;

    // Step 6: --crash-for-test panics after guard entry (used by PTY tests).
    if args.crash_for_test {
        panic!("--crash-for-test: intentional panic after guard entry");
    }

    // Step 7: Run game loop.
    run_loop(&mut guard, &flags, &args)?;

    Ok(())
}

/// Translate a crossterm KeyEvent into zero or more game inputs.
fn translate_key(key: crossterm::event::KeyEvent) -> Vec<Input> {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut inputs = Vec::new();
    // Ctrl-C / Ctrl-D → quit (handled separately via return value).
    match key.code {
        // Movement
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => {
            inputs.push(Input::MoveLeft);
        }
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => {
            inputs.push(Input::MoveRight);
        }
        // Soft drop (press)
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') => {
            inputs.push(Input::SoftDropOn);
        }
        // Hard drop
        KeyCode::Char(' ') => {
            inputs.push(Input::HardDrop);
        }
        // Rotations
        KeyCode::Char('z') => inputs.push(Input::RotateCcw),
        KeyCode::Char('x') => inputs.push(Input::RotateCw),
        // Pause
        KeyCode::Char('p') => inputs.push(Input::Pause),
        // Restart (game-over screen)
        KeyCode::Char('r') => inputs.push(Input::Restart),
        _ => {}
    }
    // Ctrl-C / Ctrl-D — surfaced as `quit` signal, no Input needed.
    let _ = KeyModifiers::CONTROL;
    inputs
}

/// Returns true if the key event signals a quit request.
fn is_quit(key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};
    matches!(key.code, KeyCode::Char('q'))
        || (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
}

/// Returns true if the key event signals soft-drop release.
fn is_soft_drop_release(key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::KeyCode;
    matches!(
        key.code,
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j')
    )
}

#[cfg(unix)]
fn run_loop(
    guard: &mut TerminalGuard,
    flags: &signals::unix::Flags,
    args: &cli::Args,
) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};

    // Build ratatui terminal on top of the existing crossterm raw-mode/alt-screen
    // that TerminalGuard already set up.
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Game state + theme.
    let seed = args.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(42)
    });
    let clock = Box::new(RealClock);
    let mut state = GameState::new(seed, clock);
    let theme = Theme::detect(args.no_color);

    // Frame cadence: 16 ms ceiling (≈ 60 fps).
    const FRAME_DT: Duration = Duration::from_millis(16);
    // Event poll: 8 ms per SPEC §4.
    const POLL_DT: Duration = Duration::from_millis(8);

    // Track soft-drop key state (pressed → SoftDropOn; released → SoftDropOff).
    let mut soft_drop_held = false;

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
        let mut inputs: Vec<Input> = Vec::new();

        // Poll for events (8 ms timeout).
        if event::poll(POLL_DT)? {
            let ev = event::read()?;
            if let event::Event::Key(key) = ev {
                use crossterm::event::KeyEventKind;
                // crossterm 0.28+ emits Press + Release; earlier only Press.
                // Treat both Press and Repeat as press; Release as release.
                match key.kind {
                    KeyEventKind::Release => {
                        if is_soft_drop_release(key) && soft_drop_held {
                            soft_drop_held = false;
                            inputs.push(Input::SoftDropOff);
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat => {
                        if is_quit(key) {
                            break;
                        }
                        // Handle soft-drop separately to track hold state.
                        let new_inputs = translate_key(key);
                        for inp in &new_inputs {
                            if *inp == Input::SoftDropOn && !soft_drop_held {
                                soft_drop_held = true;
                            }
                        }
                        inputs.extend(new_inputs);
                    }
                }
            }
        }

        // --- 3. Step game ---
        let now = Instant::now();
        let dt = now.duration_since(last_frame).min(FRAME_DT * 2);
        let events = state.step(dt, &inputs);
        last_frame = now;

        // Handle emitted events.
        for ev in events {
            if let GameEvent::GameOver(_) = ev {
                // Stay in the loop; overlay is shown via phase check in render.
            }
        }

        // If in game-over and player pressed quit, break.
        if matches!(state.phase, Phase::GameOver { .. }) {
            // Check if 'q' was in inputs.
            for inp in &inputs {
                if *inp == Input::Restart {
                    // Restart handled by step().
                    break;
                }
            }
        }

        // --- 4. Draw ---
        terminal.draw(|f| render::render(f, &state, &theme))?;

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
fn run_loop(_guard: &mut TerminalGuard, _flags: &(), _args: &cli::Args) -> anyhow::Result<()> {
    Ok(())
}
