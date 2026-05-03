# PR1 — `nordnet-model` extraction

Single atomic commit. Extracts pure types + pure-compute crypto out of
`nordnet-api` into a new sibling crate, leaving the REST client crate
holding only HTTP-shaped surface.

## What moved

| Original (`crates/nordnet-api/src/`)               | New home (`crates/nordnet-model/src/`)            |
|----------------------------------------------------|---------------------------------------------------|
| `auth.rs`                                          | `auth.rs` (sans loose `ApiKeyLoginResponse`)      |
| `ids.rs`                                           | `ids.rs` (verbatim)                               |
| `models/accounts.rs`                               | `models/accounts.rs`                              |
| `models/countries.rs`                              | `models/countries.rs`                             |
| `models/instrument_search.rs`                      | `models/instrument_search.rs`                     |
| `models/instruments.rs`                            | `models/instruments.rs`                           |
| `models/login.rs`                                  | `models/login.rs` (canonical `ApiKeyLoginResponse`) |
| `models/main_search.rs`                            | `models/main_search.rs`                           |
| `models/markets.rs`                                | `models/markets.rs`                               |
| `models/news.rs`                                   | `models/news.rs`                                  |
| `models/orders.rs`                                 | `models/orders.rs`                                |
| `models/root.rs`                                   | `models/root.rs`                                  |
| `models/shared.rs`                                 | `models/shared.rs`                                |
| `models/tick_sizes.rs`                             | `models/tick_sizes.rs`                            |
| `models/tradables.rs`                              | `models/tradables.rs`                             |

New file in `crates/nordnet-model/src/`:

- `lib.rs` — declares `pub mod auth; pub mod models; pub mod ids; pub mod error;` plus `pub use error::AuthError;`.
- `error.rs` — `AuthError` enum (`InvalidKey`, `EncryptedKey`, `WrongAlgorithm{got,expected}`, `KeyDataMismatch`).

## Files deleted

- `crates/nordnet-api/src/auth.rs`
- `crates/nordnet-api/src/ids.rs`
- `crates/nordnet-api/src/models/` (entire directory)

## Public-API deltas

1. **`ApiKeyLoginResponse` collapsed.** The loose `nordnet_api::auth::ApiKeyLoginResponse`
   (with `Option<serde_json::Value>` feeds) is gone. Callers now reach the
   typed canonical version at `nordnet_model::models::login::ApiKeyLoginResponse`
   (with required `Feed { encrypted, hostname, port }` for both `private_feed`
   and `public_feed`). `to_session()` lives on the canonical type.
2. **`Error::Auth` swap.** `nordnet_api::Error::Auth(String)` →
   `Error::Auth(#[from] nordnet_model::AuthError)`. Old call sites that
   passed strings now go through the typed enum; the `#[from]` lets the
   `?` operator bridge the two boundaries automatically.
3. **`parse_private_key_openssh` return type.** Was
   `Result<SigningKey, nordnet_api::Error>`; now
   `Result<SigningKey, nordnet_model::AuthError>`. CLI call sites
   compose via the existing `?` operator + the `#[from]` impl.
4. **`auth` module slimmed.** `auth.rs` no longer hosts
   `ApiKeyLoginResponse` (the type and its `to_session()` impl moved to
   `models/login.rs`). The auth module retains `Session`,
   `sign_challenge`, `parse_private_key_openssh`,
   `ApiKeyStartLoginRequest`, `ApiKeyVerifyLoginRequest`,
   `ChallengeResponse`.
5. **`crates/nordnet-api/src/lib.rs` re-exports.** No longer declares
   `pub mod auth`, `pub mod models`, or `pub mod ids` — consumers must
   reach those via `nordnet_model::*`. `client::Client` and `error::Error`
   re-exports stay where they were.

## Workspace plumbing

- New crate `crates/nordnet-model` added to workspace `members` in `/Cargo.toml`.
- `nordnet-model = { path = "../nordnet-model" }` added to `crates/nordnet-api/Cargo.toml` `[dependencies]`.
- `nordnet-model = { path = "../nordnet-model" }` added to `crates/nordnet-cli/Cargo.toml` `[dependencies]`.
- `xtask`'s `MOD_DIRS` extended with `crates/nordnet-model/src/models`
  (uses the same `ManagedKind::ModelsApi` rule as `nordnet-api`'s models
  directory).

## Test count

- Before: 247
- After: 247

Net delta is zero. Internally one auth-level unit test was retired
(`api_key_login_response_minimal`, which exercised the loose
`Option<serde_json::Value>` shape that no longer exists) and one
replacement was added (`api_key_verify_login_request_round_trip`,
exercising the auth-level request shape that `nordnet-model` still
owns).

## Verification

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — 247 passed, 0 failed.
- `cargo metadata` — `nordnet-model` has zero HTTP/transport deps
  (no `reqwest`, `tokio`, or `tokio-rustls`).

## Rework round 1

Two follow-ups landed inside the same PR1 commit (no separate landing —
PR1 is still atomic).

### A. Dedup login request types — `auth.rs` wins

Round 0 had left `ApiKeyStartLoginRequest`, `ApiKeyVerifyLoginRequest`,
and `ChallengeResponse` defined in **both** `auth.rs` and `models/login.rs`
(mirroring the pre-extraction state of the codebase). Per spec design
line 64 these belong with the signing primitives in `auth`, so the
`models/login.rs` copies were deleted. `models/login.rs` now hosts only
`Feed`, `ApiKeyLoginResponse` (with its `to_session()` impl), and
`LoggedInStatus`.

Call sites repointed:

- `crates/nordnet-api/src/resources/login.rs` — pulls the three from
  `nordnet_model::auth::*`, `ApiKeyLoginResponse` + `LoggedInStatus` from
  `nordnet_model::models::login::*`.
- `crates/nordnet-api/tests/login_test.rs` — same split.
- `crates/nordnet-api/tests/client_test.rs` — same split.
- `crates/nordnet-cli/src/cmd/auth.rs` — pulls everything from
  `nordnet_model::auth::*` (no longer needs `models::login`).

The auth-module types are already `pub`, so the resources crate
consumes them without extra re-export plumbing.

### B. Cleanup unused / dev-only deps in `nordnet-api`

`crates/nordnet-api/Cargo.toml` cleanup:

- Removed `ed25519-dalek` from `[dependencies]` — fully unused after the
  extraction (no production code or tests touch it).
- Moved `ssh-key` from `[dependencies]` to `[dev-dependencies]` — only
  used by `tests/client_test.rs:156` to construct an OpenSSH PEM in-test.
- Moved `base64` from `[dependencies]` to `[dev-dependencies]` — only
  used by `tests/login_test.rs:16` for the `Authorization: Basic …`
  header assertion.

### Verification (post-rework)

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — 247 passed, 0 failed.
