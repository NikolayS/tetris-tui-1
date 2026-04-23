# Reviewer B — Claude

## summary

v0.2 is a substantial improvement over v0.1 — coordinate system, top-out cases, atomic persistence, terminal lifecycle, and the corrected 7-bag property test are all well-specified. The dominant remaining issue is a still-empty architecture diagram block despite the v0.2 changelog claiming it was filled in. Beyond that, a small cluster of ambiguities around timing (soft-drop rate, lock-delay reset counter, frame/poll interaction) and a weaker-than-stated test plan for UI, DAS/ARR, and signal lifecycle tests deserve another pass. No idea-contradiction findings: the spec honors the 'no multiplayer/netcode/sound' disclaimer explicitly. Ready for a v0.3 that closes the architecture-diagram gap and resolves the timing ambiguities.

## missing-requirement

- (major) §3 Architecture contains an empty diagram block: the fenced code block reads only '(architecture not yet specified)'. The mandatory Architecture section has prose about modules and data flow but the promised architecture diagram is absent. The v0.2 changelog even claims 'fill in architecture diagram and ownership rules' — contradicted by the placeholder still present.

## ambiguity

- (major) Soft-drop rate is specified twice with conflicting semantics. §4 'Gravity & level curve' says 'Soft drop = 20× gravity', but §4 'Input handling (DAS/ARR)' says 'Soft-drop repeat: default 30 ms'. These cannot both govern soft-drop speed simultaneously (30 ms ≈ ~33 cells/s is independent of level, while 20× gravity scales with level). The spec does not say which takes precedence, whether 30 ms is a minimum cap, or how points (+1/cell) accrue when the two disagree. Needs one authoritative model.
- (major) Lock-delay reset-counter semantics are under-specified. §4 says 'resets on successful move/rotate up to a cap of 15 resets; piece locks when timer expires or after 15 resets while still grounded.' Unspecified: (a) does the reset counter clear when the piece leaves the ground and re-touches (as in modern Guideline), or persist across airborne intervals? (b) does the 500 ms timer start fresh on each reset, or only on initial grounding? (c) what happens when the timer expires mid-air (implicitly nothing, but not stated). Tests plan only calls out 'lock-delay 15-reset cap', leaving timer-expiration and re-ground behavior untested.
- (minor) O-piece spawn position in §4: 'horizontally centered on cols 3..7 (or 3..6 for O)'. Interpreting 3..6 as a half-open range yields 3 columns (3,4,5) for a piece that only occupies 2 columns — inconsistent with a bounding-box description. Should specify whether the O spawns with cells at cols 4–5 (Guideline), and whether 3..6 describes a 3-wide bounding box or a 2-wide piece range.
- (minor) Back-to-Back 4-line clear scoring is ambiguous. The table row 'Back-to-back 4-line clear ×1.5' does not state (a) whether the multiplier applies to the base 4-line-clear score (800 × level × 1.5 = 1200 × level) or to some other baseline, (b) what resets the B2B state (any non-4-line clear? any line clear? any lock?), and (c) whether the multiplier applies from the second consecutive 4-line clears onward or from the first. Scoring parameterized test in §5 mentions 'with/without B2B' but without a precise reset rule the test cases are ambiguous.
- (minor) Config file error-handling policy is inconsistent. §4 'Pre-TTY actions' promises 'path validation for --config' before entering raw mode (implying loud failure for bad paths), while 'Persistence' says 'Config file ... parse errors are logged and defaults are used (we don't fail startup).' Unclear: does a user-specified --config path with a parse error also silently fall back to defaults, or fail loudly? If the user explicitly pointed at a config, silent defaults would be surprising.
- (minor) Signal handler async-signal-safety is not addressed. §3 item 4 describes SIGTSTP handlers that 'restore terminal' (ANSI escape writes via crossterm) and SIGCONT handlers that 're-enter raw/alt/hide, force a full redraw' — these routines perform non-async-signal-safe operations (allocations, stdio writes). The idiomatic `signal-hook` pattern is to set an atomic flag and handle the transition from the main loop. Spec should either clarify that only flags are set in the handler and transitions happen on the next tick, or explicitly justify the direct-restore approach.
- (minor) Corrupt-file recovery in §4 Persistence says '.bak' is atomically renamed and 'If .bak already exists, it is overwritten — we keep only the most recent corrupt copy.' §5 test 7 asserts '.bak created and fresh list returned' but does not test the overwrite-of-existing-bak case, nor whether a corrupt-file event between two game sessions silently loses prior recoverable data. Low-stakes but the policy is worth covering in a test case since it's explicitly documented.
- (minor) Frame cadence vs. event poll interaction under-specified. §3 says 'Fixed ~60 Hz redraw cadence' and §4 says 'Event poll timeout: 8 ms, so input latency ≤ 1 poll + 1 step.' With a 60 Hz render tick (~16.6 ms) plus 8 ms event polls, the loop structure is unclear: does each iteration poll once then maybe render, or is there a separate render timer? This affects the claimed '< 20 ms p99' input-to-visible-change latency — with one render per 16.6 ms and an 8 ms poll, worst-case latency can exceed 20 ms even with a fake clock unless the loop structure is defined.

## weak-testing

- (minor) UI/rendering is explicitly excluded from TDD ('UI / rendering code is NOT TDD'd — developed against an in-memory Buffer, then covered by snapshot tests'). Snapshot tests catch regressions but not correctness of first implementation, and there is no acceptance test for the ghost-piece placement algorithm, next-piece preview content, or the HUD numeric fields. Consider adding unit tests for pure render helpers (ghost Y computation, HUD formatting) even if full-frame rendering stays snapshot-only.
- (minor) Signal lifecycle tests skip Windows/WSL and partially rely on host behavior. §5 test 9 exercises SIGINT and SIGTSTP/SIGCONT 'unix only' and verifies terminal state via 'stty -a and grepping for -icanon absence' — but the post-exit stty check is only meaningful when the child shares the parent's controlling TTY; under `assert_cmd` the default is piped stdio, in which case stty reads a different state than what the child manipulated. The test harness (pty allocation, stderr capture of restore escape sequences) is not specified, leaving the lifecycle guarantees in §3 only loosely validated.
- (minor) DAS/ARR behavior has no explicit test in §5. §4 defines DAS (170 ms), ARR (50 ms), and soft-drop repeat (30 ms) as configurable timing parameters feeding the in-sim state machine, but the tests plan lists no parameterized test asserting that holding left for N ms produces the correct shift sequence under each setting. This is a high-risk area for silent regressions — especially given the soft-drop conflict noted above.

## suggested-next-version

v0.3

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "missing-requirement",
      "text": "§3 Architecture contains an empty diagram block: the fenced code block reads only '(architecture not yet specified)'. The mandatory Architecture section has prose about modules and data flow but the promised architecture diagram is absent. The v0.2 changelog even claims 'fill in architecture diagram and ownership rules' — contradicted by the placeholder still present.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "Soft-drop rate is specified twice with conflicting semantics. §4 'Gravity & level curve' says 'Soft drop = 20× gravity', but §4 'Input handling (DAS/ARR)' says 'Soft-drop repeat: default 30 ms'. These cannot both govern soft-drop speed simultaneously (30 ms ≈ ~33 cells/s is independent of level, while 20× gravity scales with level). The spec does not say which takes precedence, whether 30 ms is a minimum cap, or how points (+1/cell) accrue when the two disagree. Needs one authoritative model.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "Lock-delay reset-counter semantics are under-specified. §4 says 'resets on successful move/rotate up to a cap of 15 resets; piece locks when timer expires or after 15 resets while still grounded.' Unspecified: (a) does the reset counter clear when the piece leaves the ground and re-touches (as in modern Guideline), or persist across airborne intervals? (b) does the 500 ms timer start fresh on each reset, or only on initial grounding? (c) what happens when the timer expires mid-air (implicitly nothing, but not stated). Tests plan only calls out 'lock-delay 15-reset cap', leaving timer-expiration and re-ground behavior untested.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "O-piece spawn position in §4: 'horizontally centered on cols 3..7 (or 3..6 for O)'. Interpreting 3..6 as a half-open range yields 3 columns (3,4,5) for a piece that only occupies 2 columns — inconsistent with a bounding-box description. Should specify whether the O spawns with cells at cols 4–5 (Guideline), and whether 3..6 describes a 3-wide bounding box or a 2-wide piece range.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "Back-to-Back 4-line clear scoring is ambiguous. The table row 'Back-to-back 4-line clear ×1.5' does not state (a) whether the multiplier applies to the base 4-line-clear score (800 × level × 1.5 = 1200 × level) or to some other baseline, (b) what resets the B2B state (any non-4-line clear? any line clear? any lock?), and (c) whether the multiplier applies from the second consecutive 4-line clears onward or from the first. Scoring parameterized test in §5 mentions 'with/without B2B' but without a precise reset rule the test cases are ambiguous.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "UI/rendering is explicitly excluded from TDD ('UI / rendering code is NOT TDD'd — developed against an in-memory Buffer, then covered by snapshot tests'). Snapshot tests catch regressions but not correctness of first implementation, and there is no acceptance test for the ghost-piece placement algorithm, next-piece preview content, or the HUD numeric fields. Consider adding unit tests for pure render helpers (ghost Y computation, HUD formatting) even if full-frame rendering stays snapshot-only.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "Signal lifecycle tests skip Windows/WSL and partially rely on host behavior. §5 test 9 exercises SIGINT and SIGTSTP/SIGCONT 'unix only' and verifies terminal state via 'stty -a and grepping for -icanon absence' — but the post-exit stty check is only meaningful when the child shares the parent's controlling TTY; under `assert_cmd` the default is piped stdio, in which case stty reads a different state than what the child manipulated. The test harness (pty allocation, stderr capture of restore escape sequences) is not specified, leaving the lifecycle guarantees in §3 only loosely validated.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "DAS/ARR behavior has no explicit test in §5. §4 defines DAS (170 ms), ARR (50 ms), and soft-drop repeat (30 ms) as configurable timing parameters feeding the in-sim state machine, but the tests plan lists no parameterized test asserting that holding left for N ms produces the correct shift sequence under each setting. This is a high-risk area for silent regressions — especially given the soft-drop conflict noted above.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "Config file error-handling policy is inconsistent. §4 'Pre-TTY actions' promises 'path validation for --config' before entering raw mode (implying loud failure for bad paths), while 'Persistence' says 'Config file ... parse errors are logged and defaults are used (we don't fail startup).' Unclear: does a user-specified --config path with a parse error also silently fall back to defaults, or fail loudly? If the user explicitly pointed at a config, silent defaults would be surprising.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "Signal handler async-signal-safety is not addressed. §3 item 4 describes SIGTSTP handlers that 'restore terminal' (ANSI escape writes via crossterm) and SIGCONT handlers that 're-enter raw/alt/hide, force a full redraw' — these routines perform non-async-signal-safe operations (allocations, stdio writes). The idiomatic `signal-hook` pattern is to set an atomic flag and handle the transition from the main loop. Spec should either clarify that only flags are set in the handler and transitions happen on the next tick, or explicitly justify the direct-restore approach.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "Corrupt-file recovery in §4 Persistence says '.bak' is atomically renamed and 'If .bak already exists, it is overwritten — we keep only the most recent corrupt copy.' §5 test 7 asserts '.bak created and fresh list returned' but does not test the overwrite-of-existing-bak case, nor whether a corrupt-file event between two game sessions silently loses prior recoverable data. Low-stakes but the policy is worth covering in a test case since it's explicitly documented.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "Frame cadence vs. event poll interaction under-specified. §3 says 'Fixed ~60 Hz redraw cadence' and §4 says 'Event poll timeout: 8 ms, so input latency ≤ 1 poll + 1 step.' With a 60 Hz render tick (~16.6 ms) plus 8 ms event polls, the loop structure is unclear: does each iteration poll once then maybe render, or is there a separate render timer? This affects the claimed '< 20 ms p99' input-to-visible-change latency — with one render per 16.6 ms and an 8 ms poll, worst-case latency can exceed 20 ms even with a fake clock unless the loop structure is defined.",
      "severity": "minor"
    }
  ],
  "summary": "v0.2 is a substantial improvement over v0.1 — coordinate system, top-out cases, atomic persistence, terminal lifecycle, and the corrected 7-bag property test are all well-specified. The dominant remaining issue is a still-empty architecture diagram block despite the v0.2 changelog claiming it was filled in. Beyond that, a small cluster of ambiguities around timing (soft-drop rate, lock-delay reset counter, frame/poll interaction) and a weaker-than-stated test plan for UI, DAS/ARR, and signal lifecycle tests deserve another pass. No idea-contradiction findings: the spec honors the 'no multiplayer/netcode/sound' disclaimer explicitly. Ready for a v0.3 that closes the architecture-diagram gap and resolves the timing ambiguities.",
  "suggested_next_version": "v0.3",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
