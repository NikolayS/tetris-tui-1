# Reviewer A — Codex

## summary

The spec has a solid core direction, but several correctness and ops claims are stronger than the implementation detail supports. The biggest issues are an impossible 7-bag test, under-specified terminal recovery, weak persistence atomicity, unpinned/incomplete dependencies, and an overclaim of Guideline compliance while deferring hold/T-spins.

## weak-implementation

- (major) The 7-bag property test is specified incorrectly: 'every 7-window is a permutation' is false for a standard 7-bag across bag boundaries. Only aligned bag chunks have that invariant; arbitrary sliding windows may contain duplicates. As written, CI would reject a correct implementation or force a nonstandard randomizer.
- (major) Persistence says 'atomic write via tempfile+rename' and later 'high_scores.json.tmp', but the dependency list omits `tempfile`, does not require fsync of the parent directory after rename, does not specify file permissions, and does not address symlink/race behavior for existing files. This is under-specified for the spec's own atomicity claim.
- (major) The spec claims 'pinned and minimal' dependencies, but no versions are pinned and several required crates are absent from the dependency list: `toml` for config, `tempfile` for atomic writes, `proptest` and `insta` for the stated tests. MSRV 1.75 compatibility is also not verified against chosen crate versions.
- (major) Top-out is ambiguous and likely wrong: 'any part of a locked piece sits entirely above row 20' mixes row indexing with visibility semantics in a 10x40 board. The spec should define coordinate origin, hidden rows, spawn rows, and the exact lock-out/block-out conditions with test cases.
- (minor) The architecture block is explicitly 'not yet specified' despite later listing modules and data flow. That leaves ownership boundaries, error propagation, lifecycle ordering, and test seams informal, which is risky for a four-engineer parallel plan.

## missing-risk

- (major) Terminal recovery only covers panic hooks, but not SIGINT/SIGTERM, process kill limits, suspend/resume, terminal resize, or failures during setup after some terminal mutations have already occurred. For a TUI, leaving the terminal in raw/alternate/cursor-hidden state is a core ops risk and needs an explicit teardown guard and signal strategy.
- (minor) The SSH responsiveness story relies on ratatui buffer diffing but lacks measurable acceptance tests for bytes written per frame, resize behavior, slow stdout/backpressure, and event polling latency. 'Inputs register within one game tick' over 30-50 ms RTT is not guaranteed by the current design language.
- (minor) `--reset-scores` includes a confirmation prompt, but the spec does not state that it runs before entering raw mode/alternate screen. Prompting inside raw mode is a common TUI failure mode and should be explicitly sequenced.

## unnecessary-scope

- (major) The goal promises 'modern Guideline-compliant mechanics' and transferable mainstream Tetris muscle memory, but v0.1 explicitly excludes hold and T-spin detection while still including back-to-back Tetris and SRS. That is a scope/claim mismatch: either downgrade the claim to 'Guideline-inspired' or include the missing mechanics needed for the advertised standard.
- (minor) Four veteran engineers for a single-player terminal Tetris v0.1 is excessive relative to the stated scope and may increase coordination cost more than delivery speed. The plan should justify this staffing or reduce the team/sprint model.

## suggested-next-version

v0.2 should tighten coordinate/top-out definitions, correct the 7-bag invariant, specify terminal lifecycle and signal handling, pin all runtime/dev dependencies with MSRV checks, harden persistence semantics, and either reduce the Guideline-compliance claim or add the missing mechanics.

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "weak-implementation",
      "text": "The 7-bag property test is specified incorrectly: 'every 7-window is a permutation' is false for a standard 7-bag across bag boundaries. Only aligned bag chunks have that invariant; arbitrary sliding windows may contain duplicates. As written, CI would reject a correct implementation or force a nonstandard randomizer.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "Terminal recovery only covers panic hooks, but not SIGINT/SIGTERM, process kill limits, suspend/resume, terminal resize, or failures during setup after some terminal mutations have already occurred. For a TUI, leaving the terminal in raw/alternate/cursor-hidden state is a core ops risk and needs an explicit teardown guard and signal strategy.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Persistence says 'atomic write via tempfile+rename' and later 'high_scores.json.tmp', but the dependency list omits `tempfile`, does not require fsync of the parent directory after rename, does not specify file permissions, and does not address symlink/race behavior for existing files. This is under-specified for the spec's own atomicity claim.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "The spec claims 'pinned and minimal' dependencies, but no versions are pinned and several required crates are absent from the dependency list: `toml` for config, `tempfile` for atomic writes, `proptest` and `insta` for the stated tests. MSRV 1.75 compatibility is also not verified against chosen crate versions.",
      "severity": "major"
    },
    {
      "category": "unnecessary-scope",
      "text": "The goal promises 'modern Guideline-compliant mechanics' and transferable mainstream Tetris muscle memory, but v0.1 explicitly excludes hold and T-spin detection while still including back-to-back Tetris and SRS. That is a scope/claim mismatch: either downgrade the claim to 'Guideline-inspired' or include the missing mechanics needed for the advertised standard.",
      "severity": "major"
    },
    {
      "category": "weak-implementation",
      "text": "Top-out is ambiguous and likely wrong: 'any part of a locked piece sits entirely above row 20' mixes row indexing with visibility semantics in a 10x40 board. The spec should define coordinate origin, hidden rows, spawn rows, and the exact lock-out/block-out conditions with test cases.",
      "severity": "major"
    },
    {
      "category": "missing-risk",
      "text": "The SSH responsiveness story relies on ratatui buffer diffing but lacks measurable acceptance tests for bytes written per frame, resize behavior, slow stdout/backpressure, and event polling latency. 'Inputs register within one game tick' over 30-50 ms RTT is not guaranteed by the current design language.",
      "severity": "minor"
    },
    {
      "category": "weak-implementation",
      "text": "The architecture block is explicitly 'not yet specified' despite later listing modules and data flow. That leaves ownership boundaries, error propagation, lifecycle ordering, and test seams informal, which is risky for a four-engineer parallel plan.",
      "severity": "minor"
    },
    {
      "category": "missing-risk",
      "text": "`--reset-scores` includes a confirmation prompt, but the spec does not state that it runs before entering raw mode/alternate screen. Prompting inside raw mode is a common TUI failure mode and should be explicitly sequenced.",
      "severity": "minor"
    },
    {
      "category": "unnecessary-scope",
      "text": "Four veteran engineers for a single-player terminal Tetris v0.1 is excessive relative to the stated scope and may increase coordination cost more than delivery speed. The plan should justify this staffing or reduce the team/sprint model.",
      "severity": "minor"
    }
  ],
  "summary": "The spec has a solid core direction, but several correctness and ops claims are stronger than the implementation detail supports. The biggest issues are an impossible 7-bag test, under-specified terminal recovery, weak persistence atomicity, unpinned/incomplete dependencies, and an overclaim of Guideline compliance while deferring hold/T-spins.",
  "suggested_next_version": "v0.2 should tighten coordinate/top-out definitions, correct the 7-bag invariant, specify terminal lifecycle and signal handling, pin all runtime/dev dependencies with MSRV checks, harden persistence semantics, and either reduce the Guideline-compliance claim or add the missing mechanics.",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
