# nordnet-api

[![crates.io](https://img.shields.io/crates/v/nordnet-api.svg)](https://crates.io/crates/nordnet-api)
[![docs.rs](https://img.shields.io/docsrs/nordnet-api)](https://docs.rs/nordnet-api)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Typed REST bindings for the Nordnet External API v2.

HTTP-shaped surface only. Wire-typed inputs/outputs (request and response
structs, ID newtypes, the Ed25519 login primitives) live in the sibling
[`nordnet-model`](https://crates.io/crates/nordnet-model) crate; this crate
composes them with `reqwest`-backed HTTP plumbing.

Covers the full non-deprecated REST surface (~42 operations across 12
resource groups). Every operation has a `wiremock` integration test in-tree.

## Install

```sh
cargo add nordnet-api
```

## Usage

```rust,no_run
use nordnet_api::Client;
use nordnet_api::resources::accounts::ListAccountsQuery;
use nordnet_model::auth::Session;

# async fn run() -> Result<(), nordnet_api::Error> {
let client = Client::new("https://public.nordnet.se/api/2")?
    .with_session(Session {
        session_key: std::env::var("NORDNET_SESSION_KEY").unwrap(),
        expires_in: 3600,
    });

let accounts = client.list_accounts(ListAccountsQuery::default()).await?;
for a in accounts {
    println!("{:?}", a);
}
# Ok(())
# }
```

Authentication is the v2 SSH-key flow: `POST /login/start` returns a
challenge, the caller signs the raw UTF-8 bytes with an Ed25519 key, and
`POST /login/verify` returns a `session_key`. The login primitives live in
[`nordnet-model::auth`](https://docs.rs/nordnet-model).

## Companion crates

- [`nordnet-model`](https://crates.io/crates/nordnet-model) — wire-typed inputs/outputs (no I/O).
- [`nordnet-feed`](https://crates.io/crates/nordnet-feed) — streaming public + private feeds.
- [`nordnet-cli`](https://crates.io/crates/nordnet-cli) — `nordnet` binary built on top.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
