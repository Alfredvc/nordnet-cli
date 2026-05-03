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

## Resource groups

Each group is a module under [`resources`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/);
every operation hangs off `Client` and carries a `#[doc(alias = "METHOD /path")]`
so rustdoc search resolves raw HTTP paths.

| Group | Ops | Coverage |
|-------|-----|----------|
| [`accounts`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/accounts/) | 6 | accounts, info, ledgers, positions, returns, trades |
| [`countries`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/countries/) | 2 | list + lookup by ISO code |
| [`instrument_search`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/instrument_search/) | 6 | attributes + stock/bullbear/minifuture/turbo/option searches |
| [`instruments`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/instruments/) | 9 | lookup, types, underlyings, leverages, suitability, trades |
| [`login`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/login/) | 4 | start, verify, refresh, logout |
| [`main_search`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/main_search/) | 1 | unified instruments/news/CMS search |
| [`markets`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/markets/) | 2 | list + lookup by ID |
| [`news`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/news/) | 2 | article fetch + sources |
| [`orders`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/orders/) | 5 | list, place, modify, activate, cancel |
| [`root`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/root/) | 1 | system status |
| [`tick_sizes`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/tick_sizes/) | 2 | list + lookup by ID |
| [`tradables`](https://docs.rs/nordnet-api/latest/nordnet_api/resources/tradables/) | 3 | info, trades, suitability |

## Companion crates

- [`nordnet-model`](https://crates.io/crates/nordnet-model) — wire-typed inputs/outputs (no I/O).
- [`nordnet-feed`](https://crates.io/crates/nordnet-feed) — streaming public + private feeds.
- [`nordnet-cli`](https://crates.io/crates/nordnet-cli) — `nordnet` binary built on top.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
