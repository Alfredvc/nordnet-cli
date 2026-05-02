# Contracts (LOCKED — do not edit after Phase 0)

## Module layout

Every resource group `<group>` owns exactly:
- `crates/nordnet-api/src/models/<group>.rs`
- `crates/nordnet-api/src/resources/<group>.rs`
- `crates/nordnet-api/tests/<group>_test.rs`
- `crates/nordnet-api/fixtures/<group>/*.json`
- `crates/nordnet-api/docs-extract/<group>/*.md`
- `crates/nordnet-cli/src/cmd/<cli_group>.rs` (Phase 4 only; for the `orders` API group, the CLI splits into `cmd/orders_read.rs` + `cmd/orders_write.rs`, dispatched by foundation-owned `cmd/orders.rs`)

No subagent edits files outside its own group.

## Type rules

- All response structs: `#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]`
- All response structs: `#[serde(deny_unknown_fields)]`
- Optional iff doc parameter table marks optional OR doc example shows null. No speculative `Option`.
- Numeric IDs: use newtype from `crate::ids::*`. Never `i64` / `String` raw.
- Timestamps: `time::OffsetDateTime` with `time::serde::iso8601`.
- Money: `crate::models::shared::Money { amount: rust_decimal::Decimal, currency: Currency }`. Never `f64`.
- Enums: full string set from docs, `#[serde(rename_all = "...")]` matching documented casing. Unknown variant = parse error, by design.
- Doc disagreement (parameter table vs example body vs response schema): pick the most-restrictive interpretation, file in `docs-extract/<group>/<op>.md` under "Doc inconsistencies", surface to reviewer.

## Resource function signature

Each operation is a method on `Client`. Naming: `<verb>_<resource>` (`get_account_info`, `place_order`, `cancel_order`, `list_accounts`).

```rust
impl Client {
    pub async fn get_account_info(&self, accid: AccountId) -> Result<AccountInfo, Error>;
}
```

## Test rules

Two test layers per group:

1. **Fixture roundtrip.** For every fixture, `serde_json::from_str::<T>(fixture)` must succeed AND re-serialize must match canonical form.
2. **Wiremock integration.** For every op, mock the endpoint with the fixture as response body, call the resource fn, assert structure matches.

There is **no** "live" test layer. Pipeline never calls the real API.

## Mod files

Never hand-edit `mod.rs`. Run `cargo xtask gen-mods` after adding a new group file. Pre-commit hook calls this and fails if it produces a diff.

## Commit hygiene

- One commit per group (one subagent, one commit), made by Phase 5, not by the implementer.
- Subject: `feat(<group>): implement <ops list>`.
- Commit hook runs full static gate. Failure blocks commit.
