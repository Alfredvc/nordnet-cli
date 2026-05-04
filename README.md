# Nordnet CLI

[github.com/Alfredvc/nordnet-cli](https://github.com/Alfredvc/nordnet-cli)

`nordnet` — an agent-friendly command-line frontend for the Nordnet
External API v2.

Every subcommand emits a single JSON document on stdout, which makes the
binary easy to script and easy for AI agents to consume. The full
non-deprecated REST surface (~42 operations across 12 resource groups)
is wrapped, including read and write endpoints. A separate streaming
feed crate covers the public market-data and private account-event
feeds.

## Status

- **REST surface** (`crates/nordnet-api`, `crates/nordnet-cli`):
  feature-complete against Nordnet's published Swagger 2.0
  documentation. Every operation has a wiremock integration test.
  Workspace test count is 270+.
- **Streaming feeds** (`crates/nordnet-feed`): public + private feed
  clients with TCP keepalive, TLS via `tokio-rustls`, heartbeat
  watchdog, framed line codec.
- **No live verification yet.** The test suite never calls Nordnet API
  hosts — every typed call was verified against in-process mocks only.
  Documentation may be stale or wrong; first authenticated round-trip
  against the live API will confirm.

## Install

Prerequisites:
- Rust 1.80 or later (`rustup install stable`).
- An OpenSSH-format Ed25519 private key registered with Nordnet
  (`ssh-keygen -t ed25519 -f ~/.ssh/nordnet_ed25519`).

Build and install the binary into `~/.cargo/bin`:

```bash
cargo install --path crates/nordnet-cli
```

Or build it without installing:

```bash
cargo build --release
./target/release/nordnet --help
```

There is no prebuilt-binary release channel yet.

## Quick start

```bash
# 1. Probe the public root endpoint — no auth required.
nordnet info

# 2. Tell the CLI where to find your credentials.
export NORDNET_API_KEY="your-api-key"
export NORDNET_KEY_PATH="$HOME/.ssh/nordnet_ed25519"

# 3. Run the SSH-key login flow. Persists a session to
#    `<config>/nordnet/session.toml` (mode 0600 on Unix).
nordnet auth login

# 4. Subsequent commands load that session transparently.
nordnet accounts list
nordnet accounts info <accid>
nordnet accounts positions <accid> --fields id,instrument,qty

# 5. Sign out when done.
nordnet auth logout
```

## Configuration

Resolution order, highest priority first:

1. CLI flags (e.g. `--session-key`).
2. Environment variables.
3. `<config_dir>/nordnet/credentials.toml`
   (`$XDG_CONFIG_HOME/nordnet/credentials.toml` on Linux,
   `~/Library/Application Support/nordnet/credentials.toml` on macOS).

| Variable                  | TOML key            | Purpose                                        |
| ------------------------- | ------------------- | ---------------------------------------------- |
| `NORDNET_BASE_URL`        | `base_url`          | API base URL (default `https://public.nordnet.se/api/2`). |
| `NORDNET_API_KEY`         | `api_key`           | API key registered with Nordnet.               |
| `NORDNET_SERVICE`         | `service`           | Service identifier (default `NEXTAPI`).        |
| `NORDNET_KEY_PATH`        | `key_path`          | Path to OpenSSH-format Ed25519 private key.    |
| `NORDNET_DEFAULT_ACCOUNT` | `default_account`   | Account ID used when a subcommand omits it.    |
| `NORDNET_SESSION_KEY`     | —                   | One-shot session override (skips disk file).   |

Run `nordnet config` to dump the resolved configuration as JSON
(secrets redacted) — useful from inside an agent loop to verify the
environment before placing any real calls.

## Authentication

The CLI implements the v2 SSH-key flow exactly as specified in
Nordnet's `nordnet/next-api-v2-examples` reference (`POST /login/start`
returns a challenge → caller signs the raw UTF-8 challenge bytes with
an Ed25519 key → `POST /login/verify` returns a `session_key`).

`nordnet auth login` performs all three steps and writes a
`session.toml` containing `session_key`, `expires_in`, and
`acquired_at`. Every other subcommand picks up that session
automatically. To override for a single call without touching the
on-disk session, pass `--session-key <key>` or set
`NORDNET_SESSION_KEY`.

`nordnet auth status` prints local session metadata without contacting
the API. `nordnet auth refresh` calls `PUT /login` to extend the
server-side session lifetime; `nordnet auth logout` invalidates the
session on the server and removes the local file.

## Output format

Stdout always carries a single pretty-printed JSON value. Use
`--fields a,b,c` (a global flag) to restrict the output to a subset of
top-level keys. The filter applies element-wise to arrays of objects:

```bash
nordnet accounts list --fields id,alias,type
```

The `--fields` filter is intentionally simple — single-level only, no
`jq` query language. Pipe through `jq` for anything richer:

```bash
nordnet accounts positions 12345 | jq '.[] | select(.qty > 0)'
```

Errors print a structured JSON document to stderr and the binary exits
non-zero.

## Security

> **The binary provides no agent containment.** Any process running as
> your UNIX user — including an AI agent you have invoked — can read
> `credentials.toml`, the SSH private key, and `session.toml`, and can
> call every write endpoint (`orders place`, `orders modify`,
> `orders activate`, `orders cancel`) without further authorization.
> Nothing is encrypted at rest, write calls are not gated by
> per-task tokens, and there is no audit log.

Run the binary only inside an isolation boundary you trust (a dedicated
UNIX user, container, VM, or hardware-isolated agent host). Set
`chmod 600` on `credentials.toml` and the SSH key yourself — the CLI
does not do it for you. `nordnet auth logout` after each session.

See [SECURITY.md](SECURITY.md) for the full threat model, file
inventory, and recommended mitigations. The hardened-mode design that
was considered and deferred is preserved at
[`docs/specs/2026-05-03-security-layer-design.md`](docs/specs/2026-05-03-security-layer-design.md).

## Command surface

| Command                        | API path                                              |
| ------------------------------ | ----------------------------------------------------- |
| `nordnet info`                 | `GET /api/2`                                          |
| `nordnet auth {login,logout,refresh,status}` | `POST/DELETE/PUT /login`                |
| `nordnet accounts list`        | `GET /accounts`                                       |
| `nordnet accounts info`        | `GET /accounts/{accid}/info`                          |
| `nordnet accounts ledgers`     | `GET /accounts/{accid}/ledgers`                       |
| `nordnet accounts positions`   | `GET /accounts/{accid}/positions`                     |
| `nordnet accounts returns-today` | `GET /accounts/{accid}/returns/transactions/today`  |
| `nordnet accounts trades`      | `GET /accounts/{accid}/trades`                        |
| `nordnet orders list`          | `GET /accounts/{accid}/orders`                        |
| `nordnet orders place`         | `POST /accounts/{accid}/orders`                       |
| `nordnet orders modify`        | `PUT /accounts/{accid}/orders/{order_id}`             |
| `nordnet orders activate`      | `PUT /accounts/{accid}/orders/{order_id}/activate`    |
| `nordnet orders cancel`        | `DELETE /accounts/{accid}/orders/{order_id}`          |
| `nordnet instruments {get,lookup,types,types-list,leverages,leverage-filters,suitability,trades,underlyings}` | `GET /instruments/...` |
| `nordnet instrument-search {attributes,stocklist,bullbearlist,minifuturelist,optionlist-pairs,unlimitedturbolist}` | `GET /instrument_search/...` |
| `nordnet tradables {info,trades,suitability}` | `GET /tradables/...`                   |
| `nordnet markets {list,get}`   | `GET /markets[/{market_id}]`                          |
| `nordnet news {list,news-sources,get-item}` | `GET /news_items[/{id}]` etc.            |
| `nordnet tick-sizes {list,get}` | `GET /tick_sizes[/{id}]`                             |
| `nordnet countries {list,get}` | `GET /countries[/{country}]`                          |
| `nordnet search <query>`       | `GET /main_search`                                    |
| `nordnet config`               | local — dump resolved config                          |

The two deprecated operations (`GET /accounts/{accid}` and `GET /news`)
are intentionally not surfaced.

`nordnet <subcommand> --help` is the canonical reference for argument
shapes. Every operation accepts `--fields` and `--session-key`.

## Workspace layout

```
nordnet-cli/                  workspace root
├── crates/
│   ├── nordnet-model/        pure types + Ed25519 SSH-key crypto (no I/O)
│   ├── nordnet-api/          REST client (reqwest, rustls, wiremock-tested)
│   ├── nordnet-feed/         streaming feeds (tokio-rustls, framed codec)
│   └── nordnet-cli/          the `nordnet` binary
├── xtask/                    repo automation (`cargo xtask gen-mods`)
├── crates/nordnet-api/fixtures/       per-operation JSON fixtures
└── CONTRIBUTING.md           contributor conventions (type rules, layout)
```

The four crates have no circular dependencies:
`nordnet-model` ← {`nordnet-api`, `nordnet-feed`} ← `nordnet-cli`.
The model crate is I/O-free and may be embedded in non-CLI consumers.

## Development

Static gates before opening a PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Repo automation lives in the `xtask` crate:

```bash
cargo xtask gen-mods   # regenerate every src/.../mod.rs from filesystem
```

`mod.rs` files are generated. Never hand-edit them — re-run `gen-mods`
after adding a new file under a managed directory. See
[CONTRIBUTING.md](CONTRIBUTING.md) for type rules, module layout, and
test conventions.

The test suite never calls `*.nordnet.*` hosts — every typed call is
verified against in-process mocks only. Live-API verification is the
operator's responsibility.

## Type strictness

A handful of invariants from [CONTRIBUTING.md](CONTRIBUTING.md):

- Numeric IDs use newtypes from `nordnet_api::ids::*` — passing an
  `OrderId` where an `AccountId` is expected is a compile error.
- Money is `nordnet_model::shared::Money { amount: Decimal, currency }`
  — never `f64`.
- Timestamps are `time::OffsetDateTime` with ISO-8601 serde.
- Decimal JSON form is bare numbers (`arbitrary_precision`), preserved
  byte-equivalent across roundtrip.
- Response structs intentionally **do not** carry
  `#[serde(deny_unknown_fields)]` — undocumented server fields are
  silently ignored so a single new field upstream does not break every
  read call. Request structs keep `deny_unknown_fields` to catch our
  own bugs in tests.

## License

Dual-licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.
