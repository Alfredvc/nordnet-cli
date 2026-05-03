# Contributing to Nordnet CLI

Thanks for your interest. This document covers the conventions the
codebase follows so a patch lands cleanly.

## Workspace layout

```
crates/
├── nordnet-model/    pure types + Ed25519 SSH-key crypto (no I/O)
├── nordnet-api/      REST client (reqwest, rustls, wiremock-tested)
├── nordnet-feed/     streaming feeds (tokio-rustls, framed codec)
└── nordnet-cli/      the `nordnet` binary
xtask/                workspace task runner (`cargo xtask gen-mods`)
```

Crate dependency graph (no cycles):
`nordnet-model` ← {`nordnet-api`, `nordnet-feed`} ← `nordnet-cli`.

Each REST resource group `<group>` owns:

- `crates/nordnet-model/src/models/<group>.rs` — types
- `crates/nordnet-api/src/resources/<group>.rs` — `Client` methods
- `crates/nordnet-api/tests/<group>_test.rs` — fixture roundtrip + wiremock
- `crates/nordnet-api/fixtures/<group>/*.json` — canonical JSON samples
- `crates/nordnet-cli/src/cmd/<group>.rs` — CLI subcommand
  (the `orders` group splits into `cmd/orders_read.rs` +
  `cmd/orders_write.rs`, dispatched from `cmd/orders.rs`)

## Type rules

- Response structs derive `Debug, Clone, Deserialize, Serialize, PartialEq`.
- Response structs intentionally do **not** carry
  `#[serde(deny_unknown_fields)]` — undocumented server fields are
  silently ignored so a single new field upstream does not break every
  read call. Request structs keep `deny_unknown_fields` to catch our
  own bugs in tests.
- `Option<T>` only when the API genuinely treats the field as optional.
  No speculative `Option`.
- Numeric IDs: newtype from `nordnet_model::ids::*`. Never raw `i64` /
  `String`.
- Timestamps: `time::OffsetDateTime` with `time::serde::iso8601`.
- Money: `nordnet_model::models::shared::Money { amount: Decimal, currency: Currency }`.
  Never `f64`.
- **Decimal JSON form: bare numbers via `arbitrary_precision`.** Every
  `Decimal` field MUST carry
  `#[serde(with = "rust_decimal::serde::arbitrary_precision")]`.
  The workspace `Cargo.toml` enables `serde_json/arbitrary_precision`
  and `rust_decimal/serde-arbitrary-precision` to support this.
  Fixtures use bare JSON numbers (`"tick": 0.01`, not `"tick": "0.01"`).
  Without the `with =` attr, rust_decimal's default serde emits/expects
  strings, breaking canonical byte-equivalent roundtrip. For tuple
  newtypes (e.g. `Amount(pub Decimal)`), put the attr on the field.
- Enums: full string set from the API, `#[serde(rename_all = "...")]`
  matching the documented casing. Unknown variant = parse error, by
  design.

## Resource function signature

Each operation is a method on `Client`. Naming: `<verb>_<resource>`
(`get_account_info`, `place_order`, `cancel_order`, `list_accounts`).

```rust
impl Client {
    pub async fn get_account_info(&self, accid: AccountId) -> Result<AccountInfo, Error>;
}
```

## Tests

Two layers per group:

1. **Fixture roundtrip.** For every fixture, `serde_json::from_str::<T>(fixture)`
   must succeed AND re-serializing the parsed value must equal the
   original fixture's canonical form. Implement as: parse fixture into
   `T`, parse fixture into `serde_json::Value` (`canonical`), serialize
   `T` and parse the result into `serde_json::Value`, assert equal to
   `canonical`. This catches asymmetries like Decimal-as-string fixtures
   vs default Decimal-as-number serialization.
2. **Wiremock integration.** For every op, mock the endpoint with the
   fixture as response body, call the resource fn, assert structure
   matches.

There is **no** "live" test layer. The pipeline never calls the real API.
Live-API verification is the operator's responsibility.

## `mod.rs` files

Never hand-edit. Run `cargo xtask gen-mods` after adding a new file
under a managed directory:

- `crates/nordnet-api/src/resources/`
- `crates/nordnet-cli/src/cmd/`
- `crates/nordnet-model/src/models/`

## Static gates

Before opening a PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- gen-mods   # should be a no-op
```

## License

By contributing, you agree that your contributions will be licensed
under the dual MIT / Apache-2.0 license that covers this project.
