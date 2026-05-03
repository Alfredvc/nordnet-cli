# nordnet-model

[![crates.io](https://img.shields.io/crates/v/nordnet-model.svg)](https://crates.io/crates/nordnet-model)
[![docs.rs](https://img.shields.io/docsrs/nordnet-model)](https://docs.rs/nordnet-model)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Pure data types and crypto for the Nordnet External API v2.

Zero I/O dependencies (no `reqwest`, no `tokio`, no `tokio-rustls`). Hosts:

- `auth` — Ed25519 SSH-key login flow primitives plus the `auth::Session`
  newtype.
- `models` — serde structs for every documented request and response shape,
  organised per resource group.
- `ids` — newtype wrappers for resource identifiers.
- `error::AuthError` — error type covering only what `auth` can fail at.

Both [`nordnet-api`](https://crates.io/crates/nordnet-api) (REST client) and
[`nordnet-feed`](https://crates.io/crates/nordnet-feed) (streaming client)
depend on this crate for shared wire-typed inputs and outputs. The crate is
I/O-free and may be embedded in non-CLI consumers.

## Install

```sh
cargo add nordnet-model
```

## Usage

```rust,no_run
use nordnet_model::auth::Session;
use nordnet_model::ids::AccountId;

let session = Session {
    session_key: "abc123".to_owned(),
    expires_in: 3600,
};
let acc = AccountId::from(12345_i64);
# let _ = (session, acc);
```

Sign a login challenge with an OpenSSH-format Ed25519 private key:

```rust,no_run
use nordnet_model::auth;

let pem = std::fs::read_to_string("/home/me/.ssh/nordnet_ed25519")?;
let signing_key = auth::parse_private_key_openssh(&pem)?;
let signature_b64 = auth::sign_challenge(&signing_key, "challenge-bytes")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Type strictness

- Numeric IDs use newtypes — passing an `OrderId` where an `AccountId` is
  expected is a compile error.
- Money is `Money { amount: Decimal, currency }` — never `f64`.
- Timestamps are `time::OffsetDateTime` with ISO-8601 serde.
- Decimal JSON form is bare numbers (`arbitrary_precision`), preserved
  byte-equivalent across roundtrip.
- Response structs do **not** carry `#[serde(deny_unknown_fields)]` —
  undocumented server fields are silently ignored. Request structs keep
  `deny_unknown_fields` to catch local bugs in tests.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
