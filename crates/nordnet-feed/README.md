# nordnet-feed

[![crates.io](https://img.shields.io/crates/v/nordnet-feed.svg)](https://crates.io/crates/nordnet-feed)
[![docs.rs](https://img.shields.io/docsrs/nordnet-feed)](https://docs.rs/nordnet-feed)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Streaming feeds for the Nordnet External API v2.

Sibling to [`nordnet-api`](https://crates.io/crates/nordnet-api). Both crates
share [`nordnet-model`](https://crates.io/crates/nordnet-model) types but have
independent transports — no `reqwest` here. Two feed types:

- `PublicFeedClient` — market data subscriptions.
- `PrivateFeedClient` — account/order events (auto-pushed after login).

## Install

```sh
cargo add nordnet-feed
```

## Usage

```rust,no_run
use nordnet_feed::{MarketDataKind, PublicFeedClient, SubscribeArgs};
use nordnet_model::models::login::Feed;

# async fn run() -> Result<(), nordnet_feed::FeedError> {
let feed = Feed {
    encrypted: true,
    hostname: "public.feed.nordnet.se".to_owned(),
    port: 443,
};
let mut client = PublicFeedClient::connect(&feed).await?;

client.subscribe(SubscribeArgs::MarketData {
    kind: MarketDataKind::Price,
    market: 11,
    identifier: "101".to_owned(),
}).await?;

while let Some(event) = client.recv().await? {
    println!("{event:?}");
}
# Ok(())
# }
```

## Production hardening

- TCP `SO_KEEPALIVE` configured at connect time (kernel-level dead-peer
  detection ~60s).
- `TCP_NODELAY` enabled (low-latency command writes).
- Connect timeout (default 10s) bounds combined TCP + TLS handshake time.
- Heartbeat watchdog (default 15s) detects half-open connections that
  survive kernel-level keepalive.

Override defaults via `FeedConfig` + `connect_with`.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
