# decisions

- No review-loop decisions yet.

## Round 1 — 2026-04-23T08:36:45.598Z

- accepted weak-implementation#1: Replaced the incorrect sliding-window 7-bag invariant with the correct aligned-bag permutation property plus a max-gap-≤-12 check, and explicitly noted sliding-window uniqueness is not a 7-bag property.
- accepted missing-risk#1: Added a full terminal lifecycle section covering SIGINT/SIGTERM, SIGTSTP/SIGCONT suspend/resume, setup-partway rollback, resize with too-small overlay, and stdout backpressure via BufWriter; lifecycle tests added to CI.
- accepted weak-implementation#2: Persistence now specifies tempfile::NamedTempFile in the same dir, file sync_all, parent-directory sync_all on unix, 0o600 perms, symlink semantics under rename, and .bak overwrite policy.
- accepted weak-implementation#3: Dependencies are now version-pinned, MSRV 1.75 is verified in CI, and the missing toml/tempfile/proptest/insta/signal-hook/assert_cmd crates were added to the explicit list.
- accepted unnecessary-scope#1: Reframed the goal as 'Guideline-inspired' (not 'Guideline-compliant'), made the v0.1 cut line explicit in the goal section and README, and kept hold + T-spin deferred to v0.2 without overclaiming.
- accepted weak-implementation#4: Defined a top-left origin with hidden rows 0..20 and visible rows 20..40, spelled out spawn position, block-out vs lock-out vs partial-lock conditions, and committed test cases for each.
- accepted missing-risk#2: Added measurable SSH responsiveness targets (bytes/frame budgets for idle/mid-move, <20 ms p99 input-to-diff latency, 8 ms poll timeout, explicit resize/backpressure behavior) and matching CI byte-budget tests.
- accepted weak-implementation#5: Replaced the empty architecture block with a labeled component diagram plus explicit ownership rules, error-propagation policy, and lifecycle ordering.
- accepted missing-risk#3: Called out that --reset-scores prompts in cooked mode before TerminalGuard::enter(), and added an assert_cmd test that asserts no ANSI escape sequences appear in its captured stdout.
- accepted unnecessary-scope#2: Reduced staffing to 2 engineers for sprints 1–2 and 4, with a third QA engineer only in sprint 3; merged the systems and TUI roles since terminal lifecycle and the ratatui/crossterm stack are tightly coupled.
