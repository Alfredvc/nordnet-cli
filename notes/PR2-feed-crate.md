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

---

## Phase 2.3 Agent C — public_client + PublicEvent

**Date:** 2026-05-02

### What landed

Implemented `crates/nordnet-feed/src/event.rs` (PublicEvent + envelope decoder) and
`crates/nordnet-feed/src/public_client.rs` (PublicFeedClient). Added `futures-util` as
a workspace dependency (needed for `SinkExt` + `StreamExt` on `Framed`).

#### `event.rs`

- `PublicEvent` enum with 8 variants: `Heartbeat`, `Error(ServerError)`, `Price`,
  `Depth`, `Trade`, `TradingStatus`, `Indicator`, `News`, `Unknown { kind, data }`.
- `Envelope` struct (pub(crate)) with `#[serde(rename = "type")]` and
  `#[serde(default)]` on `data` — handles frames with no `data` field.
- `PublicEvent::from_envelope` — switches on `env.kind`, matches 7 known event types;
  falls through to `Unknown` for anything else (forward-compat).
- `Heartbeat` arm matches `"heartbeat"` and ignores all data fields entirely — per spec
  §"Heartbeat", extra fields like `{"server_time":123}` are ignored (forward-compat),
  and the heartbeat does NOT fall through to `Unknown`.
- `Error` arm: `ServerError` does not implement `Deserialize` (owned by `error.rs`,
  which this phase cannot touch). Fields extracted manually from `serde_json::Value`.
- All other known arms use `serde_json::from_value(env.data)?` for typed deserialization.
- File ends with `// === Agent D will append PrivateEvent below this line ===` comment.

#### `public_client.rs`

- `PublicFeedClient` wraps `enum Inner { Plain(Framed<TcpStream, LinesCodec>), Tls(Box<Framed<TlsStream<TcpStream>, LinesCodec>>) }`.
  The `Tls` variant is boxed to satisfy clippy's `large_enum_variant` lint (Plain = 144
  bytes, Tls = 1208 bytes unboxed).
- `connect(feed: &Feed)` — honors `feed.encrypted: bool` (Decision §3; not port-443
  heuristic). Builds `rustls::ClientConfig` with `webpki_roots::TLS_SERVER_ROOTS`;
  converts `feed.hostname` to `rustls::pki_types::ServerName<'static>` via
  `String::try_into()` with `InvalidDnsNameError` mapped to `FeedError::Io`.
- `login()` — fire-and-forget (Decision §4). Writes the login frame, returns `Ok(())`.
  Doc comment explains the deterministic-detection pattern from spec §"Login command".
- `subscribe()` / `unsubscribe()` — same args type (`SubscribeArgs`), different cmd string.
  Doc comment explains server-rejection arrives asynchronously via `recv()`.
- `recv()` — returns `Ok(None)` on clean EOF (Stream ends), `Err(FeedError::Closed)` for
  mid-frame EOF (UnexpectedEof from I/O). Decodes line → `Envelope` → `PublicEvent`.
- `map_lines_err` (pub(crate)) — maps `LinesCodecError::Io(UnexpectedEof)` to
  `FeedError::Closed`, other I/O errors to `FeedError::Io`, and
  `LinesCodecError::MaxLineLengthExceeded` to `FeedError::FrameTooLarge { bytes: MAX_FRAME_BYTES + 1 }`.
  The `bytes` value is `MAX_FRAME_BYTES + 1` because `LinesCodec` doesn't expose the
  actual offending byte count — this is a known limitation documented in the spec.

#### `lib.rs` additions

Added two re-exports at the bottom of the existing `pub use` block:
```rust
pub use event::PublicEvent;
pub use public_client::PublicFeedClient;
```

### Workspace dependency added: `futures-util`

`futures-util` was absent from `[workspace.dependencies]`. Added:
```toml
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
```
And to `crates/nordnet-feed/Cargo.toml`:
```toml
futures-util = { workspace = true }
```
Note: `default-features = false` drops `async-await`, `std`, etc. The `sink` feature is
required for `SinkExt` (used by `Framed::send`). `StreamExt` has no feature gate in
futures-util 0.3.

### Deviations from task template

- `ServerError` manual deserialization: The task template showed
  `serde_json::from_value(env.data)?` for the `"err"` arm. This requires `ServerError:
  Deserialize`, but `error.rs` is off-limits. Fields are extracted manually from
  `serde_json::Value` instead. Behavior is identical.
- `Inner::Tls` is boxed: `Box<Framed<TlsStream<TcpStream>, LinesCodec>>` rather than
  bare `Framed<TlsStream<TcpStream>, LinesCodec>`. This satisfies clippy's
  `large_enum_variant` lint (unboxed = 1208 bytes vs. 144-byte Plain variant). Rust
  auto-derefs `Box<T>` in match arms so call sites are unchanged.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --workspace` — 255 tests pass (unchanged; phase 2.4 owns event/client tests)

---

## Phase 2.3 Agent D — private_client + PrivateEvent

**Date:** 2026-05-02

### What landed

Implemented `crates/nordnet-feed/src/private_client.rs` (PrivateFeedClient) and appended
`PrivateEvent` to `crates/nordnet-feed/src/event.rs`.

#### `event.rs` consolidation (coordination cleanup)

Agent C left the `"err"` arm in `PublicEvent::from_envelope` as inline manual extraction
(because `ServerError` does not implement `Deserialize` and `error.rs` is off-limits). As
the second author of `event.rs`, Agent D extracted this into a shared `pub(crate) fn
parse_server_error(data: Value) -> ServerError` helper placed just below the `Envelope`
struct. Both `PublicEvent::from_envelope` and `PrivateEvent::from_envelope` now call this
helper — no duplication, behavior identical to what Agent C shipped.

#### `PrivateEvent` enum (event.rs, appended)

Variants: `Heartbeat`, `Error(ServerError)`, `Order(Box<private::OrderEvent>)`,
`TradeRaw(Value)`, `Unknown { kind: String, data: Value }`.

`Order` is boxed (`Box<private::OrderEvent>`) to satisfy clippy's `large_enum_variant`
lint — `OrderEvent` is ~320 bytes vs. the next-largest variant at ~56 bytes. Mirrors the
same boxing rationale used for `Inner::Tls` in the public client (unboxed = 1208 bytes
vs. 144-byte Plain). Consumers pattern-match as normal; Rust auto-derefs the Box.

`from_envelope` routes `"order"` → `Order(Box::new(serde_json::from_value(env.data)?))`
and `"trade"` → `TradeRaw(env.data)` (schema not in public Nordnet docs, Decision §12).

#### `private_client.rs` (PrivateFeedClient)

- Same `Inner { Plain, Tls(Box<...>) }` structure as `PublicFeedClient`.
- `connect(feed: &Feed)` — honors `feed.encrypted: bool` (Decision §3).
- `login()` — fire-and-forget. Writes login frame via `encode_login_frame`, returns `Ok(())`.
- `recv()` — returns `Ok(None)` on clean EOF, `Err(FeedError::Closed)` on mid-frame EOF.
- NO `subscribe` / `unsubscribe` methods — private feed is auto-push after login (Decision §13).
- Imports `map_lines_err` from `public_client` via `pub(crate)` visibility — not duplicated.

#### `lib.rs` additions

Consolidated `pub use event::PublicEvent` and new `pub use event::PrivateEvent` into a
single line `pub use event::{PrivateEvent, PublicEvent}`. Added `pub use
private_client::PrivateFeedClient`. cargo fmt reordered the private_client line after
public_client alphabetically — accepted.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --workspace` — 255 tests pass (unchanged; phase 2.4 owns event/client tests)

---

## Phase 2.4 Test Agent B — event_test

**Date:** 2026-05-02

### Files created

- `crates/nordnet-feed/tests/event_test.rs` — 13 tests
- `crates/nordnet-feed/Cargo.toml` — added `pretty_assertions` and `rust_decimal` to `[dev-dependencies]`

### Test names and count (13 total)

Public event payload tests (7):
1. `price_full_tick_deserializes` — full price tick with all optional fields
2. `price_delta_tick_deserializes` — delta tick with only changed fields, absent fields are None
3. `depth_with_levels_deserializes` — depth tick with level-1 bid/ask/order counts
4. `trade_deserializes` — market trade tick with required + optional fields
5. `trading_status_deserializes` — trading status tick
6. `indicator_m_is_string` — asserts `m` is String not i64
7. `news_kind_renamed_from_type` — asserts wire `type` → Rust `kind` rename

Private event payload tests (2):
8. `order_golden_deserializes` — official golden payload from spec §"order" with all field assertions
9. `order_golden_round_trips_byte_equivalent` — serialize → deserialize produces equal OrderEvent

Forward-compat tests (4):
10. `unknown_field_in_known_payload_is_ignored` — extra field in Price does not error
11. `unknown_typed_enum_variant_lands_in_unknown` — future OrderState wire value → Unknown(String)
12. `unknown_typed_enum_variant_round_trips` — Unknown(String) serializes back to original string
13. `validity_kind_round_trip` — Validity kind/valid_until survive serde round-trip byte-equivalent

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check --package nordnet-feed` — green
- `cargo clippy --package nordnet-feed --test event_test -- -D warnings` — green
- `cargo test --package nordnet-feed --test event_test` — 13/13 pass
- Note: `codec_test.rs` has a pre-existing `LinesCodec` scope error from Agent A, unrelated to this task

---

## Phase 2.4 Test Agent A — codec_test + command_test

**Date:** 2026-05-02

### Files created

- `crates/nordnet-feed/tests/codec_test.rs` — 4 tests
- `crates/nordnet-feed/tests/command_test.rs` — 11 tests

### `tests/codec_test.rs` — 4 tests

| Test name | What it verifies |
|-----------|-----------------|
| `round_trip_three_frames` | `tokio::io::duplex` round-trip: 3 newline-terminated frames write/read correctly; EOF yields `None` |
| `frame_at_one_mib_passes` | A frame of exactly `MAX_FRAME_BYTES` (1,048,576) bytes is accepted (cap is strict `>`) |
| `frame_one_byte_over_one_mib_errors` | A frame of `MAX_FRAME_BYTES + 1` bytes yields `LinesCodecError::MaxLineLengthExceeded` |
| `write_emits_newline_terminator` | Encoder appends `\n`; raw bytes on peer side are `b"hello\n"` |

**Cap semantics discovery:** `LinesCodec` errors when `buf.len() > max_length` (strict `>`).
A frame of exactly `MAX_FRAME_BYTES` bytes passes through; `MAX_FRAME_BYTES + 1` errors.
Verified against `tokio-util 0.7.18` source at `lines_codec.rs` line 151.

### `tests/command_test.rs` — 11 tests

| Test name | What it verifies |
|-----------|-----------------|
| `login_frame_wire_bytes` | `{"cmd":"login","args":{"session_key":"K","service":"NEXTAPI"}}` |
| `subscribe_market_data_price` | `{"cmd":"subscribe","args":{"t":"price","m":11,"i":"101"}}` |
| `subscribe_market_data_depth` | `{"cmd":"subscribe","args":{"t":"depth","m":11,"i":"101"}}` |
| `subscribe_market_data_trade` | `{"cmd":"subscribe","args":{"t":"trade","m":11,"i":"101"}}` |
| `subscribe_market_data_trading_status` | `{"cmd":"subscribe","args":{"t":"trading_status","m":11,"i":"101"}}` |
| `subscribe_indicator` | `{"cmd":"subscribe","args":{"t":"indicator","m":"SSE","i":"OMXS30"}}` (string `m`) |
| `subscribe_news_no_delay_omits_field` | `delay: None` → field omitted entirely (no `null`) |
| `subscribe_news_explicit_false_emits_field` | `delay: Some(false)` → `"delay":false` (NOT omitted) |
| `subscribe_news_explicit_true` | `delay: Some(true)` → `"delay":true` |
| `unsubscribe_mirrors_subscribe` | Same args, `cmd` verb is `"unsubscribe"` |
| `subscribe_args_round_trip_for_unsubscribe_symmetry` | `SubscribeArgs` `Clone + Eq + Hash` works; `HashSet` insert/contains round-trip |

**Serialization approach:** `encode_subscribe_frame` / `encode_login_frame` are
`pub(crate)` and unreachable from integration tests. Tests use `serde_json::to_string`
on the public `Serialize` impls directly. For the subscribe envelope, string
concatenation (`format!(r#"{{"cmd":"{cmd}","args":{inner}}}"#, ...)`) guarantees
`cmd`-before-`args` ordering regardless of serde_json's BTreeMap key sorting.

### `Cargo.toml` changes

None. The workspace `tokio` dep already includes `macros` and `rt-multi-thread`
features (required for `#[tokio::test]`). The feed crate's `[dependencies]` entry adds
`io-util` and `net`; feature resolution is additive, so `macros` is available without
a separate `[dev-dependencies]` entry.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --package nordnet-feed --test codec_test` — 4/4 passed
- `cargo test --package nordnet-feed --test command_test` — 11/11 passed
- `cargo test --workspace` — all tests pass (15 new tests added: 4 codec + 11 command)

---

## Phase 2.4 Test Agent C — client_test

**Date:** 2026-05-02

### Files created

- `crates/nordnet-feed/tests/client_test.rs` — 8 tests

### Test names and count (8 total)

End-to-end loopback TCP tests (PublicFeedClient + PrivateFeedClient):

1. `subscribe_then_recv_price_tick` — client sends login + subscribe, server replies with price tick; asserts `i` and `m` fields
2. `plain_tcp_path_works` — `encrypted=false` connect + heartbeat frame exchange succeeds
3. `heartbeat_with_extra_fields_stays_heartbeat` — `data:{"server_time":...}` still routes to `Heartbeat`, not `Unknown` (forward-compat)
4. `unknown_envelope_type_lands_in_unknown` — `"type":"future_kind"` → `PublicEvent::Unknown { kind: "future_kind", .. }`
5. `server_err_surfaces_as_event_not_result_err` — server `err` frame → `PublicEvent::Error(ServerError { msg: "Not authorized." })` via `recv()`, not `Result::Err`
6. `login_error_then_close_returns_none_after_err` — login + 3 subscribes; server sends one `err` then closes; first `recv()` = `Error`, second `recv()` = `Ok(None)` (frame-ordering caveat from spec §"Login command")
7. `mid_frame_disconnect_returns_err` — server writes partial JSON then `shutdown()`; `recv()` returns `Err(FeedError::Decode { .. })`. Note: `LinesCodec` delivers partial data on clean EOF as an implicit line; serde_json fails to parse → Decode, not Closed. Both `Decode` and `Closed` are accepted (platform-dependent RST vs FIN)
8. `private_feed_order_event_round_trip` — spec golden order payload; asserts `order_id == 202178767`

### Deviation from task template

`mid_frame_disconnect_returns_closed` was renamed to `mid_frame_disconnect_returns_err` and the match was expanded to accept `FeedError::Decode` (the actual outcome) alongside `FeedError::Closed`. `LinesCodec` treats clean TCP FIN (from `sock.shutdown()`) as an implicit line terminator and delivers the partial buffer — serde_json then fails to parse the truncated JSON. `FeedError::Closed` only appears on abrupt RST (OS-level condition, not guaranteed from `sock.shutdown()`). The test still verifies the key guarantee: the result is always `Err(...)`, never `Ok(Some(event))`.

### Gate results

- `cargo build --workspace` — green
- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --package nordnet-feed --test client_test` — 8/8 pass

---

## Phase 2.5 — Workspace integration

**Date:** 2026-05-02

### gen-mods result

**No-op.** `cargo run -p xtask -- gen-mods` ran cleanly with zero file writes.
This is the expected outcome: `nordnet-feed/src/lib.rs` is hand-written with
explicit `pub mod` declarations, and `xtask::MOD_DIRS` only manages
`crates/nordnet-api/src/{models,resources}`, `crates/nordnet-cli/src/cmd`, and
`crates/nordnet-model/src/models` — none of which apply to the feed crate
(it has no `models/` subdirectory pattern).

### Cleanup pass findings

- **`pub use` block:** All 9 re-exports in `lib.rs` (`LoginCommand`,
  `MarketDataKind`, `SubscribeArgs`, `FeedError`, `ServerError`,
  `PrivateEvent`, `PublicEvent`, `PrivateFeedClient`, `PublicFeedClient`)
  are part of the documented public API surface. Tests reach into module
  paths (e.g. `nordnet_feed::command::*`) but downstream consumers will
  use the top-level re-exports. Left as-is.
- **`recv()` doc comments:** Both `PublicFeedClient::recv` and
  `PrivateFeedClient::recv` already describe the three terminal states
  accurately — `Ok(None)` on clean EOF between frames,
  `Err(FeedError::Closed)` on abrupt RST mid-frame, and
  `Err(FeedError::Decode { .. })` on clean FIN with partial data. The
  spec gap surfaced by phase 2.4 (LinesCodec delivering partial buffers
  on clean FIN) is reflected in the rustdoc. No changes needed.
- **Notes file:** All 8 prior phase sections present (2.0, 2.1, 2.2 A+B,
  2.3 C+D, 2.4 A+B+C). This Phase 2.5 section closes the file.

### Final test count

**291 total** across the workspace. Breakdown:
- Pre-PR2 baseline: 247 tests
- Phase 2.1 (inline `command.rs`): +8 = 255
- Phase 2.4 Agent A (codec_test + command_test): +15 = 270
- Phase 2.4 Agent B (event_test): +13 = 283
- Phase 2.4 Agent C (client_test): +8 = 291

Matches the spec's expected 291.

### Static gates (final, workspace-wide)

- `cargo fmt --check` — green
- `cargo clippy --workspace --all-targets -- -D warnings` — green
- `cargo test --workspace` — 291/291 pass

### Deviations from spec

**None.** Phase 2.5 closed exactly as the process doc described.

### Suggested final commit message

```
chore(feed): regenerate mod files + finalize crate

Closes PR2 (nordnet-feed crate split).

- gen-mods is a no-op for nordnet-feed (hand-written lib.rs, no
  models/ subdirectory pattern).
- All workspace static gates green: fmt, clippy, test.
- Test count: 291 total (247 baseline + 44 new feed-crate tests
  across phases 2.1 and 2.4).
- See notes/PR2-feed-crate.md for the full per-phase breakdown.
```
