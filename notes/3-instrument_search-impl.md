# Phase 3 — `instrument_search` group implementation notes

## Ops implemented (6, all GET)

| Op                            | Path                                                  | Returns                       |
|-------------------------------|-------------------------------------------------------|-------------------------------|
| `get_attributes`              | `/instrument_search/attributes`                       | `AttributeResults`            |
| `search_stocklist`            | `/instrument_search/query/stocklist`                  | `StocklistResults`            |
| `search_bullbearlist`         | `/instrument_search/query/bullbearlist`               | `BullBearListResults` (204 -> empty) |
| `search_minifuturelist`       | `/instrument_search/query/minifuturelist`             | `MinifutureListResults`       |
| `search_unlimitedturbolist`   | `/instrument_search/query/unlimitedturbolist`         | `UnlimitedTurboListResults`   |
| `search_optionlist_pairs`     | `/instrument_search/query/optionlist/pairs`           | `OptionListResults`           |

## Doc inconsistencies & resolutions

None that required deviation. Per-op `docs-extract/instrument_search/*.md`
slices recorded "_(none identified during Phase 1 extraction)_". Schema
fields used as-is; no contradictory parameter/response evidence to weigh.

## Fixture provenance

All six fixtures synthesized directly from each op's response schema
table. Meta files use `fixture_provenance = "synthesized_from_schema"`
with the per-op `schema_source` anchor pointing at the relevant
`docs-extract/_definitions/*.md` entry.

## Phase 3X candidates (cross-group reconciliation)

The following structurally identical types are now defined in **two or
more groups** and should be considered for promotion to
`crate::models::shared` (or for a single-group definition that the others
import) during Phase 3X:

| Type                | Defined in                                                  |
|---------------------|-------------------------------------------------------------|
| `EtpInfo`           | `models::main_search`, `models::instrument_search`          |
| `KoInfo`            | `models::main_search`, `models::instrument_search`          |
| `PriceKoInfo`       | `models::main_search`, `models::instrument_search`          |
| `MarketInfo`        | `models::main_search`, `models::instrument_search`          |
| `PriceWithDecimals` | `models::main_search`, `models::instrument_search`          |
| `opt_arb_prec` mod  | `models::main_search`, `models::instruments`, `models::instrument_search` |

Additionally, the bare-`number` fields on `OptionInfo`
(`risk_free_interest`, `strike_price`) and the bare-`number` field on
`OptionlistPair` (`strike_price`) are typed as `Decimal` per the
"never `f64`" rule. The schema does not specify decimal precision; the
`arbitrary_precision` adapter preserves whatever the wire actually sends.

### Ad-hoc local types (no foundation home today)

- `IssuerId`: not promoted to `crate::ids` (mirrors the existing
  `models::instruments::IssuerId` precedent). `instrument_search` does
  not need its own newtype because `InstrumentInfo.issuer_id` is typed as
  plain `i64` per CONTRACTS.md "no speculative options"-equivalent
  pragma — there is no clear group-ownership for the newtype and
  promoting it under a single group from this slice would be premature.
  Phase 3X may pick a home.
- Several `integer(int64)` timestamp-shaped fields stay as plain `i64`
  for the same reason `main_search` chose plain `i64` (no
  `Timestamp`-for-epoch-millis newtype exists under `shared`).

### Ops that documented multi-value query parameters

`get_attributes` has three `< … > array` query parameters
(`attribute_group`, `expand`, `minmax`). The resource forwards them as
repeated `name=value` query pairs (mirrors how `reqwest` encodes a
`Vec`). Tested in `get_attributes_with_filters_forwards_query`.
`search_stocklist` similarly has `attribute_groups` and `attributes`.

### 204 No Content

Only `search_bullbearlist` documents 204. The resource maps the empty
body to `BullBearListResults { results: None, rows: None, total_hits:
None, underlying_instrument_id: None }` — a "completely empty" results
wrapper — rather than to an empty `Vec`, since the response shape is a
struct, not an array. Mirrors the `instruments` group's 204-handling
pattern (which mapped to empty `Vec` because every instruments op
returned `Vec<T>`).

## Open questions

- Should the `*Query` builders be unified across groups via a generic
  helper? Currently each group ships its own; `instruments::LeveragesQuery`,
  `instrument_search::AttributesQuery`, etc. Phase 3X / a follow-up
  refactor could centralise the pattern under `pagination` or a new
  `query` module.
- `OptionlistPair.strike_price` is `required` but documented only as
  `number` (not `number(double)`). Treated as `Decimal` here. If the
  live API actually emits an integer literal for round strikes, the
  `arbitrary_precision` adapter still parses it correctly.

## Files written

- `crates/nordnet-api/src/models/instrument_search.rs`
- `crates/nordnet-api/src/resources/instrument_search.rs`
- `crates/nordnet-api/tests/instrument_search_test.rs`
- `crates/nordnet-api/fixtures/instrument_search/get_attributes.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/instrument_search/search_stocklist.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/instrument_search/search_bullbearlist.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/instrument_search/search_minifuturelist.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/instrument_search/search_unlimitedturbolist.{response.json,meta.toml}`
- `crates/nordnet-api/fixtures/instrument_search/search_optionlist_pairs.{response.json,meta.toml}`
- `notes/3-instrument_search-impl.md`

## Verification commands run (all green)

- `cargo fmt --package nordnet-api --check`
- `cargo clippy --package nordnet-api --tests -- -D warnings`
- `cargo test --package nordnet-api --test instrument_search_test` — 24 tests pass
- `cargo run -p xtask -- gen-mods` — added `instrument_search` to both
  `models/mod.rs` and `resources/mod.rs`
