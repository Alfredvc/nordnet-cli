# Nordnet Feed Crate + Model Extraction — Design

**Date:** 2026-05-02
**Status:** Revised after two plan-reviewer + two protocol-verifier passes; pending user approval
**Scope:** Add streaming feed support; restructure workspace into a three-crate split (model / api / feed).

---

## Goal

Add support for the Nordnet External API v2 streaming feeds (public market data + private order/trade events) without bloating `nordnet-api` and without forcing REST-only consumers to compile a TLS socket stack.

## Motivation

The Nordnet API exposes two transports:

1. **REST** (already implemented in `nordnet-api`)
2. **Streaming Feeds** — TCP/TLS socket, newline-delimited JSON. Two feeds:
   - **Public Feed:** market data (price, depth, trade, trading status, indicator, news). Subscription-based.
   - **Private Feed:** account events (orders, fills). Auto-pushed after login.

Both transports authenticate against the same `session_key` (obtained from REST `/login/verify`), and both consume the connection-info `Feed { hostname, port, encrypted }` returned in that same login response. They are siblings, not stacked layers.

## Architectural decision: three-crate split

Established Rust precedent for vendor SDKs that span multiple transports:

- **twilight** (Discord): `twilight-model` / `twilight-http` / `twilight-gateway`
- **aws-sdk-rust**: `aws-types` / `aws-smithy-*` / per-service crates
- **tonic**: codegen split for the same reason

Apply the same pattern here:

```
crates/
├── nordnet-model    pure types + pure-compute crypto. No I/O.
├── nordnet-api      REST client. Depends on nordnet-model.
├── nordnet-feed     streaming client. Depends on nordnet-model.
└── nordnet-cli      binary. Depends on nordnet-api + nordnet-feed.
```

### Why not feature-flag inside `nordnet-api`?

Feed has its own substantial surface: codec, async lifecycle, ~10 event payload structs, command types. `cfg(feature = "feed")` gates spread across hundreds of lines + a doubled test matrix gets uglier than a sibling crate. Feature flags suit small additions; a parallel transport is not small.

### Why not depend `nordnet-feed` on `nordnet-api`?

That couples the streaming crate to reqwest and the entire HTTP stack. The dependency that actually exists is on **types** (`Session`, `Feed`), not on **HTTP code**. Hoisting those types into `nordnet-model` lets each transport crate stand alone while remaining wire-compatible.

### Why not three crates is overkill?

The workspace already has three members (api / cli / xtask). One more is no organizational burden. Compile-time isolation (rustc compiles per crate) is a real benefit even for in-repo consumers. The migration is mechanical — moving files, updating imports — not redesign.

---

## Crate contents

### `nordnet-model`

Pure data + pure compute. Zero I/O dependencies.

**Modules (moved from `nordnet-api`):**

- `auth` — `Session`, `sign_challenge`, `parse_private_key_openssh`, `ApiKeyStartLoginRequest`, `ApiKeyVerifyLoginRequest`, `ChallengeResponse`. Pure crypto + login request/challenge types. The `Session::basic_auth_header` helper stays here (pure string formatting).
- `models/` — every existing per-resource module (accounts, instruments, orders, tradables, login, shared, etc.). These are serde definitions, not transport.
- `models/login` — owns the **single canonical** `ApiKeyLoginResponse` and `Feed` types. Today the codebase has two `ApiKeyLoginResponse` definitions: the loose one in `auth.rs` (with `Option<serde_json::Value>` feeds) and the typed one in `models/login.rs`. **The migration collapses these to one** — the typed version wins; the loose one is deleted; `to_session()` moves with it. The `auth` module does NOT re-export it.
- `ids` — newtype wrappers used across resources.
- `error::AuthError` — new error type covering only what `auth` can fail at: `InvalidKey(String)`, `EncryptedKey`, `WrongAlgorithm { got, expected }`, `KeyDataMismatch`. No HTTP variants.

**Workspace deps:** `serde`, `serde_json`, `rust_decimal`, `time`, `ed25519-dalek`, `ssh-key`, `base64`, `thiserror`.

**Does NOT depend on:** reqwest, tokio, tokio-rustls.

### `nordnet-api`

Keeps everything HTTP-shaped:

- `client::Client` — reqwest-backed HTTP client.
- `resources/` — accounts, instruments, orders, instrument_search, main_search, markets, countries, news, root, tick_sizes, tradables, login.
- `pagination` — query-string pagination helpers.
- `error::Error` — HTTP-mapped variants (`BadRequest`, `Unauthorized`, `Forbidden`, `TooManyRequests`, `ServiceUnavailable`, `UnexpectedStatus`, `Transport(reqwest::Error)`, `Decode`, `InvalidHeader`, `EncodeForm`). The `Auth` variant becomes `Auth(#[from] nordnet_model::AuthError)`.

The login resource (`resources/login.rs`) calls reqwest with bodies/responses **typed by `nordnet-model`** — no model definitions live here.

**Workspace dep added:** `nordnet-model = { path = "../nordnet-model" }`.

### `nordnet-feed`

New crate. Layout:

```
crates/nordnet-feed/
├── Cargo.toml
└── src/
    ├── lib.rs        re-exports
    ├── public_client.rs   PublicFeedClient
    ├── private_client.rs  PrivateFeedClient
    ├── codec.rs      newline-JSON framing (shared)
    ├── command.rs    outbound: Login, Subscribe, Unsubscribe (shared)
    ├── event.rs      PublicEvent + PrivateEvent enums + envelope decode
    ├── public.rs     public payload structs (Price, Depth, Trade, …)
    ├── private.rs    private payload structs (OrderEvent)
    └── error.rs      FeedError + ServerError
```

**Workspace deps added:** `tokio-rustls = "0.26"` (current line for tokio-rustls), `rustls = "0.23"` (current line for rustls), `webpki-roots = "1"` (note: the `0.26` line of `webpki-roots` is the deprecated semver-trick shim — pin the `1.x` line directly), `tokio-util = { version = "0.7", features = ["codec"] }` (for `LinesCodec`). Reuses existing `tokio`, `serde`, `serde_json`, `rust_decimal`, `time`, `thiserror`. The implementer MUST verify these are still current at PR time via `cargo search` — pinning the current line at write time, not at the time the spec was written.

**Crate deps:** `nordnet-model = { path = "../nordnet-model" }`.

---

## Wire protocol (canonical reference)

Source: <https://www.nordnet.se/externalapi/docs/feeds> + <https://github.com/nordnet/next-api-v2-examples>.

### Connection

- TCP, host/port from `nordnet_model::models::login::Feed { hostname, port, encrypted }`.
- TLS handshake **iff `encrypted == true`**. The official Python `test_program.py` uses a `port == 443` heuristic instead — that's a different rule that happens to coincide today (every encrypted feed reports port 443). This crate deliberately diverges by honoring the structured field rather than the port number, on the principle that respecting a typed wire field is more robust than re-deriving the same bit from the port. If `encrypted == false`, the socket is plain TCP.
- All frames are JSON objects, **terminated by a single LF (`\n`, ASCII 10)**. Frames are bounded to **1 MiB** by the codec; longer frames return `FeedError::FrameTooLarge`. The cap is a designer choice (Nordnet docs do not specify a max frame size) intended as a memory-DoS guard against malformed input — large enough that any plausible Nordnet event fits, small enough that an unbounded read is impossible. Configurable in a future revision if real payloads approach the cap.
- One public connection + one private connection per session, max. The "public" cap is documented for the private feed only in the source we have; the public-feed equivalent is asserted by the design as a safe over-restriction.

### Login command (per-feed)

After socket connect, before any other traffic:

```json
{"cmd":"login","args":{"session_key":"...","service":"NEXTAPI"}}\n
```

The `service` field is required per the official Python examples (the public docs page omits it but the reference impl always sends it). The constant value is `"NEXTAPI"`.

**Login is fire-and-forget.** `login()` writes the frame and returns immediately, matching the official Python example which sends login + subscribe back-to-back without waiting for a reply. The protocol does not define a "login OK" response; the docs only state "If the login is correct the feed will start sending as soon as there is data to send." On failure, the server sends an `err` frame and/or closes the connection — both paths surface through the normal `recv()` loop:
- `err` → `Event::Error(ServerError)` (caller handles)
- close → next `recv()` returns `Ok(None)` (clean EOF between frames) or, if the close happened mid-frame, `Err(FeedError::Decode { .. })` on a clean FIN with partial data (the OS surfaces the half-frame as a line; serde_json then errors), or `Err(FeedError::Closed)` on an abrupt RST. The implementation cannot distinguish "server intended to send more" from "server crashed" — both produce errors with the partial line attached.

This removes the timing-dependent "wait up to 5s for the first heartbeat" heuristic entirely. There is no `FeedError::LoginRejected` variant.

**Frame-ordering caveat.** Because both `login()` and `subscribe()` are fire-and-forget and the server has no `request_id` correlation in its `err` echo, a consumer that fires `login()` then a burst of `subscribe()`s and then starts `recv()`-ing receives a stream of `Event::Error` frames with no deterministic mapping back to the call that caused each one (the `cmd` echo is the original frame, not a id). For consumers that need to detect login failure deterministically before subscribing, the recommended pattern is:

```rust
client.login(key).await?;
// Drain any immediate error or wait for first non-err frame:
match client.recv().await? {
    Some(PublicEvent::Error(e)) => return Err(e.into()),  // login failed
    Some(other) => { /* feed is up; process and continue */ }
    None => return Err("connection closed during login"),
}
// Now safe to subscribe.
```

This is documented on `login()`'s rustdoc but not enforced — fire-and-burst is still allowed for consumers who can correlate errors via the echoed `cmd` field themselves.

### Subscribe / unsubscribe (public feed only)

`t` ∈ `price | depth | trade | trading_status | indicator | news`. Per Nordnet's docs, `delay` is **only** accepted on `news` subscriptions and is marked deprecated even there. The `MarketData` and `Indicator` variants therefore do **not** carry a `delay` field — the original spec was wrong to put it on all three.

Worked wire-bytes per variant:

```json
// MarketData (kind=Price, market=11, identifier="101")
{"cmd":"subscribe","args":{"t":"price","m":11,"i":"101"}}\n

// MarketData (kind=Depth, market=11, identifier="101")
{"cmd":"subscribe","args":{"t":"depth","m":11,"i":"101"}}\n

// Indicator (market="SSE", identifier="OMXS30")
{"cmd":"subscribe","args":{"t":"indicator","m":"SSE","i":"OMXS30"}}\n

// News (source_id=2)  — Rust field `source_id`, wire field `s`
{"cmd":"subscribe","args":{"t":"news","s":2}}\n

// News with delay
{"cmd":"subscribe","args":{"t":"news","s":2,"delay":true}}\n
```

`unsubscribe` mirrors `subscribe` — same args, `cmd` value `"unsubscribe"`. To make round-trip symmetry trivial for consumers, `SubscribeArgs` derives `Clone + PartialEq + Eq + Hash`; consumers can stash the same value and hand it back to `unsubscribe()` later. Serialization must be deterministic across both calls — `delay: None` omits the field entirely (no `null`), `delay: Some(false)` emits `"delay":false`.

Successful return from `subscribe()` means the frame was *written*, not that the server accepted it. Server-side rejections (rate-limit, unknown instrument, unauthorized) arrive asynchronously as `Event::Error` frames via `recv()`. Doc this on the method.

### Heartbeat

Server-to-client only, every 5s when idle:

```json
{"type":"heartbeat","data":{}}
```

No client-side heartbeat is required by the protocol. The `Heartbeat` variant carries no payload; the deserializer accepts an empty `data` object via `#[serde(default)]` and **ignores any extra fields** (forward-compat). A future server adding `{"type":"heartbeat","data":{"server_time":123}}` continues to deserialize as `Heartbeat`, not `Unknown`.

### Error frames

```json
{"type":"err","data":{"msg":"Not authorized.","cmd":{...original...}}}
```

### Rate limits

Two server-side thresholds: soft (drops + `err` frames) and hard (disconnect). Documented guidance: stay below "a few hundred commands per second."

### Tick framing: full vs delta

- **First message** for a (market, instrument) pair after subscription contains every field (full tick).
- **Subsequent messages** include only `m`, `i`, plus changed fields. A field that has been *removed* (was set, now absent) arrives as explicit `null`.
- Out of scope for v1: this crate **does not** merge deltas. Events are surfaced as the server sent them; consumers maintain their own state if desired. Field types are `Option<T>` to admit absent fields, with `#[serde(default)]` for missing fields and explicit `null` collapsing to `None`.

### Forward compatibility

The Nordnet docs explicitly state: "Nordnet can at any time add fields to the feed messages, fields not documented should be considered as features and can be removed at any time. The client must be able to handle fields not covered in the documentation." → Payload structs use `#[serde(default)]` for all optional fields and **do not** use `deny_unknown_fields`.

---

## Public feed event types

Envelope:
```json
{"type":"<kind>","data":{...}}
```

Common identifier fields on all market data: `i: String` (instrument id), `m: i64` (market id). For `indicator`, `m: String`.

### `price` (`PublicEvent::Price`)
| Field | Type | Optional |
|-------|------|----------|
| `i` | `String` | required |
| `m` | `i64` | required |
| `delayed` | `i64` | optional |
| `trade_timestamp` | `i64` (ms) | optional |
| `tick_timestamp` | `i64` (ms) | optional |
| `bid` / `ask` / `last` / `open` / `high` / `low` / `close` / `vwap` / `ep` / `extended_last` | `Decimal` | optional |
| `bid_volume` / `ask_volume` / `last_volume` / `turnover_volume` / `paired` / `imbalance` | `Decimal` | optional |
| `turnover` | `Decimal` | optional |

**Volume on the wire:** all volume fields are `Decimal`, not `i64`. The official `order` payload shows `"volume": 111.0` — server is willing to send fractional/integer-as-float numbers. `Decimal` deserializes both `111` and `111.0` cleanly via `serde-arbitrary-precision`, while `i64` chokes on the latter. Counts that are clearly not volumes (order counts in depth, instrument ids, market ids, source ids, news ids, timestamps) stay `i64`.

### `depth` (`PublicEvent::Depth`)
- `i: String`, `m: i64`, `tick_timestamp: i64` (required).
- Levels 1–5: `bid{n}` / `ask{n}` (`Decimal`), `bid_volume{n}` / `ask_volume{n}` (`Decimal`), `bid_orders{n}` / `ask_orders{n}` (`i64` — counts of orders, not volumes). All optional.

### `trade` (`PublicEvent::Trade`)
| Field | Type |
|-------|------|
| `i` | `String` (req) |
| `m` | `i64` (req) |
| `trade_timestamp` | `i64` ms (req) |
| `price` | `Decimal` (req) |
| `volume` | `Decimal` (req) |
| `broker_buying` / `broker_selling` / `trade_id` / `trade_type` | `Option<String>` |

### `trading_status` (`PublicEvent::TradingStatus`)
- `i: String`, `m: i64`, `tick_timestamp: i64`, `status: String` (single char: C/R/D/X/U).
- Optional: `source_status`, `halted`, `orderbook_status`.

### `indicator` (`PublicEvent::Indicator`)
- `i: String`, `m: String` (note: **`m` is `String` here**, not `i64` like other event types — official-docs distinction; do **not** reuse the `MarketId` newtype here).
- `tick_timestamp: i64` (req).
- Optional: `last`, `high`, `low`, `close` (`Decimal`), `delayed` (`i64`).

### `news` (`PublicEvent::News`)
- `news_id: i64`, `lang: String`, `timestamp: i64`, `source_id: i64`, `headline: String`, `type: String`, `instruments: Option<Vec<i64>>`.

---

## Private feed event types

Auto-pushed after private-feed login. No subscription. Envelope identical (`{"type":..., "data":...}`).

### `order` (`PrivateEvent::Order`)

Verified against the official example payload:

```json
{"type":"order","data":{"volume":111.0,"price":{"value":132.55,"currency":"SEK"},"volume_condition":"NORMAL","order_id":202178767,"reference":"ABC132","tradable":{"market_id":11,"identifier":"101"},"validity":{"type":"DAY","valid_until":1613061300000},"accno":123123,"accid":1,"side":"BUY","modified":1612955053717,"activation_condition":{"type":"NONE"},"order_state":"LOCAL","action_state":"INS_PEND","order_type":"LIMIT"}}
```

| Field | Type | Required |
|-------|------|----------|
| `order_id` | `i64` | yes |
| `accno` / `accid` | `i64` | yes |
| `tradable` | `{ market_id: i64, identifier: String }` | yes |
| `side` | `Side` (BUY / SELL) | yes |
| `volume` | `Decimal` | yes (wire shows `111.0`) |
| `price` | `{ value: Decimal, currency: String }` | yes |
| `volume_condition` | `VolumeCondition` (Normal / FillOrKill / ImmediateOrCancel / AllOrNone / …) | yes |
| `validity` | `Validity { kind: ValidityKind (Day / GTC / GTD / IOC / …), valid_until: i64 }` (rust field `kind`, wire `type`) | yes |
| `activation_condition` | `ActivationCondition { kind: ActivationConditionKind (None / …) }` (rust `kind`, wire `type`) | yes |
| `order_state` | `OrderState` (Local / Active / Filled / Cancelled / …) | yes |
| `action_state` | `ActionState` (InsPend / ModPend / DelPend / Acked / …) | yes |
| `order_type` | `OrderType` (Limit / Market / StopLimit / StopLoss / …) | yes |
| `modified` | `i64` (ms) | yes |
| `reference` | `Option<String>` | optional (present in example, not explicitly marked optional in docs — modelled as Option to be safe) |

**Typed-enum convention.** Every wire-string field above (`Side`, `VolumeCondition`, `ValidityKind`, `ActivationConditionKind`, `OrderState`, `ActionState`, `OrderType`) is a Rust enum modeled as:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OrderState {
    Known(KnownOrderState),
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownOrderState {
    #[serde(rename = "LOCAL")] Local,
    #[serde(rename = "ACTIVE")] Active,
    // …
}
```

The `Known + Unknown(String)` split honors Nordnet's forward-compat rule (new wire values must round-trip without errors) while giving consumers exhaustive matching on the documented set. Mirrors the existing `Side` typing precedent in the REST crate. Initial `Known` variants come from the documented values for v1; new variants are added as Nordnet publishes them. **Concrete variant lists for each enum are pulled from `docs-source/nordnet-api-v2.html` during implementation, not enumerated in this spec — the implementer treats those as the source of truth and adds an `Unknown` arm to absorb anything missed.**

### `trade` on the private feed (`PrivateEvent::TradeRaw`)
Per docs: "mirrors the HTTP GET trades request structure." Concrete schema not enumerated in the public Nordnet docs (the third-party Go model gives a candidate shape but is not Nordnet-published — risky to type against). **For v1, ship as `serde_json::Value`** behind a deliberately-named variant `PrivateEvent::TradeRaw(serde_json::Value)`. The `Raw` suffix is the in-API signal that this is the only payload without a typed struct. Follow-up: capture a live sample, define the typed struct, and add `PrivateEvent::Trade(private::Trade)` alongside (don't replace `TradeRaw` — keep it as an escape hatch).

### `heartbeat` (`{Public,Private}Event::Heartbeat`)
Empty data. Decoded into a unit variant on both feeds.

### `err` (`{Public,Private}Event::Error`)
`ServerError { msg: String, cmd: serde_json::Value }`. Always surfaced as an event, not as a `Result::Err` from `recv`, because errors are per-command and the connection is still alive. Includes login-time errors (since `login()` is fire-and-forget; see Login section).

---

## Public API surface (`nordnet-feed`)

### Public vs private split: two distinct client types

The Nordnet protocol uses `"type":"trade"` on **both** feeds with different payload shapes (public = market trade, private = own-account fill). A single `Event` enum can't unambiguously deserialize a `trade` frame without knowing the connection's role.

Resolution: ship two separate client structs that share an internal codec but expose feed-specific event enums.

```rust
/// Both client types take `&mut self` on every method. To run send and
/// receive concurrently, split externally with `tokio::io::split` plus
/// `Arc<Mutex<...>>` — not provided by the crate.
pub struct PublicFeedClient { /* TCP + optional TLS stream + codec */ }
pub struct PrivateFeedClient { /* TCP + optional TLS stream + codec */ }

impl PublicFeedClient {
    /// Connect to `feed.hostname:feed.port`. Performs a TLS handshake
    /// iff `feed.encrypted == true`.
    pub async fn connect(feed: &nordnet_model::models::login::Feed) -> Result<Self, FeedError>;

    /// Fire-and-forget login. Writes the login frame and returns. Server
    /// errors arrive via `recv()` as `PublicEvent::Error`.
    pub async fn login(&mut self, session_key: &str) -> Result<(), FeedError>;

    /// Writes the subscribe frame. Successful return means the frame was
    /// written, NOT that the server accepted the subscription. Server
    /// rejections (rate-limit, bad instrument, etc.) arrive via `recv()`.
    pub async fn subscribe(&mut self, args: SubscribeArgs) -> Result<(), FeedError>;
    pub async fn unsubscribe(&mut self, args: SubscribeArgs) -> Result<(), FeedError>;

    /// Receive the next event. Returns `Ok(None)` on clean EOF.
    pub async fn recv(&mut self) -> Result<Option<PublicEvent>, FeedError>;
}

impl PrivateFeedClient {
    pub async fn connect(feed: &nordnet_model::models::login::Feed) -> Result<Self, FeedError>;
    pub async fn login(&mut self, session_key: &str) -> Result<(), FeedError>;
    // No subscribe — private feed is auto-push after login.
    pub async fn recv(&mut self) -> Result<Option<PrivateEvent>, FeedError>;
}

pub enum PublicEvent {
    Heartbeat,
    Error(ServerError),
    Price(public::Price),
    Depth(public::Depth),
    Trade(public::Trade),
    TradingStatus(public::TradingStatus),
    Indicator(public::Indicator),
    News(public::News),
    Unknown { kind: String, data: serde_json::Value },
}

pub enum PrivateEvent {
    Heartbeat,
    Error(ServerError),
    Order(private::OrderEvent),
    /// Untyped — schema not in public Nordnet docs. See spec "Out of scope".
    TradeRaw(serde_json::Value),
    Unknown { kind: String, data: serde_json::Value },
}

pub struct ServerError {
    pub msg: String,
    pub cmd: serde_json::Value,
}

/// Compile-time-distinct subscribe variants — prevents constructing
/// `Indicator` with an integer market or `News` with `m`/`i` fields.
/// Derives `Clone + Eq + Hash` so callers can stash a value and hand it
/// back to `unsubscribe()` later (round-trip symmetry).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscribeArgs {
    /// Standard market data: price, depth, trade, trading_status.
    MarketData {
        kind: MarketDataKind,        // Price | Depth | Trade | TradingStatus
        market: i64,
        identifier: String,
    },
    /// Indicator subscriptions use a string `m` per Nordnet's docs.
    Indicator {
        market: String,
        identifier: String,
    },
    /// News uses `s` (source id) instead of `m`/`i`. `delay` is news-only
    /// per Nordnet (deprecated even there — kept for completeness).
    News {
        source_id: i64,
        delay: Option<bool>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketDataKind { Price, Depth, Trade, TradingStatus }
```

The `MarketDataKind` enum exists to keep the wire-`t` value compile-time-correct without exposing `Indicator`/`News` as choices in the wrong place. Serialization is implemented manually (custom `Serialize` impl) — each variant maps to its own wire shape; `serde(tag)` doesn't fit because the payload differs structurally per variant. The wire-bytes section above shows the exact output for each variant.

`delay` deliberately appears only on `News` per Nordnet's docs (the official wording: "Used only in `\"subscribe\"` commands for `\"news\"`"). Putting it on `MarketData`/`Indicator` was an early-spec error.

Internal `codec` and outbound `command` modules are shared between both client types. Public-facing event enums diverge so the type system enforces "you can only call `subscribe` on the feed that supports it" and "a `Trade` you receive is unambiguously the kind your client knows about."

**No shared trait between `PublicFeedClient` and `PrivateFeedClient` for v1.** They duplicate `connect`/`login`/`recv` signatures intentionally — the duplication is small and the future state-machine work for reconnect (out of scope for v1) is exactly when a shared trait would earn its keep. Adding it now is speculative; revisit when reconnect lands.

Wire-side field naming: every payload struct that has a `type` field on the wire (e.g. news has `"type":"..."`, validity has `"type":"DAY"`) maps to a Rust field named `kind` via `#[serde(rename = "type")]` to avoid the keyword collision and to disambiguate from the envelope `type`.

The single-connection API per client is intentional. Splitting into `Sender` / `Receiver` halves is rejected for v1: most consumers will run a single task that interleaves `subscribe` and `recv`, and `tokio::io::split` plus `Arc<Mutex<>>` is straightforward to layer on later if needed.

### `FeedError`

```rust
pub enum FeedError {
    Tls(rustls::Error),
    Io(std::io::Error),
    Decode { source: serde_json::Error, line: String },
    Encode(serde_json::Error),
    FrameTooLarge { bytes: usize },   // codec hit max line length (1 MiB)
    Closed,                            // peer hung up via abrupt RST mid-frame
}
```

No `LoginRejected` (login is fire-and-forget; rejections arrive as `Event::Error`). No `UnexpectedFrame` (forward-compat policy puts unknown frames in `Event::Unknown`).

**Mid-frame disconnect taxonomy.** `Closed` only fires on an abrupt RST that surfaces as `io::ErrorKind::UnexpectedEof`. A clean TCP FIN with buffered partial data delivers the half-frame as a line, which then fails JSON parsing and surfaces as `Decode { source, line }`. Both states are unrecoverable; both leave the partial line available for diagnostics. Callers that only care about "stream is over" should treat `Closed`, `Decode`, and `Ok(None)` uniformly as terminal.

No retry, no reconnect, no backoff — those are caller concerns. The crate surfaces failures verbatim.

---

## Out of scope for v1

Explicitly **not** included; deferred to follow-ups:

- **Delta merging.** Caller maintains state.
- **Reconnect / backoff.** Caller's responsibility per Nordnet's own guidance.
- **CLI integration.** No `nordnet-cli feed` subcommand. Feed crate is library-only.
- **Backpressure / drop policy.** No internal buffering; `recv` drives reads directly.
- **Concurrent senders.** Single-task API. No `Arc<Mutex>` wrappers shipped.
- **Typed `PrivateEvent::TradeRaw` payload.** Raw `serde_json::Value` with follow-up to type from a live sample.
- **Feed metrics / observability hooks.** No tracing spans, no callbacks.

---

## Testing strategy

**`nordnet-model`:**
- Existing `auth` and `models/` tests move with their modules. No behavioral change expected.

**`nordnet-api`:**
- Existing tests stay; imports update. `wiremock`-based resource tests untouched.

**`nordnet-feed`:**
- **Codec round-trip:** `tokio::io::duplex` exercises newline framing + max-length cutoff without TLS.
- **Event deserialization:** golden JSON samples per event variant (one full + one delta where applicable). Use `pretty_assertions` (already a workspace dev-dep). The official `order` example payload (quoted in spec) is a required golden.
- **Command serialization:** verify exact wire bytes for login + each `SubscribeArgs` variant (MarketData, Indicator, News with and without `delay`). Cover the `delay: None` → field-omitted vs `delay: Some(false)` → `"delay":false` distinction.
- **Subscribe→receive integration:** mocked server (over `tokio::io::duplex`) accepts a `subscribe` frame and replies with a `price` tick; client parses correctly. Catches bugs that pure codec + pure deserialize tests miss.
- **Plain-TCP path:** `connect` against a non-TLS endpoint when `feed.encrypted == false` succeeds and reads/writes work. Use a `tokio::net::TcpListener` bound to `127.0.0.1`.
- **Validity rename round-trip:** deserialize `{"type":"DAY"}` into `validity.kind = ValidityKind::Known(Day)`, reserialize back to wire form, verify exact bytes match.
- **Typed-enum unknown variants:** wire `{"order_state":"FUTURE_VALUE_NOT_YET_DEFINED"}` deserializes as `OrderState::Unknown("FUTURE_VALUE_NOT_YET_DEFINED")`, does not error.
- **Forward compat (envelope):** an unknown `type` value deserializes into `{Public,Private}Event::Unknown`; an unknown field in a known type does not error; `Heartbeat` accepts `data: { extra_field: 1 }` without falling through to `Unknown`.
- **Server error surfacing:** simulated server `err` frame → `Event::Error(ServerError)` via `recv()`.
- **Login error mid-subscribe:** mock sequence: client sends login + 3 subscribes, server replies with one `err` (login rejection) and closes; verify the `err` arrives as `Event::Error` and the next `recv()` returns `Ok(None)`. Documents the frame-ordering caveat.
- **Frame too large:** simulated server sends > 1 MiB without `\n` → `FeedError::FrameTooLarge`.
- A live test environment exists (`api.test.nordnet.se`) but requires Nordnet support to provision credentials. v1 ships without live network tests; live integration is a follow-up once test creds are obtained.

---

## Migration plan (informational — full plan comes from writing-plans)

Two PRs. PR1 is **one atomic commit** — the substeps below are facets of that single commit, not separate landings, because the workspace will not compile between them.

**PR 1: Extract `nordnet-model`.** Within one commit, in this order to keep the surgery tractable:

1. **Create scaffold:** `crates/nordnet-model/Cargo.toml` + `src/lib.rs` + empty module skeleton (`auth.rs`, `models/mod.rs`, `ids.rs`, `error.rs`). Add `nordnet-model` to workspace members.
2. **Add the dep in `nordnet-api`:** `nordnet-model = { path = "../nordnet-model" }`. Workspace still compiles (model crate is empty + new dep is unused).
3. **Copy files into `nordnet-model`** (don't delete from `nordnet-api` yet): `auth.rs` (sans the loose `ApiKeyLoginResponse` — collapse into `models/login.rs`'s typed version), `models/*.rs`, `ids.rs`. Define `nordnet_model::error::AuthError`. Adjust `auth::parse_private_key_openssh` to return `AuthError`. Workspace still compiles (originals still in place).
4. **Switch imports** in `nordnet-api` and `nordnet-cli` from `nordnet_api::{auth,models,ids}` to `nordnet_model::{auth,models,ids}`. Replace `Error::Auth(String)` with `Error::Auth(#[from] nordnet_model::AuthError)`. Workspace compiles against the new locations; originals are now unused.
5. **Delete originals** from `nordnet-api`: `auth.rs`, `models/` directory, `ids.rs`. Workspace compiles cleanly with no shims.
6. **Verify:** `cargo build`, `cargo test`, `cargo clippy` workspace-wide all green.

The "copy then switch then delete" ordering keeps every intermediate state buildable. Doing "delete first, then update imports" leaves the workspace broken between substeps even though the final state is identical.

**PR 2: Add `nordnet-feed`.** New crate per the layout above. Tests with mocked I/O. No CLI integration.

---

## Open questions resolved

- **Naming:** `nordnet-model` (matches twilight-model precedent). Not `nordnet-core` (overloaded), not `nordnet-types` (less common in mature SDKs).
- **Form-encoding helper location:** stays in `nordnet-api` (used only by HTTP). `serde_urlencoded` does not need to be a model-crate dep.
- **`Decimal` vs `f64` vs `i64`:** `Decimal` for **all** prices and **all** volumes (public + private). Server is willing to send fractional-as-float numbers (`"volume": 111.0`); `i64` chokes on those, `Decimal` round-trips both `111` and `111.0`. `i64` for counts that are clearly not volumes (order counts in depth) and for raw ids/timestamps.
- **TLS handshake:** honor `Feed.encrypted`. The Python reference impl uses a `port == 443` heuristic instead — different rule, same result today, but we trust the structured field.
- **Login:** fire-and-forget. No blocking ack, no `LoginRejected` error variant. Server errors arrive via `recv()` like any other. Recommended deterministic-detection pattern documented on `login()`.
- **`SubscribeArgs`:** enum with `MarketData`/`Indicator`/`News` variants, derives `Clone + Eq + Hash` for unsubscribe symmetry. `delay` field present **only** on `News` per Nordnet's docs.
- **Order event string fields:** typed enums with `Known + Unknown(String)` variants. Forward-compat preserved (unknown wire values surface as `Unknown`); known values give consumers exhaustive matching.
- **No shared trait between `PublicFeedClient` and `PrivateFeedClient` for v1.** Speculative now; revisit when reconnect lands.
- **`PrivateEvent::TradeRaw`:** untyped `serde_json::Value` for v1. Schema is not in the public Nordnet docs; the Go-model schema is third-party and risky. Type in a follow-up once a live sample lands.
- **`ApiKeyLoginResponse` duplication:** the typed version in `models/login.rs` wins; the loose `auth.rs` version is deleted as part of PR1.
- **Heartbeat with extra fields:** `Heartbeat` accepts and discards extra fields in `data`; does NOT fall through to `Unknown`.
- **Frame size cap:** 1 MiB (designer choice; Nordnet docs do not specify). Configurable in a future revision.

## Open questions deferred

None. Everything required for v1 implementation is specified above.
