# Reviewer B — Claude

## summary

Spec is structurally complete — all nine mandatory sections are present and the v0.1 scope is well-bounded with clear out-of-scope discipline matching the idea's disclaimers (no multiplayer/netcode/sound/GUI). The strongest weaknesses are (1) an unfilled architecture diagram placeholder in §3, (2) several rule-layer ambiguities in §4 (animation frame/ms mismatch, soft-drop behavior at max gravity, B2B reset condition, top-out wording, lock-delay cap edge case), and (3) gaps in the red/green TDD list for top-out, level progression, soft/hard-drop scoring, B2B reset, DAS/ARR timing, and ghost/preview behavior despite those being v0.1-in-scope. No idea-contradiction findings: the non-goals in §1 faithfully echo the idea's 'Out of scope' list.

## ambiguity

- (major) §3 Architecture contains a literal placeholder diagram: the fenced block between `<!-- architecture:begin -->` and `<!-- architecture:end -->` reads `(architecture not yet specified)`. The surrounding prose (module layout, data flow) partially substitutes for it, but the canonical architecture diagram slot is empty. Either remove the placeholder framing or populate the block with the actual component/data-flow diagram.
- (major) §4 line-clear animation says `2 frames of inverted cells, ~100 ms`, but the render loop is specified as `~60 Hz` (≈16.7 ms/frame). 2 frames at 60 Hz is ~33 ms, not 100 ms — so either the frame count, the duration, or the redraw cadence is wrong. Pick one authoritative definition (duration in ms OR frame count at the stated rate) and reconcile.
- (major) §4 defines `Soft drop = 20× gravity` but §4 also clamps gravity at level 20 where the Guideline formula yields ≈sub-millisecond per-cell times; 20× of that is physically unrealizable inside a ~60 Hz loop (soft drop effectively becomes hard drop without the +2/cell bonus). Specify a soft-drop floor/clamp (e.g. instant-drop behavior, or a minimum per-cell interval) so behavior at max level is deterministic.
- (major) §4 specifies `Back-to-back Tetris ×1.5` but never states the B2B reset condition (i.e., which clears break the chain — any non-Tetris line clear? soft-drop-only? game pause?). Without this, the scoring table is underspecified and the §5 `with/without B2B` test dimension is ambiguous. Add a one-sentence rule: what starts, maintains, and breaks the B2B bonus.
- (minor) §4 top-out condition reads `any part of a locked piece sits entirely above row 20 (visible area)` — `any part … sits entirely above` is self-contradictory (a part cannot be entirely above if other parts are below). The intended Guideline rule is lock-out (the *whole* piece locked above the visible field). Rephrase to `a piece locks with all of its cells above row 20` (lock-out) and/or `a new piece spawns overlapping the stack` (block-out), and name the two cases distinctly.
- (minor) §4 says DAS/ARR values are `configurable in the TOML config (v0.2 surfaces a UI for it)` while elsewhere stating the TOML loader ships in v0.1. Clarify that the *file-based* config is v0.1 and only the in-game settings UI is v0.2, so the v0.1 scope reader is not left guessing whether keybinds/timing are user-configurable at release.
- (minor) §2 user story 3 promises `T-spin-adjacent kicks behave per the SRS kick table` while §4 defers T-spin detection to v0.2 and §8 lists `T-spin detection` as deferred. The story phrasing is defensible (kicks ≠ detection), but it reads as a T-spin commitment. Reword to `SRS kicks (including the moves that would yield T-spins) are resolved per the kick table; T-spin *scoring/detection* is deferred to v0.2`.
- (minor) §4 persistence says a corrupt high-score file is backed up to `high_scores.json.bak` and fresh scores are started, but does not define behavior if the `.bak` already exists (overwrite? rotate? fail?). Pin a rule so repeated corruption is not silently data-destructive.
- (minor) §4 specifies `lock delay ... up to a cap of 15 resets; piece locks when timer expires or after 15 resets when still grounded`. The phrase `after 15 resets when still grounded` is ambiguous about whether the 16th successful move/rotate *itself* forces a lock, or whether the cap only takes effect once the piece is grounded with no further input. State the rule as: on the 16th reset attempt while grounded, the piece locks immediately regardless of timer.

## weak-testing

- (major) §5's test-first list covers SRS, bag, scoring, gravity, lock-delay, line detection, and persistence, but omits explicit TDD coverage for: (a) top-out detection (§4 defines two distinct top-out conditions), (b) level progression at the 10-line boundary, (c) soft-drop/hard-drop point awards (+1/cell, +2/cell), and (d) B2B multiplier reset. These are rule-layer behaviors that belong in the same red-first bucket as the rest of `rules.rs`. Add them to the red/green list or justify their exclusion.
- (minor) User story 1 commits to a measurable SLA (`launches into a playable board in under 200 ms`) and user story 4 commits to `no full-screen repaints per frame` and `inputs register within one game tick`, but §5 has no performance/latency test. Either drop the numeric guarantees from the user stories or add a CI benchmark (e.g. `criterion` harness measuring cold-start to first-frame, and an assertion on dirty-region count in snapshot tests).
- (minor) §5 does not call out any test for the ghost-piece projection or the next-piece preview — both are explicitly IN-scope for v0.1 per §4's nice-to-have cut line. Snapshot tests for `mid-game` may incidentally cover them, but a targeted unit test (ghost row == first colliding drop row; preview reflects next bag entry) would prevent silent regressions.
- (minor) §5 lists a DAS/ARR model with specific defaults (170 ms / 50 ms / 30 ms) but no timing test with the fake `Clock`. Given §7 calls DAS/ARR out as sprint-3 work, add a deterministic test: fake clock + held input → first shift at t=170 ms, subsequent shifts at 50 ms intervals.
- (minor) §5 item 9 (panic-safety) asserts `raw mode is off after exit` via a smoke script, but does not check the other two terminal-state mutations listed in §3: alternate screen exit and cursor visibility restore. Extend the assertion to cover all three, or the panic hook can regress on two of them undetected.

## suggested-next-version

v0.2

<!-- samospec:critique v1 -->
{
  "findings": [
    {
      "category": "ambiguity",
      "text": "§3 Architecture contains a literal placeholder diagram: the fenced block between `<!-- architecture:begin -->` and `<!-- architecture:end -->` reads `(architecture not yet specified)`. The surrounding prose (module layout, data flow) partially substitutes for it, but the canonical architecture diagram slot is empty. Either remove the placeholder framing or populate the block with the actual component/data-flow diagram.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§4 line-clear animation says `2 frames of inverted cells, ~100 ms`, but the render loop is specified as `~60 Hz` (≈16.7 ms/frame). 2 frames at 60 Hz is ~33 ms, not 100 ms — so either the frame count, the duration, or the redraw cadence is wrong. Pick one authoritative definition (duration in ms OR frame count at the stated rate) and reconcile.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§4 defines `Soft drop = 20× gravity` but §4 also clamps gravity at level 20 where the Guideline formula yields ≈sub-millisecond per-cell times; 20× of that is physically unrealizable inside a ~60 Hz loop (soft drop effectively becomes hard drop without the +2/cell bonus). Specify a soft-drop floor/clamp (e.g. instant-drop behavior, or a minimum per-cell interval) so behavior at max level is deterministic.",
      "severity": "major"
    },
    {
      "category": "ambiguity",
      "text": "§4 specifies `Back-to-back Tetris ×1.5` but never states the B2B reset condition (i.e., which clears break the chain — any non-Tetris line clear? soft-drop-only? game pause?). Without this, the scoring table is underspecified and the §5 `with/without B2B` test dimension is ambiguous. Add a one-sentence rule: what starts, maintains, and breaks the B2B bonus.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "§5's test-first list covers SRS, bag, scoring, gravity, lock-delay, line detection, and persistence, but omits explicit TDD coverage for: (a) top-out detection (§4 defines two distinct top-out conditions), (b) level progression at the 10-line boundary, (c) soft-drop/hard-drop point awards (+1/cell, +2/cell), and (d) B2B multiplier reset. These are rule-layer behaviors that belong in the same red-first bucket as the rest of `rules.rs`. Add them to the red/green list or justify their exclusion.",
      "severity": "major"
    },
    {
      "category": "weak-testing",
      "text": "User story 1 commits to a measurable SLA (`launches into a playable board in under 200 ms`) and user story 4 commits to `no full-screen repaints per frame` and `inputs register within one game tick`, but §5 has no performance/latency test. Either drop the numeric guarantees from the user stories or add a CI benchmark (e.g. `criterion` harness measuring cold-start to first-frame, and an assertion on dirty-region count in snapshot tests).",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§5 does not call out any test for the ghost-piece projection or the next-piece preview — both are explicitly IN-scope for v0.1 per §4's nice-to-have cut line. Snapshot tests for `mid-game` may incidentally cover them, but a targeted unit test (ghost row == first colliding drop row; preview reflects next bag entry) would prevent silent regressions.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§5 lists a DAS/ARR model with specific defaults (170 ms / 50 ms / 30 ms) but no timing test with the fake `Clock`. Given §7 calls DAS/ARR out as sprint-3 work, add a deterministic test: fake clock + held input → first shift at t=170 ms, subsequent shifts at 50 ms intervals.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4 top-out condition reads `any part of a locked piece sits entirely above row 20 (visible area)` — `any part … sits entirely above` is self-contradictory (a part cannot be entirely above if other parts are below). The intended Guideline rule is lock-out (the *whole* piece locked above the visible field). Rephrase to `a piece locks with all of its cells above row 20` (lock-out) and/or `a new piece spawns overlapping the stack` (block-out), and name the two cases distinctly.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4 says DAS/ARR values are `configurable in the TOML config (v0.2 surfaces a UI for it)` while elsewhere stating the TOML loader ships in v0.1. Clarify that the *file-based* config is v0.1 and only the in-game settings UI is v0.2, so the v0.1 scope reader is not left guessing whether keybinds/timing are user-configurable at release.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§2 user story 3 promises `T-spin-adjacent kicks behave per the SRS kick table` while §4 defers T-spin detection to v0.2 and §8 lists `T-spin detection` as deferred. The story phrasing is defensible (kicks ≠ detection), but it reads as a T-spin commitment. Reword to `SRS kicks (including the moves that would yield T-spins) are resolved per the kick table; T-spin *scoring/detection* is deferred to v0.2`.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4 persistence says a corrupt high-score file is backed up to `high_scores.json.bak` and fresh scores are started, but does not define behavior if the `.bak` already exists (overwrite? rotate? fail?). Pin a rule so repeated corruption is not silently data-destructive.",
      "severity": "minor"
    },
    {
      "category": "weak-testing",
      "text": "§5 item 9 (panic-safety) asserts `raw mode is off after exit` via a smoke script, but does not check the other two terminal-state mutations listed in §3: alternate screen exit and cursor visibility restore. Extend the assertion to cover all three, or the panic hook can regress on two of them undetected.",
      "severity": "minor"
    },
    {
      "category": "ambiguity",
      "text": "§4 specifies `lock delay ... up to a cap of 15 resets; piece locks when timer expires or after 15 resets when still grounded`. The phrase `after 15 resets when still grounded` is ambiguous about whether the 16th successful move/rotate *itself* forces a lock, or whether the cap only takes effect once the piece is grounded with no further input. State the rule as: on the 16th reset attempt while grounded, the piece locks immediately regardless of timer.",
      "severity": "minor"
    }
  ],
  "summary": "Spec is structurally complete — all nine mandatory sections are present and the v0.1 scope is well-bounded with clear out-of-scope discipline matching the idea's disclaimers (no multiplayer/netcode/sound/GUI). The strongest weaknesses are (1) an unfilled architecture diagram placeholder in §3, (2) several rule-layer ambiguities in §4 (animation frame/ms mismatch, soft-drop behavior at max gravity, B2B reset condition, top-out wording, lock-delay cap edge case), and (3) gaps in the red/green TDD list for top-out, level progression, soft/hard-drop scoring, B2B reset, DAS/ARR timing, and ghost/preview behavior despite those being v0.1-in-scope. No idea-contradiction findings: the non-goals in §1 faithfully echo the idea's 'Out of scope' list.",
  "suggested_next_version": "v0.2",
  "usage": null,
  "effort_used": "max"
}
<!-- samospec:critique end -->
