# PR2 — nordnet-feed crate

## Phase 2.0 — Scaffold

**Date:** 2026-05-02

### Dep versions chosen (verified via `cargo search` + `cargo info`)

| Dep | cargo search result | Latest stable | Line pinned |
|-----|---------------------|---------------|-------------|
| `tokio-rustls` | `0.26.4` | `0.26.4` | `"0.26"` |
| `rustls` | `0.24.0-dev.0` (pre-release); stable is `0.23.40` | `0.23.40` | `"0.23"` |
| `webpki-roots` | `1.0.7` | `1.0.7` | `"1"` |
| `tokio-util` | `0.7.18` | `0.7.18` | `"0.7"` |

**Note on rustls:** `cargo search rustls --limit 1` returned `0.24.0-dev.0`, which is a
pre-release on crates.io. `cargo info rustls` confirmed the latest *stable* release is
`0.23.40`. The `0.26.x` line of `tokio-rustls` depends on `rustls 0.23`, so pinning
`"0.23"` is the correct choice for compatibility and correctness. The `0.24` pre-release
was ignored per the spec's intent to pin current stable.

### Workspace Cargo.toml changes

Added to `[workspace.dependencies]` under the "HTTP / async" block:
- `tokio-rustls = "0.26"`
- `tokio-util = { version = "0.7", features = ["codec"] }`

Added a new "TLS" block:
- `rustls = "0.23"`
- `webpki-roots = "1"`

Added `"crates/nordnet-feed"` to `members` (alphabetically between `nordnet-cli` and `nordnet-model`).

### Files created

- `crates/nordnet-feed/Cargo.toml` — crate manifest with all required deps
- `crates/nordnet-feed/src/lib.rs` — declares 8 modules (stubs)
- `crates/nordnet-feed/src/codec.rs` — stub
- `crates/nordnet-feed/src/command.rs` — stub
- `crates/nordnet-feed/src/error.rs` — stub
- `crates/nordnet-feed/src/event.rs` — stub
- `crates/nordnet-feed/src/private.rs` — stub
- `crates/nordnet-feed/src/private_client.rs` — stub
- `crates/nordnet-feed/src/public.rs` — stub
- `crates/nordnet-feed/src/public_client.rs` — stub
- `crates/nordnet-feed/tests/` — empty directory

### Deviations from spec template

None. The spec template had tokio features `["macros", "rt-multi-thread", "time"]` in the
workspace dep and used option (a) to add `["io-util", "net"]` only in the feed crate's
dep entry. That is exactly what was implemented. The workspace tokio dep is unchanged.

### Gate results

- `cargo build --workspace` — green
- `cargo metadata --format-version 1 | grep nordnet-feed` — confirmed membership
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green

---

## Phase 2.1 — Codec + command

**Date:** 2026-05-02

### Files implemented

- `crates/nordnet-feed/src/error.rs` — `FeedError` enum (Tls, Io, Decode, Encode,
  FrameTooLarge, Closed) via thiserror. `ServerError` struct (msg, cmd) without
  std::error::Error — surfaced as an event payload, not a Rust error.
- `crates/nordnet-feed/src/codec.rs` — `MAX_FRAME_BYTES = 1 << 20` (1 MiB constant)
  and `new_lines_codec()` constructor returning `LinesCodec::new_with_max_length(MAX_FRAME_BYTES)`.
- `crates/nordnet-feed/src/command.rs` — `LoginCommand<'a>`, `SubscribeArgs`,
  `MarketDataKind`, `encode_subscribe_frame`, `encode_login_frame`. All serialization
  uses `SerializeMap` directly (not `serde_json::Map` or `json!` macro) to guarantee
  insertion-order field emission.
- `crates/nordnet-feed/src/lib.rs` — added `pub use` re-exports for `LoginCommand`,
  `MarketDataKind`, `SubscribeArgs`, `FeedError`, `ServerError`.

### Key discovery: serde_json field ordering

`serde_json` without the `preserve_order` feature uses `BTreeMap` internally for
both the `json!` macro and `serde_json::Map`, sorting keys alphabetically. This means:
- `serde_json::json!({"cmd": ..., "args": ...})` would emit `{"args":{...},"cmd":"..."}` 
  (args before cmd alphabetically).
- `serde_json::Map::insert("session_key", ...).insert("service", ...)` would emit 
  `service` before `session_key` alphabetically.

**Fix:** All serialization uses `SerializeMap::serialize_entry` directly, which writes
fields in the order called and never passes through an intermediate map. Helper
structs (`LoginArgs`, `SubscribeFrame`) wrap the sub-objects to keep the Serialize
impls composable.

### Wire-byte verification

Added inline `#[cfg(test)]` tests in `command.rs` asserting exact JSON output for
all 6 required variants (login, MarketData price, Indicator, News/no-delay,
News/delay=false, News/delay=true) plus unsubscribe symmetry and the cmd-before-args
regression guard. All 8 tests pass.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --workspace` — 255 tests pass (247 pre-existing + 8 new wire-byte
  inline tests in command.rs)

---

## Phase 2.2 Agent A — public.rs

**Date:** 2026-05-02

### What landed

Implemented `crates/nordnet-feed/src/public.rs` with six public structs:

- `Price` — full price tick; all fields except `i` (String) and `m` (i64) are
  `Option<T>` with `#[serde(default)]` for delta compatibility. Decimal for all
  prices and volumes. Derives `Default` so consumers can build a "starting state"
  they then merge into.
- `Depth` — order book depth tick; `i`, `m`, `tick_timestamp` required; levels 1–5
  (`bid{n}`, `ask{n}` as Decimal; `bid_volume{n}`, `ask_volume{n}` as Decimal;
  `bid_orders{n}`, `ask_orders{n}` as i64 counts). Derives `Default`.
- `Trade` — market trade tick; `i`, `m`, `trade_timestamp`, `price`, `volume`
  required; `broker_buying`, `broker_selling`, `trade_id`, `trade_type` optional.
- `TradingStatus` — `i`, `m`, `tick_timestamp`, `status` required; `source_status`,
  `halted`, `orderbook_status` optional.
- `Indicator` — `m` is `String` (NOT i64 — per spec Decision §9); `i`, `tick_timestamp`
  required; `last`, `high`, `low`, `close` (Decimal), `delayed` (i64) optional.
- `News` — wire `type` field renamed to Rust `kind` via `#[serde(rename = "type")]`;
  `instruments: Option<Vec<i64>>` with `#[serde(default)]`.

### Binding constraints honored

- No `#[serde(deny_unknown_fields)]` on any struct (forward-compat rule).
- Every optional field has `#[serde(default)]`.
- `Decimal` for all prices/volumes; `i64` for counts, ids, timestamps. No f64.
- `Indicator.m` is `String`, not the `MarketId` newtype.
- `Price` and `Depth` derive `Default`; other structs do not (they have required fields).
- No `Hash` derives (Decimal is not Hash by default; unneeded).
- `lib.rs` unchanged — `pub mod public;` was already present from phase 2.0 scaffold.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --workspace` — 255 tests pass (unchanged; phase 2.4 owns event tests)

---

## Phase 2.2 Agent B — private.rs

**Date:** 2026-05-02

### What landed

Implemented `crates/nordnet-feed/src/private.rs` with `OrderEvent` and all nested types:

**Structs:**
- `OrderEvent` — all required fields from the spec table; `reference: Option<String>` with `#[serde(default)]`.
- `Tradable` — `market_id: i64`, `identifier: String`.
- `PriceWithCurrency` — `value: Decimal`, `currency: String`.
- `Validity` — `kind: ValidityKind` (wire `type` renamed via `#[serde(rename = "type")]`), `valid_until: i64`.
- `ActivationCondition` — `kind: ActivationConditionKind` (wire `type` renamed via `#[serde(rename = "type")]`).

**Typed enums (Known + Unknown split — Decision §10):** Seven enums total.

| Outer enum | Inner `Known*` enum | Known variants (HTML source + seed) |
|------------|--------------------|------------------------------------|
| `Side` | `KnownSide` | BUY, SELL |
| `VolumeCondition` | `KnownVolumeCondition` | NORMAL, ALL_OR_NOTHING, AON, FOK, IOC |
| `ValidityKind` | `KnownValidityKind` | DAY, UNTIL_DATE, EXTENDED_HOURS, IMMEDIATE, GTC, GTD, IOC |
| `ActivationConditionKind` | `KnownActivationConditionKind` | NONE, MANUAL, STOP_ACTPRICE_PERC, STOP_ACTPRICE |
| `OrderState` | `KnownOrderState` | DELETED, LOCAL, ON_MARKET, LOCKED, ACTIVE, FILLED, CANCELLED |
| `ActionState` | `KnownActionState` | DEL_FAIL, DEL_PEND, DEL_CONF, DEL_PUSH, INS_FAIL, INS_PEND, INS_CONF, INS_STOP, MOD_FAIL, MOD_PEND, MOD_PUSH, INS_WAIT, MOD_WAIT, DEL_WAIT, MOD_CONF, ACKED |
| `OrderType` | `KnownOrderType` | FAK, FOK, NORMAL, LIMIT, STOP_LIMIT, STOP_TRAILING, OCO, MARKET, STOP_LOSS |

### Extra variants found beyond the spec seed list

From `docs-source/nordnet-api-v2.html` (source of truth):

| Enum | Extra variants from HTML (not in seed) |
|------|----------------------------------------|
| `VolumeCondition` | `ALL_OR_NOTHING` |
| `ValidityKind` | `UNTIL_DATE`, `EXTENDED_HOURS`, `IMMEDIATE` |
| `ActivationConditionKind` | `MANUAL`, `STOP_ACTPRICE_PERC`, `STOP_ACTPRICE` |
| `OrderState` | `DELETED`, `ON_MARKET`, `LOCKED` |
| `ActionState` | `DEL_FAIL`, `DEL_CONF`, `DEL_PUSH`, `INS_FAIL`, `INS_CONF`, `INS_STOP`, `MOD_FAIL`, `MOD_PUSH`, `INS_WAIT`, `MOD_WAIT`, `DEL_WAIT`, `MOD_CONF` |
| `OrderType` | `FAK`, `NORMAL`, `STOP_TRAILING`, `OCO` |

Variants only in the seed (not in HTML REST docs, feed-wire only): `AON`, `FOK`, `IOC` (VolumeCondition), `GTC`, `GTD` (ValidityKind), `ACTIVE`, `FILLED`, `CANCELLED` (OrderState), `ACKED` (ActionState), `MARKET`, `STOP_LOSS` (OrderType).

### Binding constraints honored

- No `#[serde(deny_unknown_fields)]` on any struct (forward-compat rule).
- `#[serde(default)]` on optional field (`reference`).
- `Decimal` for `volume` and `price.value`; `i64` for `order_id`, `accno`, `accid`, `modified`, `valid_until`, `market_id`. No f64.
- No `Hash` derives (Decimal is not Hash).
- All derives include `Eq` (Decimal implements Eq; confirmed via crates.io).
- `lib.rs` unchanged — `pub mod private;` was already present from phase 2.0 scaffold.
- Local types only — no imports from `nordnet_model`.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --workspace` — 255 tests pass (unchanged; phase 2.4 owns event tests)
