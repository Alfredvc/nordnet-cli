# nordnet-cli

[![crates.io](https://img.shields.io/crates/v/nordnet-cli.svg)](https://crates.io/crates/nordnet-cli)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Agent-friendly command-line frontend for the Nordnet External API v2.

Every subcommand emits a single pretty-printed JSON document on stdout —
trivial to script, trivial for AI agents to consume. The full
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

## Output contract

Designed to be parsed without surprises. The contract is:

- **Stdout** carries exactly one pretty-printed JSON value per
  invocation, terminated by a trailing newline. No banners, no progress
  spinners, no ANSI colour codes.
- **Stderr** carries human-readable diagnostics and, on failure, a
  structured JSON error document.
- **Exit codes**:

  | Code | Meaning                                                    |
  | ---- | ---------------------------------------------------------- |
  | `0`  | Success.                                                   |
  | `1`  | Any unhandled error — request failure, parse error, I/O.   |

  Distinguishing an HTTP 401 from an HTTP 500 is done by inspecting the
  stderr JSON document, not the exit code.
- **`--fields a,b,c`** is a global flag that restricts the output to a
  subset of top-level keys, preserving the requested order. Arrays of
  objects are filtered element-wise; scalar payloads with `--fields` set
  exit non-zero (`FilterInapplicable`).

### Stability promise

Within a `0.x` minor version series:

- Output schemas are **append-only**. Top-level field names will not be
  renamed or removed without a minor-version bump.
- Subcommand names, flag names, and exit codes are stable.
- New optional flags and new fields may be added at any time.

A `0.x → 0.(x+1)` bump may rename or remove fields and is documented in
the changelog. Pin loosely (`nordnet-cli = "0.1"`) to follow patch
releases automatically; pin exactly (`= 0.1.2`) if you cannot tolerate
schema additions.

### jq examples

```sh
# Open positions only
nordnet accounts positions 12345 | jq '.[] | select(.qty > 0)'

# All active orders, just the fields that matter
nordnet orders list 12345 \
    --fields order_id,side,price,volume,state \
    | jq '.[] | select(.state == "ACTIVE")'

# Resolve an account by alias
nordnet accounts list | jq -r '.[] | select(.alias=="ISK").accid'
```

## For AI agents and scripts

`nordnet` is designed to be driven by code as a first-class use case:

- **Single-shot, deterministic output.** No interactive prompts, no
  pagers, no colour. `nordnet <subcommand> --help` is the authoritative
  flag list — every subcommand carries a `long_about` description and an
  `EXAMPLES:` block with concrete invocations.
- **Stable JSON contract.** See the stability promise above.
- **Composable with `jq` / `jaq` / `gron`.** Output is always one JSON
  value, so any structural query tool works without preprocessing.
- **Field projection.** `--fields a,b,c` keeps the prompt window small
  by trimming responses to exactly what the agent needs — a 200-row
  positions response fits in a single tool result this way.
- **Idempotent reads.** Every read endpoint is safe to retry. Write
  endpoints (`auth login`, `orders place|modify|activate|cancel`) are
  documented per-subcommand and produce JSON that includes the resulting
  IDs so the agent can chain calls.
- **Structured errors.** Failures emit a JSON document on stderr (not
  stdout) so success output is never accidentally polluted on retry
  loops.

Recommended agent loop:

```sh
nordnet auth status --fields status \
    | jq -e '.status == "logged_in"' >/dev/null \
    || nordnet auth login

# Now safe to issue authenticated calls.
nordnet accounts list --fields accid,alias
```

## Shell completions

`nordnet completions <shell>` writes a completion script to stdout.
Generated at runtime from the live clap definition, so the script always
matches the installed binary.

```sh
# Bash (Linux)
nordnet completions bash > ~/.local/share/bash-completion/completions/nordnet

# Zsh — append to a directory in $fpath
nordnet completions zsh > "${fpath[1]}/_nordnet"

# Fish
nordnet completions fish > ~/.config/fish/completions/nordnet.fish

# PowerShell
nordnet completions powershell | Out-String | Invoke-Expression

# Elvish
nordnet completions elvish > ~/.config/elvish/lib/nordnet.elv
```

## Library access

The REST and feed surfaces are also published as standalone library crates:

- [`nordnet-api`](https://crates.io/crates/nordnet-api) — typed REST client.
- [`nordnet-feed`](https://crates.io/crates/nordnet-feed) — streaming feeds.
- [`nordnet-model`](https://crates.io/crates/nordnet-model) — wire types + crypto (no I/O).

Run `nordnet <subcommand> --help` for the authoritative flag list.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
