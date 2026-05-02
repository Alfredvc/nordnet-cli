# Phase 3 — `accounts` group implementation notes

## Ops implemented (6)

| Op | Method | Path | Returns |
|---|---|---|---|
| `list_accounts` | GET | `/accounts` | `Vec<Account>` |
| `get_account_info` | GET | `/accounts/{accid}/info` | `Vec<AccountInfo>` |
| `list_ledgers` | GET | `/accounts/{accid}/ledgers` | `LedgerInformation` |
| `list_positions` | GET | `/accounts/{accid}/positions` | `Vec<Position>` |
| `get_returns_today` | GET | `/accounts/{accid}/returns/transactions/today` | `Vec<AccountTransactionsToday>` |
| `list_account_trades` | GET | `/accounts/{accid}/trades` | `Vec<Trade>` |

The deprecated `GET /accounts/{accid}` (no `/info`) is SKIPPED per the
inventory.

## Naming deviations

- `list_trades` -> `list_account_trades` to coexist with
  `tradables::list_tradable_trades` and `instruments::list_instrument_trades`
  on the single `Client` impl. Mirrors the precedent set in
  `resources/instruments.rs`. Documented at the top of
  `resources/accounts.rs`.

## Path deviation from task brief

The task brief proposed `/accounts/{accid}/returns/today` for
`get_returns_today` but the saved HTML schema documents the path as
`/accounts/{accid}/returns/transactions/today`. Per CONTRACTS.md priority
#1 (documentation faithfulness) we use the documented path. The
`docs-extract/accounts/get_returns_today.md` slice is unchanged.

## Doc inconsistencies

None identified during Phase 1; none surfaced during Phase 3
implementation.

## Local types flagged for Phase 3X promotion

The `accounts` group needed several types that either don't exist in
`crate::models::shared` (locked) or whose foundation forms don't match
the documented schema. These are defined LOCALLY in
`crates/nordnet-api/src/models/accounts.rs` and flagged here:

1. **`Amount`** (the documented `Amount` schema, `{currency, value}`)
   - Conflicts with foundation `crate::models::shared::Amount` which is
     a bare `Decimal` newtype, AND with `shared::Money` which uses field
     name `amount` (not `value`).
   - Local `Amount` keeps `value: Decimal` (with `arbitrary_precision`)
     per CONTRACTS.md.
   - Phase 3X candidate: rename foundation `shared::Amount` -> something
     else and either promote this local `Amount` (or `shared::Money` with
     a `value` alias) to `shared.rs`.

2. **`PositionInstrument`** (the `Instrument` shape used by
   `Position.instrument`)
   - The full `Instrument` schema lives in `models/instruments.rs`. We
     re-derive a local copy here per the module-ownership rule, exposing
     the documented required fields plus the commonly-populated optional
     ones.
   - Phase 3X candidate: extract a shared `Instrument` (potentially
     under `models/shared.rs` if locked-after-Phase-0 status is relaxed,
     or as a top-level `models/instrument.rs` shared module).

3. **`TradableRef`** (the `TradableId` *schema object*,
   `{identifier, market_id}`)
   - The foundation `crate::ids::TradableId` is a bare-string newtype —
     a different concept from this object form.
   - Phase 3X candidate: either rename `crate::ids::TradableId` to
     `TradableIdentifier` (clearer) and promote `TradableRef` to
     `TradableId` in `shared.rs`, or keep both with clearer module
     namespacing.

4. **`Reserved`** (small struct nested inside `AccountInfo`)
   - Used only by `accounts`; no cross-group reuse expected. Probably
     stays local.

## Test coverage

- Layer 1 (fixture roundtrip): 6 fixtures + Decimal precision survival
  test + TradableRef object-shape test.
- Layer 2 (`deny_unknown_fields` rejection): 4 explicit tests covering
  `Account`, `AccountInfo`, `Position`, `Trade`. Helper-construction
  test for `Ledger`, `Reserved`, `PositionInstrument`.
- Layer 3 (wiremock integration): 19 tests — every op has a success +
  ≥1 error mapping. 4 ops (`list_accounts`, `list_positions`,
  `get_returns_today`, `list_account_trades`) cover the 204 No Content
  -> empty `Vec` mapping. `list_account_trades` asserts the `days`
  query param is forwarded; query-flag forwarding is also asserted for
  `list_accounts`, `get_account_info`, `list_positions`,
  `get_returns_today`.

Total: 34 tests, all green.

## Static gates

- `cargo fmt --package nordnet-api --check` — clean.
- `cargo clippy --package nordnet-api --tests -- -D warnings` — clean.
- `cargo test --package nordnet-api --test accounts_test` — 34/34
  passing.
- `cargo run --package xtask -- gen-mods` — re-ran, only updated
  `resources/mod.rs` to add the new `accounts` entry; `models/mod.rs`
  was already in shape (alphabetically-sorted addition after the
  pre-existing entries).

## Open questions

- None blocking. Optional follow-ups for Phase 3X listed under "Local
  types flagged for Phase 3X promotion" above.
