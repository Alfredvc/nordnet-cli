# Phase 3 — `instruments` group

## Ops implemented (9)

| Method | Path | Rust fn |
|---|---|---|
| GET | `/instruments/lookup/{lookup_type}/{lookup}` | `lookup` |
| GET | `/instruments/types` | `list_types` |
| GET | `/instruments/types/{instrument_type}` | `get_type` |
| GET | `/instruments/underlyings/{derivative_type}/{currency}` | `list_underlyings` |
| GET | `/instruments/validation/suitability/{instrument_id}` | `get_instrument_suitability` |
| GET | `/instruments/{instrument_id}` | `get_instrument` |
| GET | `/instruments/{instrument_id}/leverages` | `list_leverages` |
| GET | `/instruments/{instrument_id}/leverages/filters` | `get_leverage_filters` |
| GET | `/instruments/{instrument_id}/trades` | `list_instrument_trades` |

## Naming overrides

Two ops renamed so they coexist on `Client`:

- `list_trades` → `list_instrument_trades` (avoid clash with `tradables::list_tradable_trades` and future `accounts::list_trades`)
- `get_suitability` → `get_instrument_suitability` (avoid clash with `tradables::get_suitability`)

Phase 3X may pick a uniform scheme.

## Doc inconsistencies / decisions

- `IssuerId` newtype declared locally because foundation `ids` module is locked. Phase 3X candidate for promotion.
- `Instrument.expiration_date` left as `String` (not `time::Date`); deferred to Phase 3X.
- `Tradable.identifier` carried as `String`; Phase 3X may swap to `TradableId`.
- `PublicTrade` shape duplicated vs `tradables`; Phase 3X reconciliation candidate.
- `lookup_type`, `derivative_type`, `market_view` are documented enums but the resource API takes `&str`. Input enum typing deferred to CLI phase.
- 204 No Content is mapped to empty `Vec` for all `Vec<T>`-returning ops (mirrors `tradables`); `get_leverage_filters` (single `LeverageFilter`) does not map 204.

## Tests

38 tests, all green. Each op has wiremock success path + at least one error mapping (400/401/403/204 as applicable). Every fixture has a roundtrip test. `LeveragesQuery` has dedicated query-string encoding tests including percent-encoding.

## Reviewer outcome

Approved (Phase 3R). Minor observations recorded above as Phase 3X follow-ups.
