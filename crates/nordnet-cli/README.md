# nordnet-cli

[![crates.io](https://img.shields.io/crates/v/nordnet-cli.svg)](https://crates.io/crates/nordnet-cli)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Agent-friendly command-line frontend for the Nordnet External API v2.

Every subcommand emits a single JSON document on stdout, which makes the
binary easy to script and easy for AI agents to consume. The full
non-deprecated REST surface (~42 operations across 12 resource groups) is
wrapped, including read and write endpoints.

Installs the `nordnet` binary.

## Install

```sh
cargo install nordnet-cli
```

Prebuilt tarballs for Linux and macOS (x86_64 + aarch64) ship with each
GitHub Release: <https://github.com/Alfredvc/nordnet-cli/releases>.

Prerequisite: an OpenSSH-format Ed25519 private key registered with Nordnet:

```sh
ssh-keygen -t ed25519 -f ~/.ssh/nordnet_ed25519
```

## Quick start

```sh
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

`nordnet config` dumps the resolved configuration as JSON (secrets redacted).

## Output format

Stdout always carries a single pretty-printed JSON value. Use
`--fields a,b,c` (a global flag) to restrict the output to a subset of
top-level keys. Pipe through `jq` for richer queries:

```sh
nordnet accounts positions 12345 | jq '.[] | select(.qty > 0)'
```

Errors print a structured JSON document to stderr and the binary exits
non-zero.

## Library access

The REST and feed surfaces are also published as standalone library crates:

- [`nordnet-api`](https://crates.io/crates/nordnet-api) — typed REST client.
- [`nordnet-feed`](https://crates.io/crates/nordnet-feed) — streaming feeds.
- [`nordnet-model`](https://crates.io/crates/nordnet-model) — wire types + crypto (no I/O).

Run `nordnet <subcommand> --help` for the authoritative flag list.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
