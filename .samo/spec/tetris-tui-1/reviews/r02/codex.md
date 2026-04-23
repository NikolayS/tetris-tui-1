# Reviewer A — Codex

## summary

The spec is much stronger than a typical game spec, but the biggest remaining risks are around local filesystem trust boundaries, unsafe terminal/signal recovery assumptions, legal release exposure, and a still-missing architecture contract. Some v0.1 scope, especially TOML config and highly specific SSH byte budgets, adds support surface before the core game needs it.

## missing-risk

- (major) Persistent state hardening is incomplete: the spec sets 0600 on the score file but does not require creating or validating the data/config directories with private permissions, ownership checks, or symlink-safe parent handling. If the parent directory is world-writable, symlinked, or attacker-controlled, the atomic-file semantics do not protect against path-swap and denial-of-service cases. Define mkdir behavior, expected mode such as 0700 on Unix, owner checks where feasible, and fail-closed behavior for unsafe parents.
- (major) The spec has no explicit legal/release risk treatment for shipping a product named and described as a Tetris clone with Guideline-inspired mechanics, colors, scoring, and SRS references. This is a real distribution risk, especially once release artifacts are published. Add a trademark/IP section: naming, README wording, asset/color choices, and whether published SRS tables are referenced or independently encoded/tested.
- (minor) Config parsing errors are logged and defaults are used, but score-file corruption triggers a destructive recovery path that overwrites the previous .bak. This can destroy forensic/debug information and repeatedly discard user data if a parser bug is introduced. Prefer timestamped backups or only overwrite .bak after a successful rename/write sequence is confirmed.

## weak-implementation

- (major) The signal/terminal lifecycle design risks doing non-async-signal-safe work from signal handlers. Restoring terminal modes, re-entering alternate screen, redrawing, flushing, or re-raising SIGTSTP from an actual handler is fragile and can deadlock or corrupt terminal state. Specify a flag/self-pipe based design where the main loop or a dedicated safe signal thread performs all terminal I/O, and keep raw handlers limited to async-signal-safe operations.
- (major) Architecture is still marked '(architecture not yet specified)' while later sections rely on a nailed-down pure-core/renderer interface and parallel ownership. This is a planning contradiction and weakens the implementation boundary most important for testing, lifecycle safety, and team split. Replace the placeholder with the actual component diagram and contracts before implementation starts.
- (major) Input handling is underspecified for terminal reality. DAS/ARR requires reliable held-key state, but many terminals/crossterm paths differ on key press, repeat, and release events. The spec does not define how held left/right are detected, how key-repeat events are normalized, or what happens in terminals that do not emit release events. This can make controls inconsistent across the exact platforms the spec targets.
- (minor) The persistence path depends on ProjectDirs but does not define behavior when directory discovery returns None, directory creation fails, the path is not writable, or storage is read-only. A game should remain playable even when high-score persistence fails. Specify graceful degradation: warn outside raw mode when possible, continue with in-memory scores, and never panic during startup/title rendering.
- (minor) The SSH byte-budget tests are likely too implementation-coupled to ratatui/crossterm internals and may not predict real PTY behavior. Capturing writes into a Vec does not model terminal emulator parsing, network buffering, backpressure, or line discipline. Keep the budget as a perf target, but add an integration-style PTY test or relax exact byte ceilings to avoid brittle CI failures.
- (minor) The spec claims 'pinned' dependencies but uses broad Cargo.toml requirements such as serde = '1' and anyhow = '1'. Cargo.lock makes builds reproducible for the app, but the wording is misleading and does not cover advisory response or intentional update cadence. Say 'locked via Cargo.lock' and define how dependency updates and cargo deny advisories are handled.

## unnecessary-scope

- (minor) Optional TOML config in v0.1 adds filesystem parsing, validation, error handling, and support burden without being required by the core user stories. It also expands the attack and failure surface for path/device/symlink/oversized-file cases. Consider cutting config.toml from v0.1 and keeping only CLI/env controls needed for accessibility and deterministic tests.

## suggested-next-version

v0.3 should close the ops gaps: define safe directory creation/validation and persistence degradation, rewrite signal handling around main-loop-safe terminal I/O, replace the architecture placeholder with real contracts, specify terminal key-repeat semantics, add legal/release guidance, and cut or defer optional config unless it is required for launch.

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "missing-risk",
      "text": "Persistent state hardening is incomplete: the spec sets 0600 on the score file but does not require creating or validating the data/config directories with private permissions, ownership checks, or symlink-safe parent handling. If the parent directory is world-writable, symlinked, or attacker-controlled, the atomic-file semantics do not protect against path-swap and denial-of-service cases. Define mkdir behavior, expected mode such as 0700 on Unix, owner checks where feasible, and fail-closed behavior for unsafe parents.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "The signal/terminal lifecycle design risks doing non-async-signal-safe work from signal handlers. Restoring terminal modes, re-entering alternate screen, redrawing, flushing, or re-raising SIGTSTP from an actual handler is fragile and can deadlock or corrupt terminal state. Specify a flag/self-pipe based design where the main loop or a dedicated safe signal thread performs all terminal I/O, and keep raw handlers limited to async-signal-safe operations.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "The spec has no explicit legal/release risk treatment for shipping a product named and described as a Tetris clone with Guideline-inspired mechanics, colors, scoring, and SRS references. This is a real distribution risk, especially once release artifacts are published. Add a trademark/IP section: naming, README wording, asset/color choices, and whether published SRS tables are referenced or independently encoded/tested.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Architecture is still marked '(architecture not yet specified)' while later sections rely on a nailed-down pure-core/renderer interface and parallel ownership. This is a planning contradiction and weakens the implementation boundary most important for testing, lifecycle safety, and team split. Replace the placeholder with the actual component diagram and contracts before implementation starts.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Input handling is underspecified for terminal reality. DAS/ARR requires reliable held-key state, but many terminals/crossterm paths differ on key press, repeat, and release events. The spec does not define how held left/right are detected, how key-repeat events are normalized, or what happens in terminals that do not emit release events. This can make controls inconsistent across the exact platforms the spec targets.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "The persistence path depends on ProjectDirs but does not define behavior when directory discovery returns None, directory creation fails, the path is not writable, or storage is read-only. A game should remain playable even when high-score persistence fails. Specify graceful degradation: warn outside raw mode when possible, continue with in-memory scores, and never panic during startup/title rendering.",
      "severity": "minor"
    },
    {
      "category": "unnecessary-scope",
      "text": "Optional TOML config in v0.1 adds filesystem parsing, validation, error handling, and support burden without being required by the core user stories. It also expands the attack and failure surface for path/device/symlink/oversized-file cases. Consider cutting config.toml from v0.1 and keeping only CLI/env controls needed for accessibility and deterministic tests.",
      "severity": "minor"
    },
    {
      "category": "weak-implementation",
      "text": "The SSH byte-budget tests are likely too implementation-coupled to ratatui/crossterm internals and may not predict real PTY behavior. Capturing writes into a Vec does not model terminal emulator parsing, network buffering, backpressure, or line discipline. Keep the budget as a perf target, but add an integration-style PTY test or relax exact byte ceilings to avoid brittle CI failures.",
      "severity": "minor"
    },
    {
      "category": "missing-risk",
      "text": "Config parsing errors are logged and defaults are used, but score-file corruption triggers a destructive recovery path that overwrites the previous .bak. This can destroy forensic/debug information and repeatedly discard user data if a parser bug is introduced. Prefer timestamped backups or only overwrite .bak after a successful rename/write sequence is confirmed.",
      "severity": "minor"
    },
    {
      "category": "weak-implementation",
      "text": "The spec claims 'pinned' dependencies but uses broad Cargo.toml requirements such as serde = '1' and anyhow = '1'. Cargo.lock makes builds reproducible for the app, but the wording is misleading and does not cover advisory response or intentional update cadence. Say 'locked via Cargo.lock' and define how dependency updates and cargo deny advisories are handled.",
      "severity": "minor"
    }
  ],
  "summary": "The spec is much stronger than a typical game spec, but the biggest remaining risks are around local filesystem trust boundaries, unsafe terminal/signal recovery assumptions, legal release exposure, and a still-missing architecture contract. Some v0.1 scope, especially TOML config and highly specific SSH byte budgets, adds support surface before the core game needs it.",
  "suggested_next_version": "v0.3 should close the ops gaps: define safe directory creation/validation and persistence degradation, rewrite signal handling around main-loop-safe terminal I/O, replace the architecture placeholder with real contracts, specify terminal key-repeat semantics, add legal/release guidance, and cut or defer optional config unless it is required for launch.",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
