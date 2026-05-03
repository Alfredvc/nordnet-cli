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
