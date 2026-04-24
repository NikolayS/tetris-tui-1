# blocktxt multiplayer — SPEC v0.1

> **Scope note.** This document specifies the two-player competitive
> multiplayer mode added to blocktxt in game version **v0.2**. The project
> slug `multiplayer-v1` refers to the *first* version of the multiplayer
> subsystem, not to a rewrite of v0.1 gameplay. Single-player v0.1
> behaviour is unchanged and must continue to work offline.

## 1. Goal & why it's needed

**Goal.** Ship a head-to-head competitive mode where two human players,
each in their own terminal, play simultaneous falling-block games against
each other. Line clears by one player send garbage rows to the other
player's well. First player to top-out (block-out or lock-out) loses.

**Why this exists.** blocktxt v0.1 is a polished but strictly solitary
experience. Competitive multiplayer is the single largest replayability
multiplier we can add: it turns a 5-minute coffee-break toy into a
social artifact you send to a friend. It also exercises a class of
engineering — deterministic simulation, network protocols, matchmaking
— that the repo has deliberately avoided so far, so v0.2 is the right
moment to pay that cost once, cleanly, rather than bolting it on later.

**Non-goals for this version (enforced strictly).**

- **NOT a spectator / streaming feature.** No watch-only clients in v1.
- **NOT a leaderboard / rating service.** Matches are ad-hoc; no Elo,
  no persistent win/loss records beyond a local session counter.
- **NOT a lobby / chat / social product.** Two players connect via a
  short room code. No friends list, no chat UI, no profiles.
- **NOT cross-version compatible.** v0.2 clients only talk to v0.2
  servers and v0.2 peers. Version mismatch → clean refusal.
- **NOT a replacement for single-player.** Single-player v0.1 remains
  the default launch mode; multiplayer is opt-in via a subcommand.
- **NOT >2 players.** No free-for-all, no teams. Exactly 2.

## 2. Design decisions (resolved from interview)

The interview left five questions as "decide for me". The chosen answers
and their rationale are recorded below so future reviewers don't have to
reverse-engineer them.

### 2.1 Language — Rust (client + server)

The client is already Rust; sharing the simulation crate between client
and server is the single highest-leverage decision we can make. It
makes bit-exact determinism trivial (same code, same RNG, same fixed-
point math) and halves the test surface. Server uses `tokio` for async
I/O; client keeps its existing blocking main loop and speaks to the
network from a dedicated thread.

### 2.2 Topology — authoritative central server ("relay + referee")

The server runs the canonical simulation for *both* players at 60 Hz.
Clients send inputs; server returns authoritative state diffs. Chosen
over:

- **Peer-to-peer with a host**: needs NAT traversal (STUN/TURN) and one
  peer becomes de-facto authoritative anyway — same cheating surface,
  worse UX.
- **Pure lockstep**: elegant for identical-input games but Tetris has
  independent boards; we'd still need a referee for garbage exchange,
  at which point we're 80 % of the way to server-authoritative.
- **Rollback netcode**: premium experience but a full sprint of work
  we can defer until there's demand.

Server is stateless between matches and horizontally scalable; a single
$5/month VM comfortably hosts hundreds of concurrent matches at 60 Hz
with the tick budget below.

### 2.3 Sync model — server-authoritative fixed-tick, client input-forward with input delay

- Fixed simulation tick: **60 Hz** (16.67 ms), matching the existing
  single-player frame cadence.
- Clients forward inputs with a monotonically increasing sequence
  number plus the client tick they were pressed on.
- Server applies inputs at **`client_tick + INPUT_DELAY`** where
  `INPUT_DELAY = 2 ticks ≈ 33 ms`. This absorbs typical jitter without
  visible lag and avoids the complexity of rollback.
- Server broadcasts a **state snapshot** every tick (two boards,
  scores, incoming-garbage queues, active pieces, phase). Snapshots
  are delta-compressed against the last-acked snapshot per client.
- No client-side prediction in v1: on LAN / sub-50 ms RTT the
  round-trip + 2-tick input delay is imperceptible for Tetris-paced
  play. Prediction is an explicit v2 lever if telemetry shows we need
  it.

### 2.4 Transport — TLS-terminated WebSocket, binary frames

- WebSocket over TLS (`wss://`) on port **443**. Chosen for
  firewall-friendliness (corporate / school networks), standard Rust
  tooling (`tokio-tungstenite`), and because matchmaking and game
  traffic share one connection.
- Binary frames carry a compact framed protocol (see §4.3); no JSON on
  the hot path.
- TLS is mandatory: room codes travel over the wire, and we refuse to
  teach users to type plaintext credentials into a terminal.

### 2.5 Scale target — 2 players, 60 Hz, 150 ms RTT budget

| Dimension          | v0.1 target                                        |
|--------------------|----------------------------------------------------|
| Players per match  | exactly 2                                          |
| Server tick rate   | 60 Hz                                              |
| Client frame rate  | 60 fps (unchanged)                                 |
| RTT budget         | ≤ 150 ms one-way latency for "feels responsive"    |
| Bandwidth / client | ≤ 8 KiB/s steady state (delta snapshots)           |
| Concurrent matches | 200 per 1-vCPU / 512 MiB server                    |
| Match duration cap | 10 minutes (hard cap; otherwise sudden-death)      |

## 3. User stories

1. **Casey the commuter** (persona: single-player regular, 30 min/day
   on trains). *Action:* runs `blocktxt mp host`, reads aloud the
   5-character room code to a friend on a call. *Outcome:* within 10
   seconds both are playing head-to-head; no signup, no account, no
   browser.

2. **Riley the reviewer** (persona: rust-curious developer trying
   blocktxt for the first time). *Action:* runs `blocktxt mp join
   ABCDE` from a fresh install behind a corporate HTTP proxy.
   *Outcome:* connection succeeds over port 443; if it fails, the CLI
   prints an actionable error naming *which* step failed (DNS, TLS,
   handshake, version-mismatch, room-not-found).

3. **Sam the skeptic** (persona: privacy-conscious, offline-first).
   *Action:* launches `blocktxt` with no arguments. *Outcome:* gets
   exactly the v0.1 single-player experience, no network calls, no
   background threads touching the internet, binary size unchanged
   beyond the added multiplayer code path.

4. **Taylor the twitch player** (persona: competitive Tetris player
   used to Puyo Puyo Tetris / TETR.IO). *Action:* clears a tetris
   (4-line) while their opponent has a full well. *Outcome:* opponent
   receives 4 garbage rows within one server tick of the flash-phase
   completing, and the garbage-incoming indicator in Taylor's HUD
   shows pending rows they're about to send.

5. **Jordan the judge** (persona: QA / release-blocking role).
   *Action:* runs the manual multiplayer test plan against a release
   candidate on macOS arm64 and Linux x86_64 simultaneously.
   *Outcome:* can verify in under 15 minutes that matchmaking,
   gameplay, disconnection, and clean shutdown all behave; any failure
   maps to a specific checklist item.

## 4. Architecture

<!-- architecture:begin -->

```text
(architecture not yet specified)
```

<!-- architecture:end -->

### 4.1 Components & boundaries

```
┌──────────────────── blocktxt client (Rust binary) ────────────────────┐
│                                                                       │
│  cli  ─▶  mode dispatch  ─▶  single-player loop (unchanged v0.1)      │
│                      │                                                │
│                      └───▶   mp::client  ──────────────┐              │
│                                 │                       │             │
│                                 ▼                       ▼             │
│                          net thread (tokio)     render/input (main)   │
│                                 │                       │             │
│                                 └── shared state ◀──────┘             │
│                                     (sim::SnapshotMirror)             │
└─────────────────────────────────┬─────────────────────────────────────┘
                                  │ wss://  (TLS 1.3, binary frames)
                                  ▼
┌──────────────────── blocktxt-server (new binary) ─────────────────────┐
│                                                                       │
│  ws accept ─▶ session ─▶ matchmaker (room code)                       │
│                    │         │                                        │
│                    └────────▶│  match ─▶ sim (60 Hz, authoritative)   │
│                              │             │                          │
│                              │             └─▶ snapshot diff          │
│                              └───────────────▶ broadcast to 2 peers   │
└───────────────────────────────────────────────────────────────────────┘
```

New crates / modules in the workspace:

- `blocktxt-sim` (library, extracted from existing `src/game/`): pure,
  deterministic simulation. No `Instant::now()`, no thread-local RNG,
  no I/O. Shared verbatim between client and server.
- `blocktxt-proto` (library): wire-format types, framing, version
  constants, `Encode`/`Decode` traits.
- `blocktxt` (existing binary): gains `mp host` / `mp join` /
  `mp local` subcommands.
- `blocktxt-server` (new binary): WebSocket listener, matchmaker,
  match runner.

### 4.2 Key abstractions

- **`sim::Match`** — holds two `GameState` instances plus a shared
  garbage exchange queue. Pure function: `step(inputs_a, inputs_b,
  dt) -> (events_a, events_b, garbage_delta)`.
- **`sim::DetRng`** — seeded ChaCha20 RNG, replaces `StdRng` in the
  simulation crate. Both 7-bag generators are seeded from the match
  seed so sequences are reproducible for replays / desync audits.
- **`proto::Frame`** — tagged union of `Hello`, `HostRoom`,
  `JoinRoom`, `RoomReady`, `MatchStart`, `Input`, `Snapshot`, `Ack`,
  `Pong`, `Goodbye`, `Error`. Versioned by the `Hello` handshake.
- **`client::SnapshotMirror`** — lock-free SPSC slot that the net
  thread writes and the render thread reads each frame. One tick
  behind at worst.
- **`server::RoomCode`** — 5-character `Crockford-base32` code (no
  `ILOU`); 32⁵ ≈ 33 M codes. Matchmaker rejects codes already live.

### 4.3 Wire protocol (v1)

All frames are length-prefixed (u16 big-endian) binary payloads inside
WebSocket binary messages. Fields are little-endian `bincode` v2 with
the `varint` codec. `proto_version = 1` is carried in `Hello` and any
mismatch is fatal.

| Tag | Name          | Fields (summary)                                  |
|----:|---------------|---------------------------------------------------|
| 0x01| Hello         | proto_version, client_build, capabilities         |
| 0x02| HostRoom      | — (server assigns code)                           |
| 0x03| JoinRoom      | code (5 bytes)                                    |
| 0x04| RoomReady     | code, role (host/guest), peer_build               |
| 0x05| MatchStart    | match_id, seed (u64), start_tick                  |
| 0x06| Input         | client_tick, seq, bitset of Inputs                |
| 0x07| Snapshot      | server_tick, ack_seq, delta-encoded state         |
| 0x08| Ack           | ack_tick                                          |
| 0x09| Pong          | echoed nonce, server_tick                         |
| 0x0A| Goodbye       | reason (enum)                                     |
| 0x0B| Error         | code, human-readable string                       |

`Input` frames are idempotent and replay-safe (seq numbers). Snapshots
are sent unconditionally every tick at 60 Hz; the server rate-limits
them to a minimum tick gap of 1 even when acks stall.

## 5. Implementation details

### 5.1 Deterministic simulation

- Extract `src/game/` into `blocktxt-sim`. Swap `StdRng` for
  `ChaCha20Rng` seeded from the match seed.
- Remove all `Instant::now()` from the simulation; time advances only
  via `step(dt)` where `dt` is always exactly one tick on the server.
- Add a property test: given the same seed and the same input stream,
  two independent runs produce byte-identical `Match` state after N
  ticks (N = 10 000).
- The *client* continues to use a real clock for animations (juice,
  spawn-fade, score rollup) — those are rendered from the mirrored
  snapshot and never feed back into simulation.

### 5.2 State transitions (client)

```
            ┌────────────┐  mp host / mp join   ┌────────────────┐
            │   Title    │ ───────────────────▶ │   Connecting   │
            └─────┬──────┘                      └───────┬────────┘
                  │ any key (single-player)             │
                  ▼                                     ▼
            ┌────────────┐                      ┌────────────────┐
            │  Playing   │                      │ WaitingForPeer │
            │ (local v1) │                      └───────┬────────┘
            └────────────┘                              │ RoomReady
                                                        ▼
                                                ┌────────────────┐
                                                │  MatchStarting │ (3-2-1 countdown)
                                                └───────┬────────┘
                                                        ▼
                                                ┌────────────────┐
                                 ┌──────────────│   MatchPlaying │
                                 │ disconnect   └───────┬────────┘
                                 ▼                      │ GameOver event
                           ┌────────────┐               ▼
                           │   Aborted  │       ┌────────────────┐
                           └────────────┘       │   MatchResult  │
                                                └────────────────┘
```

### 5.3 Garbage exchange

- Line clears produce `garbage_out = lines_cleared - 1` (standard
  versus-Tetris rule; single clears send 0). B2B quads send 5.
- Garbage is queued per-recipient on the server. Before the
  recipient's next piece spawn, the server drains the queue by
  inserting up to 8 rows of garbage at the bottom of their board,
  with one random gap column per row (RNG seeded from match seed +
  tick + recipient id, so deterministic).
- Incoming-garbage indicator: a HUD column on the inboard side of the
  playfield shows pending rows; same colour ramp as existing animation
  DIM → OVERLAY → NEW_BEST as the queue grows.

### 5.4 Disconnect & lag handling

- Heartbeat: client sends `Pong(nonce)` every 500 ms; server replies.
  No pong for 3 s → server declares the session stalled and awards
  the match to the peer.
- If the *peer* disconnects mid-match, the surviving client gets a
  `Goodbye(PeerDisconnected)` and a win by default.
- Clean shutdown: Ctrl-C / SIGTERM sends `Goodbye(UserQuit)` before
  closing the socket.

### 5.5 Security posture

- TLS 1.3 only; `rustls` with the `ring` provider. No plaintext
  fallback.
- Room codes rate-limited: ≤ 10 `JoinRoom` attempts per IP per minute.
- Server ignores client-supplied `server_tick` fields; tick is a
  server-owned monotonic counter.
- Server validates every client `Input` against the simulation — an
  input that names a non-existent move (e.g. `HardDrop` during
  `Phase::GameOver`) is silently dropped, not crash-inducing.
- No persistent data about players is stored server-side. Logs are
  match_id + truncated IP hash, 7-day retention.

### 5.6 CLI surface

```
blocktxt                         # unchanged: single-player v0.1
blocktxt mp host [--server URL]  # host a room, print room code, wait
blocktxt mp join CODE [--server URL]
blocktxt mp local                # two local players, split-screen TTY
                                 #  (no network; dev aid and offline mode)
```

`--server` defaults to the compiled-in official URL; overridable for
self-hosting. `BLOCKTXT_SERVER` env var is also honoured.

## 6. Tests plan

### 6.1 Red/green TDD — built test-first

The following are written as failing tests *before* their implementation
lands, per strict TDD:

- **`sim::Match` determinism** — property test, 10 000 random input
  sequences across two independent instances → identical state.
- **`proto` round-trip** — for every frame variant, `decode(encode(x))
  == x`; malformed bytes → specific `Error` variant (never panic).
- **Version-mismatch refusal** — client with `proto_version = 2`
  against server v1 receives `Error(VersionMismatch)` and exits with
  status 3.
- **Garbage exchange table** — `lines_cleared → garbage_out` matches
  the specified table exactly, including B2B × 1.5 rounding.
- **Room-code charset** — generated codes never contain `I`, `L`,
  `O`, `U`; property test over 100 k samples.
- **Tick-budget invariant** — server `step` for 2 active matches
  completes in < 1 ms on the reference CI VM (criterion bench gated
  as a test).

### 6.2 Integration (green-path, written after the TDD pieces above)

- **Loopback match**: spin up `blocktxt-server` on `127.0.0.1:0`,
  connect two in-process clients, script a 30-second match, assert a
  `MatchResult` event is emitted and both clients exit cleanly.
- **Disconnect mid-match**: kill one client's socket; assert the
  other receives `Goodbye(PeerDisconnected)` within 3 s.
- **Corporate-proxy simulation**: run the loopback suite through
  `mitmproxy` in forward mode to confirm wss:443 still works.
- **TLS refusal**: plaintext `ws://` connection attempt → server
  drops with clear error, no process crash.
- **Cross-arch determinism**: run the determinism property test on
  macOS arm64 and Linux x86_64 in CI; seeds must produce identical
  SHA-256 digests of the final state.

### 6.3 CI additions

- New matrix entry: `cargo test -p blocktxt-sim -p blocktxt-proto
  -p blocktxt-server`.
- Fuzz target (`cargo-fuzz`) on `proto::decode` — 5-minute nightly
  run; any panic fails the job.
- `cargo deny check` gains an advisories gate for `rustls`, `tokio`,
  `tokio-tungstenite`.

### 6.4 Manual test plan (release-blocking)

Extends `docs/manual-test-plan.md` with a **Multiplayer** section:

- [ ] `mp host` prints a 5-char code; `mp join <code>` on a second
      machine connects within 2 s.
- [ ] Line clear on host inserts garbage on guest well within one
      visible frame of flash-phase completion.
- [ ] Ctrl-C on host ends the match cleanly for guest with
      `opponent disconnected` overlay.
- [ ] Wrong room code → "room not found" error, no crash.
- [ ] Server unreachable → "cannot reach blocktxt server" within 5 s,
      clean process exit, cooked terminal.
- [ ] `mp local` works fully offline (no network syscalls observed
      via `dtruss` / `strace`).

## 7. Team

Veteran experts to hire for v0.2 multiplayer:

- **Veteran real-time multiplayer game networking engineer (1)** —
  lead for protocol, tick/sync model, garbage exchange, desync audit.
- **Veteran Rust systems engineer (1)** — extracts `blocktxt-sim`
  crate, enforces no-std-like purity (no clocks, no thread-locals),
  owns `blocktxt-proto` codec and fuzzing.
- **Veteran async-Rust / tokio services engineer (1)** — builds
  `blocktxt-server` (WS accept, matchmaker, match runner, graceful
  shutdown, observability).
- **Veteran CLI / TUI engineer (1)** — wires `mp host` / `mp join`
  subcommands, connection-status overlays, incoming-garbage HUD, and
  integrates the net thread with the existing render loop without
  breaking single-player cadence.
- **Veteran application security engineer (0.5)** — reviews TLS
  config, input validation, rate limits, and the threat model;
  sign-off before public server goes live.
- **Veteran release/QA engineer (0.5)** — owns the manual multiplayer
  test plan, cross-platform matrix, and the "does single-player still
  work offline" regression gate.

Total: **4 full-time + 1 split** specialist hires on top of the
existing maintainers.

## 8. Implementation plan (sprints)

Each sprint is 1 calendar week. Ordering between specialists shown
with `→` for dependencies and `∥` for parallel work.

### Sprint 1 — Foundations (parallel everywhere)

- Rust systems ∥ networking: extract `blocktxt-sim` crate, swap RNG
  for `ChaCha20Rng`, delete all `Instant::now()` from sim. (TDD:
  determinism property test first, red.)
- Networking ∥ Rust systems: draft `blocktxt-proto` frame catalogue
  + round-trip tests (red → green).
- CLI: add `blocktxt mp --help` stub subcommand skeleton behind a
  compile-time `multiplayer` feature flag, defaulted *off* until
  Sprint 3 so main stays shippable.
- Security: threat-model doc + `cargo deny` rules merged.

### Sprint 2 — Server & offline two-player

- Async-tokio: `blocktxt-server` accepts TLS-terminated WS
  connections, implements matchmaker with room codes, runs one
  `sim::Match` per room at 60 Hz.
- Networking → async-tokio: garbage-exchange logic lands in
  `sim::Match`; server broadcasts snapshots.
- CLI ∥: `blocktxt mp local` (two-player on one machine, no network)
  ships as a dev aid and exercises the new `sim::Match` end-to-end.
- QA: multiplayer manual test plan draft v1, reviewed.

### Sprint 3 — Online play & polish

- CLI → networking: `mp host` / `mp join` wired to server; connection
  overlays ("waiting for opponent", "connecting…", "peer
  disconnected") rendered by TUI engineer.
- Networking ∥ CLI: input-delay pipeline + snapshot mirror on client;
  incoming-garbage HUD column in playfield view.
- Security: penetration pass on deployed staging server (rate-limit
  verification, TLS config audit, fuzzing).
- Async-tokio ∥ QA: observability (match_id structured logs,
  Prometheus `/metrics` endpoint for concurrent_matches and
  tick_budget_p99).
- CI: cross-arch determinism check, fuzz job, release artifact for
  `blocktxt-server` (static musl binary).

### Sprint 4 — Release hardening

- QA leads the manual matrix across macOS arm64, macOS x86_64,
  Linux x86_64; all veterans on-call for triage.
- Security sign-off before flipping DNS for the public server.
- Docs: README multiplayer section, self-hosting guide.
- Cut `blocktxt v0.2.0` + `blocktxt-server v0.1.0`.

## 9. Risks & mitigations

| Risk                                     | Likelihood | Mitigation                                              |
|------------------------------------------|------------|---------------------------------------------------------|
| Simulation desync between arches         | Med        | Fixed-point only; CI digest check on macOS & Linux.     |
| Corporate networks block wss:443         | Low        | Use standard port; document HTTP_PROXY support.         |
| Server cost balloons                     | Low        | Match cap of 10 min; 200 matches/VM; scale horizontally.|
| Single-player regresses                  | Med        | `multiplayer` feature is opt-in compile gate in Sprint 1–2; existing v0.1 manual test plan stays green every sprint. |
| Room-code collision                      | Very low   | 33 M codes; matchmaker rejects live collisions.         |

## 10. Changelog

- v0.1 (this document) — initial scaffold: language/topology/sync/
  transport/scale decided; architecture, protocol, tests, team,
  4-sprint plan laid out; strict non-goals recorded.
