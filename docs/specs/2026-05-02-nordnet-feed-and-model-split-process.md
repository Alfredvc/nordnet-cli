# Nordnet Feed + Model Split — Build Process

Companion to `docs/specs/2026-05-02-nordnet-feed-and-model-split-design.md`. Custom pipeline for the two-PR effort. Distinct from the root `PROCESS.md` (which built the REST crate end-to-end and has reached Phase 5 + post-Phase-5 hardening).

This pipeline assumes the REST pipeline is closed: workspace compiles, `cargo test --workspace` is green, `models/`, `auth.rs`, and `ids.rs` exist in `crates/nordnet-api/src/` and are stable. We are not re-extracting docs, not re-deriving types, not re-running Phase 3X. We are moving code, then adding a new sibling crate.

- **Hard rule (carried from root PROCESS.md):** no agent in this pipeline calls the Nordnet API. General network use (rustup, crates.io, apt, doc fetches for non-Nordnet libraries) is allowed and expected. All Nordnet inputs come from saved docs. All Nordnet tests run against in-process mocks (`wiremock` for REST, `tokio::io::duplex` + `tokio::net::TcpListener` for feed). The user is responsible for any real-Nordnet-API verification after a release is produced.

## Priority order (binding)

1. **Workspace stays buildable across every commit.** PR1 is a single atomic commit because intermediate states between substeps don't compile. PR2's commits each leave the workspace green. No commit produces a state where `cargo build --workspace` fails.
2. **Spec faithfulness.** Crate boundaries, module placement, type signatures, wire-byte tests, error variants, and forward-compat policy match the design doc verbatim. Spec deviations require user escalation.
3. **No regressions in existing REST behavior.** Every test that passes before PR1 passes after PR1. Every test that passes before PR2 passes after PR2. New tests are additive.
4. **Token efficiency via subagent fan-out** where independent. PR1 cannot be split (single atomic commit). PR2's payload modules and client modules fan out cleanly.
5. **Strict typing carries over.** Newtypes for IDs that stay as IDs. `Decimal` for prices and volumes (per spec resolution). Typed enums with `Known + Unknown(String)` split for forward compat. `#[serde(deny_unknown_fields)]` on request bodies only — response/event types deliberately omit it (per design §"Forward compatibility").
6. **Static gates.** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` must pass at every commit boundary. Pre-commit hook enforces.

## What "correctness" means here

Same offline-only standard as the REST pipeline:

- **Move-equivalence (PR1).** Every public item moved from `nordnet-api` into `nordnet-model` is reachable at the new path with the same shape. The only intentional public-API change is the `ApiKeyLoginResponse` collapse (one definition wins; the loose duplicate is deleted) and the `Error::Auth(String)` → `Error::Auth(#[from] AuthError)` swap. All other call sites compile with import-only changes.
- **Wire-byte verification (PR2).** Every outbound `command` frame and every inbound `event` payload has a unit test asserting exact bytes / exact deserialization against a hand-written sample. The official `order` example payload (quoted in spec) is a required golden.
- **Forward-compat verification (PR2).** Tests exist for: unknown envelope `type` → `Event::Unknown`; unknown field on a known type → ignored, not error; unknown enum variant → `OrderState::Unknown(String)`; heartbeat with extra fields → `Heartbeat`, not `Unknown`.
- **Round-trip stability (PR2).** Every typed-enum sample re-serializes byte-equivalent. `SubscribeArgs` round-trips through `subscribe`/`unsubscribe` with deterministic field ordering (`delay: None` omitted, `delay: Some(false)` emitted as `"delay":false`).

What we do **not** guarantee, by user instruction:

- That live feed servers accept our login frame (no live login is performed).
- That a subscribe is server-accepted (the API only confirms the frame was written; rejections arrive asynchronously through `recv()`).
- That the `PrivateEvent::TradeRaw` payload schema matches the live wire (deliberately untyped for v1; spec §"Out of scope").
- That the live login flow accepts the Ed25519 signature scheme (carried forward from root PROCESS.md decision #6 update).

## Workspace layout (target after PR2)

```
nordnet-cli/                            (workspace root)
├── Cargo.toml                          (workspace; member glob `crates/*` + xtask)
├── crates/
│   ├── nordnet-model/                  (NEW — PR1 extracts here)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth.rs                 (sans loose ApiKeyLoginResponse)
│   │       ├── ids.rs
│   │       ├── error.rs                (AuthError only)
│   │       └── models/
│   │           ├── mod.rs              (GENERATED)
│   │           ├── shared.rs
│   │           ├── login.rs            (canonical ApiKeyLoginResponse + Feed)
│   │           └── <group>.rs          (accounts, instruments, orders, …)
│   ├── nordnet-api/                    (REST client, slimmed)
│   │   ├── Cargo.toml                  (depends on nordnet-model)
│   │   └── src/
│   │       ├── lib.rs                  (re-exports from nordnet-model dropped — see Decision #1)
│   │       ├── client.rs
│   │       ├── error.rs                (Error::Auth(#[from] AuthError); transport variants)
│   │       ├── pagination.rs
│   │       └── resources/
│   │           ├── mod.rs              (GENERATED)
│   │           └── <group>.rs
│   ├── nordnet-feed/                   (NEW — PR2 adds)
│   │   ├── Cargo.toml                  (depends on nordnet-model)
│   │   └── src/
│   │       ├── lib.rs                  (re-exports)
│   │       ├── codec.rs                (newline-JSON framing, 1 MiB cap)
│   │       ├── command.rs              (Login + Subscribe + Unsubscribe outbound)
│   │       ├── event.rs                (PublicEvent + PrivateEvent enums + envelope decode)
│   │       ├── public.rs               (Price, Depth, Trade, TradingStatus, Indicator, News)
│   │       ├── private.rs              (OrderEvent + nested types)
│   │       ├── public_client.rs        (PublicFeedClient)
│   │       ├── private_client.rs       (PrivateFeedClient)
│   │       └── error.rs                (FeedError + ServerError)
│   └── nordnet-cli/                    (binary, mostly unchanged — imports retargeted)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── output.rs
│           ├── config.rs
│           ├── session.rs
│           └── cmd/
│               ├── mod.rs              (GENERATED)
│               └── <cli_group>.rs
├── xtask/
│   └── src/main.rs                     (gen-mods picks up new crate dirs automatically)
├── docs/
│   └── specs/
│       ├── 2026-05-02-nordnet-feed-and-model-split-design.md
│       └── 2026-05-02-nordnet-feed-and-model-split-process.md   (this file)
├── docs-source/nordnet-api-v2.html     (unchanged)
├── CONTRACTS.md                        (amended per Decisions §1–§4 below)
└── PROCESS.md                          (root REST pipeline doc; unchanged)
```

`mod.rs` files remain generated by `cargo xtask gen-mods`. The xtask glob already picks up new `crates/*` members and new `models/` files; no xtask changes expected.

## PR plan

| PR | Goal | Phases | Commits | Atomic? |
|----|------|--------|---------|---------|
| 1 | Extract `nordnet-model` from `nordnet-api` | 0–5, sequential, single agent | 1 | yes (workspace doesn't build mid-PR) |
| 2 | Add `nordnet-feed` crate | 0–5, mixed sequential + parallel waves | 6 | no (every commit green) |

### Phase boundaries

| # | Phase | PR | Parallel? | Owns |
|---|---|---|---|---|
| 1.0 | Model crate scaffold | 1 | sequential, 1 agent (opus) | `crates/nordnet-model/{Cargo.toml,src/lib.rs}`, workspace member entry, dep injection in `nordnet-api/Cargo.toml` |
| 1.1 | File copy | 1 | sequential, same agent | duplicate `auth.rs`/`ids.rs`/`models/` into `nordnet-model`, collapse `ApiKeyLoginResponse` to one definition, define `AuthError` |
| 1.2 | Import switch | 1 | sequential, same agent | every `use nordnet_api::{auth,models,ids}` in `nordnet-api`/`nordnet-cli` flips to `nordnet_model::…`; `Error::Auth(String)` → `Error::Auth(#[from] AuthError)` |
| 1.3 | Original deletion | 1 | sequential, same agent | remove `crates/nordnet-api/src/{auth.rs,ids.rs,models/}`; regenerate `mod.rs` |
| 1.4 | Verify gate | 1 | sequential, same agent | full static gate (`fmt`, `clippy`, `test`) workspace-wide |
| 1.R | PR1 review | 1 | reviewer agent (opus) | independent diff audit + checklist |
| 2.0 | Feed crate scaffold | 2 | sequential, 1 agent (sonnet) | `crates/nordnet-feed/Cargo.toml`, `src/lib.rs`, workspace member entry, deps pinned |
| 2.1 | Codec + command | 2 | sequential, 1 agent (sonnet) | `codec.rs` (1 MiB framing) + `command.rs` (`SubscribeArgs` enum, manual `Serialize`) |
| 2.2 | Event payloads | 2 | parallel (2 agents, sonnet) | `public.rs` + `private.rs` |
| 2.3 | Clients | 2 | parallel (2 agents, sonnet) | `public_client.rs` + `private_client.rs` |
| 2.4 | Tests | 2 | parallel (matches owner of file under test) | `tests/codec_test.rs`, `tests/event_test.rs`, `tests/command_test.rs`, `tests/client_test.rs` |
| 2.R | Per-task review | 2 | parallel | review notes per implementer |
| 2.5 | Workspace integration | 2 | sequential, 1 agent (opus) | `cargo xtask gen-mods`, full workspace gate, sequential commits |

Each phase boundary is a hard gate: next phase only starts when previous passes (compile + lint + test green for PR2; for PR1, passes only at 1.4).

---

## PR 1 — Extract `nordnet-model` (atomic, single agent, opus)

Single commit. Six substeps. Workspace does not compile between 1.1 and 1.3. The reviewer audits the final state, not intermediate states.

### Why one agent / one commit

- Cross-file rename: `nordnet_api::auth::Session` → `nordnet_model::auth::Session` touches every file that imports it. Splitting across commits means each intermediate commit fails to build.
- The `ApiKeyLoginResponse` collapse changes the public type's shape (loose `Option<serde_json::Value>` feeds → typed `Vec<Feed>`). Half-applied, callers see two competing types.
- `Error::Auth(String)` → `Error::Auth(#[from] AuthError)` requires `auth.rs` and `error.rs` to land together.

### Implementer prompt template

```
You execute PR1 of the nordnet-model + nordnet-feed split.

READ ONLY:
- /Users/alfredvc/src/nordnet-cli/docs/specs/2026-05-02-nordnet-feed-and-model-split-design.md
- /Users/alfredvc/src/nordnet-cli/docs/specs/2026-05-02-nordnet-feed-and-model-split-process.md (this file)
- /Users/alfredvc/src/nordnet-cli/CONTRACTS.md
- /Users/alfredvc/src/nordnet-cli/Cargo.toml
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/Cargo.toml
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/{auth.rs,ids.rs,error.rs,client.rs,lib.rs}
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/src/models/*.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-api/tests/*.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-cli/src/**/*.rs
- /Users/alfredvc/src/nordnet-cli/crates/nordnet-cli/Cargo.toml

DO NOT CALL THE NORDNET API. General network for cargo / crates.io is fine.

PROCESS — execute strictly in order. Run `cargo build --workspace` only after step 1.4. Do NOT commit until step 1.4 passes.

1.0 SCAFFOLD
  - Create crates/nordnet-model/Cargo.toml (deps: serde, serde_json, rust_decimal, time, ed25519-dalek, ssh-key, base64, thiserror — workspace inherited).
  - Create crates/nordnet-model/src/lib.rs declaring `pub mod auth; pub mod models; pub mod ids; pub mod error;`.
  - Add `nordnet-model` to the workspace `members` list (it should be picked up by the `crates/*` glob automatically; verify).
  - In crates/nordnet-api/Cargo.toml: add `nordnet-model = { path = "../nordnet-model" }`.

1.1 COPY (do NOT delete originals yet)
  - Copy crates/nordnet-api/src/auth.rs → crates/nordnet-model/src/auth.rs.
    - Inside the copy, REMOVE the loose `ApiKeyLoginResponse` struct and its `to_session()` impl.
    - Adjust any `parse_private_key_openssh` / signature paths to return the new `nordnet_model::error::AuthError`.
  - Copy crates/nordnet-api/src/ids.rs → crates/nordnet-model/src/ids.rs verbatim.
  - Copy crates/nordnet-api/src/models/*.rs (every file, recursively) → crates/nordnet-model/src/models/.
    - In `models/login.rs`: this is the canonical `ApiKeyLoginResponse`. Move `to_session()` here from the deleted loose copy. The auth module does NOT re-export it.
  - Create crates/nordnet-model/src/error.rs containing `pub enum AuthError { InvalidKey(String), EncryptedKey, WrongAlgorithm { got: String, expected: &'static str }, KeyDataMismatch }` with `#[derive(Debug, thiserror::Error)]` and Display impls per spec.
  - Run `cargo xtask gen-mods` to regenerate models/mod.rs in the new crate.

1.2 IMPORT SWITCH
  - In crates/nordnet-api/src/error.rs: change `Auth(String)` → `Auth(#[from] nordnet_model::AuthError)`. Update its `Display` and any constructor sites.
  - In crates/nordnet-api/src/{client.rs,resources/login.rs,resources/*.rs,lib.rs}: every `use crate::{auth,models,ids}::…` → `use nordnet_model::{auth,models,ids}::…`. Drop crate-local `pub mod auth;`, `pub mod models;`, `pub mod ids;` declarations from `lib.rs`.
  - In crates/nordnet-cli/src/**/*.rs: every `use nordnet_api::{auth,models,ids}::…` → `use nordnet_model::{auth,models,ids}::…`. (HTTP-side imports — `nordnet_api::Client`, `nordnet_api::Error`, `nordnet_api::resources::*` — are unchanged.)
  - In crates/nordnet-api/tests/*.rs: same import switch.
  - In crates/nordnet-cli/Cargo.toml: add `nordnet-model = { path = "../nordnet-model" }`.

1.3 DELETE ORIGINALS
  - Remove crates/nordnet-api/src/auth.rs.
  - Remove crates/nordnet-api/src/ids.rs.
  - Remove crates/nordnet-api/src/models/ (entire directory).
  - Run `cargo xtask gen-mods` again to clean up nordnet-api's mod files.

1.4 VERIFY GATE
  - cargo fmt --check
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --workspace
  - All three must pass. If any fail, fix in place — do not commit a broken intermediate state.

1.5 NOTES
  - Write notes/PR1-model-extraction.md with: files moved, files deleted, public-API deltas (the ApiKeyLoginResponse collapse, the Error::Auth swap), test count before/after.

Return JSON when done:
{
  "status": "done" | "blocked",
  "summary": "<=80 words",
  "files_added": ["..."],
  "files_deleted": ["..."],
  "tests_before": <int>,
  "tests_after": <int>
}

Do NOT git commit. The orchestrator commits after the reviewer approves.
```

### PR1 reviewer prompt template

```
You review PR1 — extraction of nordnet-model from nordnet-api.

READ ONLY:
- The two spec/process docs.
- /Users/alfredvc/src/nordnet-cli/CONTRACTS.md
- The post-implementer state of every file under crates/nordnet-model/, crates/nordnet-api/, crates/nordnet-cli/.
- notes/PR1-model-extraction.md.
- `git diff --name-only` against the base.

CHECKLIST:
1. Crate boundary: nordnet-model has no reqwest/tokio dep. Cargo.toml grep proves it.
2. ApiKeyLoginResponse: exactly one definition exists, in models/login.rs. Search the workspace for the type name and verify count == 1 (plus any test fixtures).
3. AuthError: defined in nordnet-model/src/error.rs. Variants match spec §"nordnet-model" (InvalidKey, EncryptedKey, WrongAlgorithm{got,expected}, KeyDataMismatch). No HTTP variants.
4. Error::Auth: in nordnet-api, the variant is `Auth(#[from] nordnet_model::AuthError)`. The old `Auth(String)` is gone — grep confirms.
5. Imports: zero `use crate::{auth,models,ids}` survive in nordnet-api. Zero `use nordnet_api::{auth,models,ids}` survive anywhere. Imports route through nordnet_model.
6. Files deleted: crates/nordnet-api/src/auth.rs, ids.rs, models/ are gone. `git diff --stat` confirms.
7. mod files: not hand-edited. Re-run `cargo xtask gen-mods` and verify `git diff` empty.
8. Static gates green: fmt + clippy + test workspace-wide.
9. Test count: tests_after >= tests_before. No tests silently dropped.
10. No Nordnet API calls: grep test code for `nordnet.se` / `nordnet.no` / `api.test.nordnet.*` host literals; only allowed in spec/doc/fixture markdown.

If any check fails:
{ "status": "rework", "issues": [{"check": "...", "file": "...", "fix": "..."}], "summary": "..." }

If all pass:
{ "status": "approved", "summary": "..." }
```

### PR1 fix loop

- Reviewer returns `rework` → orchestrator re-dispatches implementer with the issue list. Max 2 rework rounds. Third = pause + escalate.
- Reviewer returns `approved` → orchestrator creates the single PR1 commit. Subject: `refactor(workspace)!: extract nordnet-model from nordnet-api`. Pre-commit hook runs the full gate.

### PR1 gate

- Reviewer `approved`.
- Single commit lands cleanly through the pre-commit hook.
- `cargo test --workspace` test count is ≥ pre-PR1 count.

---

## PR 2 — Add `nordnet-feed` crate (incremental, multi-agent, sonnet implementers + opus integrator)

Six commits. Each leaves the workspace green. Wave plan keeps the agent fan-out small (max 3 concurrent implementers).

### Phase 2.0 — Feed crate scaffold (sequential, 1 agent, sonnet)

Outputs:

- `crates/nordnet-feed/Cargo.toml` with deps: `nordnet-model`, `tokio`, `tokio-util` (with `codec` feature), `tokio-rustls`, `rustls`, `webpki-roots` (1.x line — not the deprecated 0.26 shim), `serde`, `serde_json`, `rust_decimal`, `time`, `thiserror`. **Implementer must verify pinned versions are still current at PR time** via `cargo search` (per spec §"nordnet-feed" footnote).
- `crates/nordnet-feed/src/lib.rs` declaring all eight modules from the spec layout (empty stubs allowed; phase 2.1+ fill them).
- Workspace member glob picks the new crate up automatically; verify with `cargo metadata`.
- Empty `crates/nordnet-feed/tests/` directory created.

Phase 2.0 commit subject: `feat(feed): scaffold nordnet-feed crate`.

**Gate:** `cargo build --workspace` green. No new tests yet.

### Phase 2.1 — Codec + command (sequential, 1 agent, sonnet)

Owns:

- `crates/nordnet-feed/src/codec.rs` — newline-JSON framing wrapping `tokio_util::codec::LinesCodec`. Maximum frame length **1 MiB** (1,048,576 bytes); overshoot returns `FeedError::FrameTooLarge { bytes }`.
- `crates/nordnet-feed/src/command.rs` — outbound types:
  - `pub struct LoginCommand { session_key: String, service: &'static str = "NEXTAPI" }`. Manual `Serialize` to emit `{"cmd":"login","args":{...}}`.
  - `pub enum SubscribeArgs { MarketData { kind: MarketDataKind, market: i64, identifier: String }, Indicator { market: String, identifier: String }, News { source_id: i64, delay: Option<bool> } }`. Derives `Clone + PartialEq + Eq + Hash`. Manual `Serialize` (per spec — `serde(tag)` doesn't fit because the payload differs structurally per variant).
  - `pub enum MarketDataKind { Price, Depth, Trade, TradingStatus }` — derives `Clone + Copy + PartialEq + Eq + Hash`.
  - Helper that serializes a `(cmd_name, SubscribeArgs)` pair into `{"cmd":"<name>","args":{...}}` form for both `subscribe` and `unsubscribe` calls.
- `crates/nordnet-feed/src/error.rs` — `FeedError` enum per spec (`Tls`, `Io`, `Decode { source, line }`, `Encode`, `FrameTooLarge { bytes }`, `Closed`). Plus `pub struct ServerError { msg: String, cmd: serde_json::Value }`.

**Wire-byte requirements (asserted in 2.4 tests):**
- `delay: None` ⇒ `delay` field omitted (no `null`).
- `delay: Some(false)` ⇒ `"delay":false` literal.
- Indicator with market `"SSE"` and identifier `"OMXS30"` ⇒ `{"cmd":"subscribe","args":{"t":"indicator","m":"SSE","i":"OMXS30"}}\n`.
- News with `source_id: 2`, `delay: None` ⇒ `{"cmd":"subscribe","args":{"t":"news","s":2}}\n`.
- MarketData (Price/11/"101") ⇒ `{"cmd":"subscribe","args":{"t":"price","m":11,"i":"101"}}\n`.

Phase 2.1 commit: `feat(feed): newline-JSON codec + command serialization`.

**Gate:** `cargo build --workspace` green. Tests deferred to phase 2.4 wave A.

### Phase 2.2 — Event payloads (parallel, 2 agents, sonnet)

Two implementers run concurrently. Strict file ownership.

#### Agent A — `public.rs`

Owns `crates/nordnet-feed/src/public.rs`. Defines:
- `pub struct Price` — fields per spec §"price" table. Every field except `i` and `m` is `Option<T>` (delta-friendly). `Decimal` for prices and volumes (per spec resolution — `"volume": 111.0` is wire-allowed). `i64` for timestamps and `delayed`. `#[serde(default)]` on every optional. **No** `#[serde(deny_unknown_fields)]`.
- `pub struct Depth` — `i: String`, `m: i64`, `tick_timestamp: i64` required; levels 1–5 `bid{n}` / `ask{n}` (`Decimal`), `bid_volume{n}` / `ask_volume{n}` (`Decimal`), `bid_orders{n}` / `ask_orders{n}` (`i64` — count, not volume). All level fields optional.
- `pub struct Trade` — `i`, `m`, `trade_timestamp`, `price`, `volume` required; `broker_buying`, `broker_selling`, `trade_id`, `trade_type` optional `String`.
- `pub struct TradingStatus` — `i`, `m`, `tick_timestamp`, `status: String`; `source_status`, `halted`, `orderbook_status` optional.
- `pub struct Indicator` — `i: String`, `m: String` (NOT `i64` — spec call-out, do not reuse `MarketId` newtype here), `tick_timestamp: i64` required; `last`, `high`, `low`, `close` (`Decimal`), `delayed` (`i64`) optional.
- `pub struct News` — `news_id: i64`, `lang: String`, `timestamp: i64`, `source_id: i64`, `headline: String`, `kind: String` (wire `type`, renamed via `#[serde(rename = "type")]`), `instruments: Option<Vec<i64>>`.

#### Agent B — `private.rs`

Owns `crates/nordnet-feed/src/private.rs`. Defines:
- `pub struct OrderEvent` — fields per spec §"order" table. `volume: Decimal` (wire allows `111.0`). `price: PriceWithCurrency { value: Decimal, currency: String }`. Nested types `Tradable { market_id: i64, identifier: String }`, `Validity { kind: ValidityKind, valid_until: i64 }` (rust `kind`, wire `type`), `ActivationCondition { kind: ActivationConditionKind }`. `modified: i64`. `reference: Option<String>`.
- Every wire-string field becomes a typed enum with the `Known + Unknown(String)` shape:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
  #[serde(untagged)]
  pub enum Side { Known(KnownSide), Unknown(String) }

  #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
  pub enum KnownSide { #[serde(rename = "BUY")] Buy, #[serde(rename = "SELL")] Sell }
  ```
  Apply to: `Side`, `VolumeCondition`, `ValidityKind`, `ActivationConditionKind`, `OrderState`, `ActionState`, `OrderType`. Initial `Known` variants come from `docs-source/nordnet-api-v2.html` (treat as source of truth — agent reads only the relevant slice).

**Constraint for both agents:** no `#[serde(deny_unknown_fields)]` on any event struct. Use `#[serde(default)]` for every optional field. Forward-compat policy is binding.

Phase 2.2 commit: `feat(feed): public + private event payload types`. (One commit covers both files; the parallel split is purely an authoring optimization.)

**Gate:** `cargo build --workspace` green. Unit tests for typed-enum round-trip arrive in phase 2.4 wave B.

### Phase 2.3 — Clients (parallel, 2 agents, sonnet)

Two implementers, strict file ownership, may run concurrent with phase 2.4 wave A's test agents.

#### Agent C — `public_client.rs`

Owns `crates/nordnet-feed/src/public_client.rs`. Defines `PublicFeedClient` per spec §"Public API surface". Methods: `connect(feed: &nordnet_model::models::login::Feed)`, `login(&mut self, session_key: &str)`, `subscribe(&mut self, args: SubscribeArgs)`, `unsubscribe(&mut self, args: SubscribeArgs)`, `recv(&mut self) -> Result<Option<PublicEvent>, FeedError>`. `connect` honors `feed.encrypted` for the TLS branch (port heuristic explicitly rejected — see Decision §3 below). `login` is fire-and-forget — writes the frame, returns `Ok(())`. `recv` returns `Ok(None)` on clean EOF, `Err(FeedError::Closed)` on mid-frame disconnect.

Also owns `crates/nordnet-feed/src/event.rs` — `pub enum PublicEvent { Heartbeat, Error(ServerError), Price(public::Price), Depth(public::Depth), Trade(public::Trade), TradingStatus(public::TradingStatus), Indicator(public::Indicator), News(public::News), Unknown { kind: String, data: serde_json::Value } }`. Manual envelope decode that switches on `type` and falls through to `Unknown`. `Heartbeat` accepts `data: {…anything…}` via `#[serde(default)]` and ignores extra fields (does NOT fall through to `Unknown`).

#### Agent D — `private_client.rs`

Owns `crates/nordnet-feed/src/private_client.rs`. Defines `PrivateFeedClient` per spec. No `subscribe` / `unsubscribe` methods (private feed is auto-push after login).

Also owns the `PrivateEvent` enum within `event.rs` (extends Agent C's file, but the variants are private-only — no name collision). Variants: `Heartbeat`, `Error(ServerError)`, `Order(private::OrderEvent)`, `TradeRaw(serde_json::Value)`, `Unknown { kind: String, data: serde_json::Value }`.

**Coordination note:** `event.rs` is co-edited by Agents C and D within phase 2.3. To avoid the file-ownership conflict, Agent C writes the `PublicEvent` enum first; Agent D then appends `PrivateEvent` (file is opened after C lands its diff). Sequenced inside phase 2.3, not parallel within `event.rs` specifically.

Phase 2.3 commit: `feat(feed): public + private client types`.

**Gate:** `cargo build --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` green.

### Phase 2.4 — Tests (parallel, up to 3 agents, sonnet)

Test files live under `crates/nordnet-feed/tests/`. Three test agents, parallel, strict file ownership.

#### Test agent A — `tests/codec_test.rs` + `tests/command_test.rs`

- Codec round-trip across `tokio::io::duplex`: write 3 frames terminated by `\n`, read 3 events. Frames at exactly 1 MiB pass; frames at 1 MiB + 1 byte produce `FeedError::FrameTooLarge`.
- Command serialization wire-byte assertions (every variant from spec §"Subscribe / unsubscribe" worked-bytes block). `delay: None` vs `delay: Some(false)` distinction asserted.
- Login frame wire-byte assertion: `{"cmd":"login","args":{"session_key":"K","service":"NEXTAPI"}}\n`.

#### Test agent B — `tests/event_test.rs`

- Golden JSON deserialization for every `PublicEvent` variant (price, depth, trade, trading_status, indicator, news) using one full + one delta sample where applicable.
- Golden JSON deserialization for `PrivateEvent::Order` using the official sample payload quoted in spec §"order" (this is a required golden).
- Forward-compat tests:
  - Unknown envelope `type` → `PublicEvent::Unknown { kind, data }`.
  - Unknown field on a known type → ignored, deserializes successfully.
  - Heartbeat with `data: { extra: 1 }` → `Heartbeat`, NOT `Unknown`.
  - Typed-enum unknown variant: `{"order_state":"FUTURE_VALUE"}` → `OrderState::Unknown("FUTURE_VALUE")`. Round-trip back to JSON byte-equivalent.
  - Validity rename: `{"type":"DAY"}` deserializes into `validity.kind = ValidityKind::Known(Day)`; reserialize matches input.
- Server `err` frame surfaces as `Event::Error(ServerError)` via `recv()`, not as `Result::Err`.

#### Test agent C — `tests/client_test.rs`

- Subscribe → receive: mocked server (`tokio::io::duplex`) accepts a `subscribe` frame, replies with a `price` tick; `PublicFeedClient::recv` parses correctly.
- Plain-TCP (`encrypted = false`) path: `tokio::net::TcpListener` bound to `127.0.0.1`, client connects, exchanges a frame.
- Login error mid-subscribe sequence: client sends login + 3 subscribes, server replies with one `err` frame and closes; verify `err` arrives as `Event::Error` and the next `recv()` returns `Ok(None)`. Documents the frame-ordering caveat from spec §"Login command".
- Mid-frame disconnect: server closes after writing `{"type":"price",` (no terminator) → next `recv()` returns `Err(FeedError::Closed)` on RST or `Err(FeedError::Decode { .. })` on a clean FIN with partial data. Test relaxed to accept either since loopback shutdown is FIN, not RST.

**TLS path is NOT live-tested.** TLS handshake against a real Nordnet host would violate the no-Nordnet-API rule. The `encrypted = true` branch is exercised only at the type level (`PublicFeedClient::connect` compiles for both branches; the TLS code path is covered by the upstream `tokio-rustls` test suite, not by this crate).

Phase 2.4 commit: `test(feed): codec, command, event, client coverage`.

**Gate:** all four static gates green; new test count = pre-2.4 + new tests added.

### Phase 2.5 — Workspace integration (sequential, 1 agent, opus)

The closer.

1. `cargo xtask gen-mods` — should be a no-op if 2.0–2.4 ran correctly. Any diff = bug; investigate before proceeding.
2. `cargo fmt --check` workspace.
3. `cargo clippy --workspace --all-targets -- -D warnings`.
4. `cargo test --workspace`.
5. If anything is yellow/red, fix in place. Do not commit a broken closing state.
6. Final commit: `chore(feed): regenerate mod files + finalize crate`.

Phase 2.5 also produces `notes/PR2-feed-crate.md` with: ops/events implemented, deviations from the spec (none expected), test count delta.

**Phase 2.5 gate:** all four static gates green workspace-wide. Pipeline ends here.

### PR2 reviewer prompts (per phase)

Each implementer phase (2.1, 2.2 ×2, 2.3 ×2, 2.4 ×3) gets a paired reviewer agent dispatched after the implementer returns `done`. Reviewer prompt body:

```
You review subagent output for nordnet-feed phase <PHASE>.

READ ONLY:
- The two spec/process docs.
- The files written by this implementer (`git diff --name-only` against the previous commit).
- /Users/alfredvc/src/nordnet-cli/CONTRACTS.md.
- The implementer's phase notes.

CHECKLIST (subset per phase — apply only those that fit):
1. File scope: implementer touched only owned files. `git diff --name-only` matches the allowed list.
2. Spec faithfulness: every type signature / wire byte / field name / optionality flag matches the design doc verbatim. Cite spec line numbers in any flagged deviation.
3. Forward-compat: no `deny_unknown_fields` on event/response types. `#[serde(default)]` on every optional event field.
4. Newtype + numeric discipline: `Decimal` for prices and volumes, `i64` for counts/ids/timestamps, no `f64` anywhere.
5. Typed-enum shape: every wire-string enum follows `Known + Unknown(String)` with `#[serde(untagged)]` on the outer enum. Variants match `docs-source/nordnet-api-v2.html`.
6. Wire-byte tests cover the exact patterns from spec (delay omitted vs explicit-false; indicator's `m: String`; news's `s` field).
7. No Nordnet host literals: grep test code for `nordnet.se` / `nordnet.no`; only spec/doc paths may contain them.
8. Static gates green for the affected crate.
9. mod files: not hand-edited; `cargo xtask gen-mods` produces empty diff.

If any check fails:
{ "status": "rework", "issues": [{"check": "...", "file": "...", "line": "...", "fix": "..."}], "summary": "..." }

If all pass:
{ "status": "approved", "summary": "..." }
```

### PR2 fix loop

- Reviewer returns `rework` → orchestrator re-dispatches the same implementer with the issue list. Max 2 rework rounds per phase. Third = pause + escalate.
- Reviewer returns `approved` → orchestrator advances to the next phase or commits, as applicable.

---

## Conflict elimination

| Risk | Mitigation |
|---|---|
| Two implementers edit the same file | File ownership matrix per phase. `git diff --name-only` enforced by reviewer. Phase 2.3's `event.rs` shared by Agents C and D is sequenced inside the phase, not run in parallel for that file. |
| `mod.rs` hand-edits | Generated by `cargo xtask gen-mods`. Pre-commit hook regenerates and fails on diff. PR1 substep 1.3 + PR2 phase 2.5 both call gen-mods explicitly. |
| `Cargo.toml` workspace conflicts | Workspace `Cargo.toml` uses `members = ["crates/*", "xtask"]` glob. New crate dirs are picked up automatically. Per-crate Cargo.toml is owned by the scaffold phase that creates the crate; later phases append deps via Edit, not rewrite. |
| Cross-PR collisions | PR1 lands fully (single commit) before PR2 starts. PR2's `nordnet-feed` doesn't touch `nordnet-model` or `nordnet-api`. |
| Half-applied refactors | PR1 is atomic. PR2 commits each leave the workspace green; pre-commit hook rejects red commits. |
| Subagent reaches for the Nordnet API | Allowlist denies Nordnet hosts; reviewer greps for host literals. |
| Subagent reads outside its slice | Implementer prompt enumerates exact paths to read. Reviewer can flag if the implementer's notes show wider reads. |

## Definition of done (PR1 + PR2)

- `nordnet-model` crate exists, has zero HTTP/transport deps, and contains every type previously in `nordnet-api::{auth,models,ids}`.
- `nordnet-feed` crate exists with the eight modules from the spec layout.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- `cargo test --workspace` green; test count ≥ pre-PR1 count + new feed tests added by phase 2.4.
- Wire-byte tests cover every documented `subscribe` and `login` frame variant.
- Forward-compat tests cover unknown envelope `type`, unknown fields, unknown enum variants, and heartbeat-with-extras.
- The official `order` example payload deserializes losslessly into `private::OrderEvent` and re-serializes byte-equivalent (modulo field-order normalization).
- `notes/PR1-model-extraction.md` and `notes/PR2-feed-crate.md` exist and summarize the work.
- README / AGENTS.md (if present) acknowledge the new crate split and the no-live-API verification rule.

Live-API verification (login flow + feed handshake) remains the user's responsibility, performed outside this pipeline.

## Locked decisions

1. **Crate names: `nordnet-model`, `nordnet-feed`.** Matches twilight precedent. Not `nordnet-types` or `nordnet-core`.
2. **PR1 atomicity.** Single commit. Six substeps inside that commit. The workspace does not build between substeps; do not split.
3. **TLS gate via structured field.** `connect` honors `feed.encrypted: bool` from the login response. The Python reference impl's `port == 443` heuristic is rejected — it coincides today but trusting the typed wire field is more robust per spec §"Connection".
4. **Login is fire-and-forget.** No `LoginRejected` error variant. Server errors arrive via `recv()` as `Event::Error`. Recommended deterministic-detection pattern is documented on `login()`'s rustdoc but not enforced.
5. **Frame size cap: 1 MiB.** Designer choice; Nordnet docs do not specify. Configurable in a future revision.
6. **`SubscribeArgs` is an enum with three variants.** `MarketData`, `Indicator`, `News`. `delay` field appears **only** on `News` per Nordnet's docs. Manual `Serialize` impl. Derives `Clone + PartialEq + Eq + Hash` for unsubscribe symmetry.
7. **`MarketDataKind` is a separate enum** (`Price`, `Depth`, `Trade`, `TradingStatus`). Lives inside `command.rs`. Keeps the wire-`t` value compile-time-correct without exposing `Indicator`/`News` as choices in the wrong place.
8. **`Decimal` for prices and volumes; `i64` for counts/ids/timestamps.** Server is willing to send fractional-as-float (`"volume": 111.0`); `i64` chokes on that. `Decimal` round-trips both `111` and `111.0`.
9. **Indicator's `m` is `String`, not `i64`.** Do NOT reuse `MarketId` newtype for `Indicator`. Per spec §"indicator".
10. **Typed-enum forward compat.** Every wire-string enum field on `OrderEvent` (`Side`, `VolumeCondition`, `ValidityKind`, `ActivationConditionKind`, `OrderState`, `ActionState`, `OrderType`) uses the `Known + Unknown(String)` split with `#[serde(untagged)]` on the outer enum. Initial `Known` variants from `docs-source/nordnet-api-v2.html`.
11. **No `deny_unknown_fields` on event/response payloads.** Forward-compat rule from Nordnet's docs is binding for v1. Request bodies (`LoginCommand`, etc.) keep `deny_unknown_fields` to catch our own bugs.
12. **`PrivateEvent::TradeRaw(serde_json::Value)` is deliberately untyped for v1.** Schema not in public Nordnet docs; Go-model schema is third-party and risky. Type in a follow-up after a live sample lands. The `Raw` suffix is the in-API signal that this is the only payload without a typed struct.
13. **No shared `FeedClient` trait between `PublicFeedClient` and `PrivateFeedClient` for v1.** Speculative now; revisit when reconnect lands.
14. **No CLI integration in v1.** `nordnet-feed` is library-only. `nordnet-cli` does not gain a `feed` subcommand in this pipeline.
15. **`ApiKeyLoginResponse` collapse.** The typed version in `models/login.rs` wins. The loose `auth.rs` copy with `Option<serde_json::Value>` feeds is deleted in PR1 substep 1.1. `to_session()` moves to `models/login.rs`.
16. **`Error::Auth` swap.** `Auth(String)` → `Auth(#[from] nordnet_model::AuthError)` in `nordnet-api::Error`. Lands inside PR1 (atomic).
17. **`webpki-roots` 1.x line.** Pin the `1.x` line directly. The `0.26` line is the deprecated semver-trick shim.
18. **Per spec §"nordnet-feed" footnote — implementer re-verifies pinned versions.** `tokio-rustls`, `rustls`, `webpki-roots`, `tokio-util` deps are confirmed against `cargo search` at PR2 phase 2.0 time, not at spec write time.

## Pipeline state log

| Phase | Status | Commit | Notes |
|---|---|---|---|
| PR1.0 Scaffold | not started | — | — |
| PR1.1 Copy | not started | — | — |
| PR1.2 Import switch | not started | — | — |
| PR1.3 Delete originals | not started | — | — |
| PR1.4 Verify gate | not started | — | — |
| PR1.R Review | not started | — | — |
| PR1 commit | not started | — | Subject: `refactor(workspace)!: extract nordnet-model from nordnet-api` |
| PR2.0 Feed crate scaffold | not started | — | — |
| PR2.1 Codec + command | not started | — | — |
| PR2.2 Event payloads (parallel A/B) | not started | — | — |
| PR2.3 Clients (parallel C/D) | not started | — | — |
| PR2.4 Tests (parallel A/B/C) | not started | — | — |
| PR2.5 Workspace integration | not started | — | — |
