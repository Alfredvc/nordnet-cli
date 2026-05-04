# Security Layer Design

Status: draft for review
Date: 2026-05-03

## Problem

`nordnet-cli` is an agent-facing CLI that wraps Nordnet's brokerage REST
API, including write operations that move real money (place / modify /
activate / cancel orders). Today it stores raw long-lived credentials
(`api_key`, OpenSSH Ed25519 private key) and the active session key in
cleartext on disk. An AI agent invoking the binary has the full surface
of the API at its disposal with no user-defined limits.

Two failure modes drive this work:

1. **Buggy or curious agent** invokes a write subcommand it should not
   have, with parameters the user never sanctioned.
2. **Adversarial behavior in-process**: agent reads disk, calls API
   directly, bypasses any best-effort gating inside the CLI.

Nordnet's API itself offers no scopes, no read-only key, no sandbox
([api.test.nordnet.se docs](https://api.test.nordnet.se/api-docs/index.html)),
so all enforcement must live client-side.

## Goals

- A user-controlled, per-task **capability token** gates every write
  operation. Tokens carry caveats (scope, account, per-trade notional
  cap, aggregate notional cap, instrument, expiry) and are signed by a
  key the agent cannot mint without the user's password.
- A **user-session daemon** holds the live Nordnet session key in memory
  so it never appears on disk.
- Long-lived secrets (Ed25519 SSH key, `api_key`, mint private key) are
  encrypted at rest with a key derived from the user's password.
- All write code paths route through one verification function.
- A separately-named binary, `nordnet-unsafe`, ships from a separate
  crate (`nordnet-cli-unsafe`) for users who deliberately accept the
  weaker model — daemonless, session key on disk.

## Non-goals

- Defending against root on the host.
- Defending against a tampered binary (password capture, MitM of prompt,
  `LD_PRELOAD`, in-process `unsafe` synthesizing verification proofs).
- Cross-host session sharing.
- Multi-account simultaneous tokens (single active token only).
- Token attenuation / delegation.
- Windows support in v1.
- Hardware-key escalation tier (YubiKey, Secure Enclave) in v1.

## Threat model

| Level | Description | `nordnet` (daemon) | `nordnet-unsafe` |
|---|---|---|---|
| L1 | Buggy: agent calls wrong endpoint | ✓ token gate | ✓ token gate |
| L2a | Curious: agent reads long-lived secrets at rest | ✓ encrypted | ✓ encrypted |
| L2b | Curious: agent reads active token file | ✗ usable until expiry (mode 0600) | ✗ same |
| L3 | Malicious agent invoking the binary with crafted argv | ✓ verify chokepoint | ✓ same |
| L4 | Active extraction of session key | ✓ daemon memory | ✗ session.toml on disk |
| L0 | Read-side exfil via legitimate read endpoints | ✗ reads not gated | ✗ same |

L2b mitigation is operational: short TTLs (default 1 h) and explicit
revocation via `auth revoke`. `nordnet-unsafe`'s name is the L4
warning.

## Architecture

### Crate layout

```
crates/
├── nordnet-model/         existing — adds src/token/ submodule
│                          (Claims, verify, mint — pure crypto, no I/O)
├── nordnet-api/           existing — adds resource methods for mutating ops
├── nordnet-feed/          existing, untouched
├── nordnet-cli/           refactored: lib + 2 bins
│   ├── src/lib.rs         clap tree, Operation, Dispatcher trait,
│   │                      SecretsBackend trait, run()
│   ├── src/secure/        crypto (KEK), paths, password prompt, mint CLI
│   ├── src/daemon/        IPC server, prctl, memfd_secret
│   ├── src/client/        DaemonClient (impls Dispatcher + SecretsBackend
│   │                      via IPC)
│   ├── src/bin/nordnet.rs            daemon-client binary
│   └── src/bin/nordnet-daemon.rs     daemon binary
└── nordnet-cli-unsafe/    NEW — bin only, depends on nordnet-cli (lib)
                           implements Dispatcher + SecretsBackend directly
```

Dependency graph (no cycles):

```
nordnet-model ←┬─ nordnet-api ─┬─ nordnet-cli (lib + 2 bins)
               └─ nordnet-feed └─ nordnet-cli-unsafe (bin) → nordnet-cli (lib)
```

`nordnet-model::token` lives in the model crate. Charter relaxes from
"pure types, minimal deps" to "pure logic + pure crypto, no I/O" —
`pasetors`, `ed25519-dalek` enter here.

**Defense against accidental daemon code in `nordnet-unsafe`**:
Cargo feature gating + code review.

- `nordnet-cli`'s `Cargo.toml` puts daemon and client modules behind
  a `daemon` feature, on by default:
  ```toml
  [features]
  default = ["daemon"]
  daemon  = ["dep:tokio-uds", "dep:nix"]    # actual deps as needed
  ```
  `daemon::*` and `client::*` modules in the lib carry
  `#![cfg(feature = "daemon")]`. The `nordnet` and `nordnet-daemon`
  bins require the feature; the `nordnet daemon ...` clap subtree
  is also `#[cfg(feature = "daemon")]`.
- `nordnet-cli-unsafe`'s dep line disables the feature:
  ```toml
  nordnet-cli = { path = "../nordnet-cli", default-features = false }
  ```
  Cargo will not link daemon/client code into the unsafe binary —
  the modules are physically absent from the compilation unit.
- Code review on the small (~400-line) `nordnet-cli-unsafe` source
  catches any reintroduction (e.g. a contributor adding `daemon` to
  the dep line). The crate's `main.rs` only imports
  `nordnet_cli::{run, Operation, Dispatcher, SecretsBackend,
  secure}` plus its own `DirectClient` / `DirectSecrets`.
- No symbol-table xtask, no pinned toolchain. Cargo's feature
  resolution is the load-bearing gate; review is the second line.

### Crate responsibilities

#### `nordnet-cli` lib

- **Clap command tree** — every subcommand defined once.
- **`Operation` enum** — discriminated union of every supported
  request, parameterized by typed args from `nordnet-model`.
- **`Dispatcher` trait** — handles the API-call path:

  ```rust
  #[async_trait]
  pub trait Dispatcher {
      async fn execute(
          &self,
          op: Operation,
          token: Option<String>,
      ) -> Result<serde_json::Value, DispatchError>;
  }
  ```

- **`SecretsBackend` trait** — auth ceremonies:

  ```rust
  #[async_trait]
  pub trait SecretsBackend {
      async fn login   (&self, password: SecretString) -> Result<SessionInfo, SecretsError>;
      async fn refresh (&self) -> Result<SessionInfo, SecretsError>;
      async fn logout  (&self) -> Result<(), SecretsError>;
      async fn mint    (&self, password: SecretString, caveats: Caveats) -> Result<String, SecretsError>;
      async fn status  (&self) -> Result<DaemonStatus, SecretsError>;
  }
  ```

- **`run<I, D, S>(args: I, dispatcher: D, secrets: S) -> ExitCode`** —
  single entry point used by both binaries. Parses args, loads token
  for mutating ops, calls either `dispatcher.execute` or
  `secrets.<method>`, prints JSON result.

- **`secure::` modules** (require `#[allow(unsafe_code)]`):
  - At-rest encryption: argon2id (m=19MiB, t=2, p=1, OWASP minimum)
    KEK; XChaCha20-Poly1305 ciphertext.
  - Password prompt: TTY-direct via `rpassword` (opens `/dev/tty`).
  - Token mint: PASETO v4 public tokens. Logic lives in
    `nordnet_model::token::mint`; this module is CLI plumbing.
  - Storage paths: resolves `secrets.enc`, `verify.pub`, `token`,
    `session.toml` per platform.
  - Secret hygiene: `secrecy` wrappers, `zeroize` on drop, `mlock` via
    `memsec`.

- **`daemon::` modules** (compiled into `nordnet-daemon` binary): IPC
  server, hardening (`prctl`, `memfd_secret`).

- **`client::` modules** (compiled into `nordnet` binary):
  `DaemonClient` impls `Dispatcher` + `SecretsBackend` over the Unix
  socket.

#### `nordnet-cli-unsafe`

Bin-only crate. `main.rs`:

```rust
fn main() -> ExitCode {
    nordnet_cli::run(env::args(), DirectClient::new(), DirectSecrets::new())
}
```

`DirectClient` reads `session.toml` cleartext (refresh on near-expiry),
calls API, returns JSON. `DirectSecrets` runs auth flow locally,
writes `session.toml` cleartext, decrypts mint-private locally for
`mint`. Does not expose `nordnet daemon` subcommand (clap subtree
gated by `#[cfg(feature = "daemon")]` on the lib; the unsafe bin
disables the feature in its `Cargo.toml` dep line).

#### Adding a new endpoint

1. Add request/response types in `nordnet-model`.
2. If mutating, `impl VerifiableOp for <RequestType>` in
   `nordnet-model::token`.
3. Add resource method in `nordnet-api`.
4. Add `Operation` variant + clap subcommand in `nordnet-cli` lib.
5. Wire dispatcher arm. `match` exhaustiveness keeps the build red
   until the new variant is handled.

### Verification chokepoint

Single function in `nordnet_model::token`:

```rust
pub fn verify(
    token: &str,
    pk:    &PasetoV4PublicKey,
    op:    &dyn VerifiableOp,
) -> Result<Claims, TokenError> {
    use pasetors::claims::ClaimsValidationRules;
    let mut rules = ClaimsValidationRules::new();
    rules.validate_issuer_with("nordnet-cli/v1");
    let trusted = pasetors::version4::PublicToken::verify(pk, token, None, None)?;
    rules.validate_claims(&trusted)?;          // iat/nbf/exp/iss via pasetors clock
    let claims: Claims = serde_json::from_str(trusted.payload())?;
    op.check_caveats(&claims)?;
    Ok(claims)
}
```

`pasetors::ClaimsValidationRules::validate_claims` reads
`OffsetDateTime::now_utc()` internally; no `now` parameter is threaded
in. Returns `Claims` so the caller (dispatcher) can read `jti` for
aggregate-budget bookkeeping without re-parsing.

`Claims` is a typed struct with `#[serde(deny_unknown_fields)]` so
old verifiers reject any unknown future-version caveat rather than
silently accept it. Carries a `jti: Uuid` for per-token state keying
(aggregate notional, audit correlation).

```rust
pub trait VerifiableOp {
    const SCOPE: &'static str;                              // "orders.place"
    fn account_id(&self)    -> AccountId;
    fn instrument_id(&self) -> Option<InstrumentId>;
    fn notional(&self)      -> Option<Money>;

    fn check_caveats(&self, claims: &Claims) -> Result<(), CaveatError> {
        if !claims.scope.contains(Self::SCOPE)        { return Err(CaveatError::ScopeMismatch); }
        if claims.account_id != self.account_id()     { return Err(CaveatError::AccountMismatch); }
        if let Some(allow) = &claims.instruments {
            if let Some(id) = self.instrument_id() {
                if !allow.contains(&id)               { return Err(CaveatError::InstrumentNotAllowed); }
            }
        }
        if let (Some(cap), Some(amt)) = (&claims.max_notional_per_trade, self.notional()) {
            if amt.currency != cap.currency           { return Err(CaveatError::NotionalCurrencyMismatch); }
            if amt.amount   > cap.amount              { return Err(CaveatError::PerTradeNotionalExceeded); }
        }
        // Aggregate-notional check is NOT here — it requires daemon-side
        // persistent state (token_usage/<jti>.json). See "Aggregate budget".
        Ok(())
    }
}
```

`notional()` returns `Option<Money>` — cancel/activate carry no
amount. `mint` CLI rejects `--max-notional-per-trade` or
`--max-notional-aggregate` combined with a scope set whose every
member's `notional()` is structurally `None`. `mint` also rejects if
both notional caps are set with different currencies.

#### Aggregate budget

Per-token spend is persisted at
`<state_dir>/nordnet/token_usage/<jti>.json` (mode 0600), schema
`{ spent: Money }`. Dispatcher flow per mutating op:

1. `flock(LOCK_EX)` on `<state_dir>/nordnet/token_usage/<jti>.lock`.
2. `verify(token, pk, op)?` → `Claims`.
3. Read `<jti>.json` (default `{ spent: 0 }` if absent).
4. If `claims.max_notional_aggregate.is_some()` and
   `op.notional().is_some()`: check `spent + amt ≤ cap` (currency
   already matched at mint), else `Err(AggregateNotionalExceeded)`.
5. Increment `spent` (write-tmp-fsync-rename) **before** API call —
   conservative: rejected orders consume budget; simpler and
   fail-safe.
6. API call.
7. Release lock.

`auth revoke` deletes `token_usage/<jti>.json` alongside the token
file. Stale `<jti>.json` files for expired tokens are cleaned up
opportunistically by daemon on startup (walk dir, drop entries whose
referenced token is gone or whose mtime exceeds max possible TTL).

`nordnet-unsafe` follows the identical flow in-process. Attacker can
`rm token_usage/<jti>.json` to reset the aggregate counter — listed
under "Not defended (`nordnet-unsafe`)".

The dispatcher's match pairs each mutating `Operation` variant with
one `verify(...)?` call followed by the API call. New mutating
variant without verify+dispatch arm fails the build via match
exhaustiveness.

**Dispatcher-arm-without-verify gate**: match exhaustiveness ensures
the variant is handled, not that `verify(...)?` precedes the API
call. Caught at test time by `tests/dispatcher_verify_gate.rs`:

```rust
#[tokio::test]
async fn every_mutating_op_rejects_tampered_token() {
    let mock = MockServer::start().await;
    let dispatcher = test_dispatcher(mock.uri());
    let bad_token  = tampered_paseto();
    for op in every_mutating_operation() {
        let result = dispatcher.execute(op, Some(bad_token.clone())).await;
        assert!(result.is_err());
        assert_eq!(mock.received_requests().await.unwrap().len(), 0);
    }
}
```

`every_mutating_operation()` is a helper whose body is a `match` on
`Operation` returning one fixture per mutating variant. New mutating
variant → match exhaustiveness fails this helper → fixture must be
added → test runs the new arm. The gate stays current automatically.

#### What this defends

- L3 argv attacker: every code path from clap-parsed args to HTTP
  goes through one `verify` call against op-derived caveats.
- Developer omission (new mutating op without `verify(...)?`):
  caught at test time by `tests/dispatcher_verify_gate.rs` above.
  Match exhaustiveness in the test's `every_mutating_operation()`
  helper keeps coverage current as variants are added.
- Caveat-logic regression: per-op property tests
  (`tests/<op>_caveat.rs`) assert
  `verify(...).is_ok() <=> manually_check(op, claims)`.

#### What this does NOT defend

- Linked-library / tampered-binary attacker: in-process `unsafe` can
  fabricate any value or replace `verify` itself. Already in the
  non-defended list.
- Dispatcher logic bug that fetches the wrong account-id from the
  parsed `Operation` before calling `verify`. Mitigated by the per-op
  property tests above.

## Auth and storage

### Filesystem layout

```
$XDG_CONFIG_HOME/nordnet/         (mode 0700, parent dir)
├── secrets.enc      argon2id(password) + XChaCha20Poly1305
│                    contents: { ed25519_private, api_key, mint_private }
│                    (mode 0600)
├── verify.pub       PASETO v4 public key in PASERK k4.public format
│                    (mode 0644)
├── token            current capability token (PASETO v4, mode 0600,
│                    absent if none)
├── config.toml      existing non-sensitive config (mode 0644)
└── session.toml     ONLY written by nordnet-unsafe
                     ({ session_key, expires_in, acquired_at }, mode 0600)

$XDG_STATE_HOME/nordnet/          (mode 0700)
├── audit.log        append-only JSONL of every write attempt
│                    (mode 0600, no rotation in v1)
└── token_usage/
    ├── <jti>.json   { spent: Money } per active token (mode 0600)
    └── <jti>.lock   flock target for serialized increment
```

macOS uses `~/Library/Application Support/nordnet/` for config and
`~/Library/Logs/nordnet/audit.log` for the audit log;
`~/Library/Application Support/nordnet/token_usage/` for budget state.

Daemon socket lives at `$XDG_RUNTIME_DIR/nordnet/agent.sock` on Linux
and `~/Library/Application Support/nordnet/run/agent.sock` on macOS
(avoids `cachedeleted` eviction), mode 0600. The kernel enforces
mode 0600 at `connect(2)` — only the owning uid can connect. No
client-side peer-credential check is needed.

All file writes use write-tmp-fsync-rename on the same filesystem.

### Migration from cleartext

Documented manual procedure in README: read your existing
`credentials.toml`, run `nordnet secure init` (prompts for new
password and pastes secret values), delete `credentials.toml`,
re-run `auth login`. No `--migrate-from` flag, no transactional
dance — one-time cost, low user count.

### CLI ceremonies

| Command | Password? | Effect |
|---|---|---|
| `nordnet secure init` | Yes (set + confirm) | Generates Ed25519 mint keypair; prompts for `api_key` + Ed25519 SSH key; encrypts long-lived secrets to `secrets.enc`; writes `verify.pub`. Refuses if `secrets.enc` exists without `--force`. |
| `nordnet auth login [--password-stdin]` | Yes | Daemon decrypts secrets; runs Nordnet `/login/start` + `/login/verify`; caches session key in memory; password zeroized. |
| `nordnet auth refresh` | No | Daemon calls `PUT /login`. Renewable indefinitely (rolling 30-min idle TTL). |
| `nordnet auth logout` | No | Daemon `DELETE /login/<session_key>`, clears state. |
| `nordnet auth status` | No | Prints daemon state, session expiry, token caveats. |
| `nordnet auth mint --scope <scope>... --account <id> [--max-notional-per-trade <amount>:<currency>] [--max-notional-aggregate <amount>:<currency>] [--instrument <id>]... --ttl <duration> [--password-stdin]` | Yes | Daemon decrypts mint-private; signs PASETO v4 token (with fresh `jti`); writes `token` file (replaces existing after y/N confirm; non-TTY fails closed). Rejects mismatched currencies between the two notional caps. Rejects either notional cap if scope set is entirely notional-less (cancel/activate). |
| `nordnet auth show` | No | Prints current token caveats. |
| `nordnet auth revoke` | No | Deletes `token` file and `token_usage/<jti>.json`. |
| `nordnet daemon {start, stop, status, restart}` | No | Lifecycle. Only on `nordnet`. |

`nordnet-unsafe` exposes the same commands except `daemon`. `auth
login` writes `session.toml` cleartext instead of caching in memory.

### Session lifecycle

| Event | Result |
|---|---|
| Daemon alive, session active | Auto-refresh ~5 min before idle expiry. |
| Daemon alive, session lost | Next call returns `SessionExpired`; user runs `auth login`. |
| Daemon crash | Next client call returns `ECONNREFUSED` or `ENOENT`; prints "daemon dead — run `nordnet daemon start` then `nordnet auth login`". |
| Reboot | Daemon and session gone. User runs `auth login`. |
| `auth login` while session already active | Daemon `DELETE /login/<old>` server-side, then runs new login. |
| Two `nordnet-daemon` processes started concurrently | Startup: `flock(LOCK_EX, LOCK_NB)` on `$XDG_RUNTIME_DIR/nordnet/daemon.lock`; under the lock `unlink(agent.sock)` (ignore ENOENT), `bind`, `chmod 0600`, `listen`. Lock retained for daemon lifetime. Loser of the race exits `DaemonAlreadyRunning`. |
| `nordnet-unsafe` cold call within 5 min of expiry | `flock(LOCK_EX)` on `session.toml`; re-read under lock to check whether another invocation already refreshed; otherwise `PUT /login`, write-tmp-fsync-rename, release. |
| `auth mint` invoked twice concurrently | `flock(LOCK_EX)` on sibling `token.lock`; write-tmp-fsync-rename. Last writer wins. New `jti` per mint; old `token_usage/<jti>.json` cleaned up by `auth revoke` or daemon-startup sweep. |
| Two `Execute`s of same token concurrently | `flock(LOCK_EX)` on `token_usage/<jti>.lock` serializes verify → budget-check → increment → API call. |

**Token effective TTL** = `min(token.expires_at, session-liveness)`.
A 7-day token does not survive a reboot if the daemon is gone.
Stated in `auth show` output.

### Token model

- **Single active token.** Re-minting prompts y/N; new token replaces
  old. No merge.
- **Caveats**:
  - `scope`: set of `VerifiableOp::SCOPE` strings (`orders.place`,
    `orders.cancel`, etc.). CLI flag `--scope`, repeatable.
  - `account_id`: single `AccountId`. Op's `account_id()` must match.
  - `instrument_id`: optional allowlist. CLI flag `--instrument`,
    repeatable; omit to permit all.
  - `max_notional_per_trade`: optional currency-tagged `Money`. CLI
    flag `--max-notional-per-trade <amount>:<currency>` (e.g.
    `10000:NOK`); strict parser. Trade in non-matching currency
    rejected as caveat violation. Bounds the size of any single
    order.
  - `max_notional_aggregate`: optional currency-tagged `Money`. CLI
    flag `--max-notional-aggregate <amount>:<currency>`. Bounds the
    cumulative notional across all uses of this token. Enforced via
    daemon-side `token_usage/<jti>.json`. **Spend is incremented on
    submit, not on fill** — rejected orders consume budget. Conservative
    by design.
  - `expires_at`: RFC 3339 timestamp.
  - `jti`: UUID, generated at mint time. Keys per-token state files.
- **Read operations are never gated by tokens** in either binary.
  Reads use only the session key. Read-side exfil named in
  "Not defended".
- **Write operations always require a token** in both binaries.
- **Verification** runs once per call via
  `nordnet_model::token::verify(token, pk, op)` pre-HTTP. Same
  function in daemon (for `nordnet`) and in-process (for
  `nordnet-unsafe`). Aggregate-budget check + increment wrap the
  call inside the dispatcher under `flock(token_usage/<jti>.lock)`.
- **Revocation**: file deletion. TTL is the only intrinsic
  revocation; Nordnet API does not know about our tokens.

## IPC protocol

Length-prefixed framing over Unix socket. **u32 length, then JSON
body bytes.** First field of every Request and Response is `version:
u32` (current = 1). Mismatch closes the connection with a
`ProtocolVersionMismatch` error response.

```rust
pub enum Request {
    Ping,
    Status,
    AuthLogin   { password: SecretString },
    AuthLogout,
    AuthRefresh,
    AuthMint    { password: SecretString, caveats: Caveats },
    Execute     { op: Operation, token: Option<String> },
}

pub enum Response {
    Pong,
    Status(DaemonStatus),
    Ok,
    SessionInfo(SessionInfo),
    Token(String),
    ExecuteResult(serde_json::Value),
    Error(DaemonError),
}

/// Returned to the client after a successful login or refresh. Carries
/// no `session_key` — the daemon's contract is that the session key
/// never leaves daemon memory.
pub struct SessionInfo {
    pub expires_at:   DateTime<Utc>,
    pub acquired_at:  DateTime<Utc>,
    pub account_ids:  Vec<AccountId>,
    pub login_method: LoginMethod,
}
```

`SecretString` (from `secrecy`) zeroizes on drop. The serde codec
deserializes directly into the wrapper; the password never sits in a
plain `String` on the daemon side.

A unit test (`tests/session_info.rs`) asserts `SessionInfo`
serializes to a fixed key allowlist (no `session_key` ever appears).

## Error model

CLI exits with stable numeric code per category:

| Exit | Category | Examples |
|---|---|---|
| 0  | Success | |
| 1  | Generic / unexpected | uncategorized errors |
| 10 | Bad invocation | unknown subcommand, missing arg, `--max-notional-*` on notional-less scope, mismatched currency between per-trade and aggregate caps |
| 20 | Daemon unreachable | socket missing, ECONNREFUSED |
| 21 | Protocol mismatch | client/daemon version skew |
| 30 | Session expired | Nordnet 401, idle timeout |
| 31 | Session not established | no `auth login` yet |
| 40 | No active token | mutating op without token file |
| 41 | Token expired | |
| 42 | Token caveat violation | scope/account/per-trade-notional/instrument mismatch |
| 43 | Token signature invalid | tampered token file |
| 44 | Aggregate notional exceeded | cumulative spend on this token would exceed `max_notional_aggregate` |
| 50 | Password required, no TTY | `auth login` in non-interactive shell without `--password-stdin` |
| 60 | Nordnet API error | HTTP 4xx/5xx |
| 70 | Crypto failure | bad password / corrupt secrets.enc |

Errors emit JSON on stderr: `{ "exit": N, "code": "TOKEN_CAVEAT_VIOLATION",
"message": "...", "details": {...} }`.

## Audit log

Every write attempt is recorded as one JSONL entry, written
post-`verify` and post-API-call so outcome is captured. Same path,
same schema for `nordnet` (daemon-written) and `nordnet-unsafe`
(in-process-written).

```
$XDG_STATE_HOME/nordnet/audit.log              (Linux, mode 0600)
~/Library/Logs/nordnet/audit.log               (macOS, mode 0600)
```

Per-line schema:

```json
{ "ts": "2026-05-04T12:34:56.789Z",
  "binary": "nordnet" | "nordnet-unsafe",
  "op": "orders.place",
  "account_id": "...",
  "instrument_id": "..." | null,
  "notional": { "amount": "1000", "currency": "NOK" } | null,
  "token_jti": "uuid" | null,
  "verify_result": "ok" | "scope_mismatch" | "account_mismatch"
                 | "per_trade_notional_exceeded" | "aggregate_notional_exceeded"
                 | "instrument_not_allowed" | "expired" | "signature_invalid"
                 | "no_token",
  "api_status": 200 | null,
  "api_error": null | "..." }
```

- **All verify attempts logged** — successes and failures. Failures
  are the high-value forensic signal.
- **Writes only** in v1; reads not gated and not logged.
- **No rotation in v1**; documented as user responsibility. JSONL
  grows slowly for a CLI workload.
- **Append-only**: `O_APPEND` open; concurrent processes (parallel
  unsafe invocations) interleave whole lines safely on POSIX up to
  `PIPE_BUF`. Lines stay well under that.
- **Best-effort write**: audit-log write failure does not block the
  operation; error logged to stderr. Trading off forensic completeness
  for availability — operation already authorized by token.

A test (`tests/audit_log_schema.rs`) asserts every variant of
`verify_result` round-trips through the schema.

## Testing

| Concern | Test |
|---|---|
| Endpoint roundtrip | Existing wiremock per `CONTRIBUTING.md`. |
| IPC server | `daemon::server` exercised via `(UnixStream, UnixStream)` pair in unit tests. |
| Token verify | Property tests on `Caveat`: tamper signature → reject; expire → reject; mismatched scope/account/per-trade-notional/instrument → reject; valid → accept. |
| Per-op caveat | `tests/<op>_caveat.rs` property-fuzzes op + claims and asserts `verify(...).is_ok() <=> manually_check(op, claims)`. |
| `VerifiableOp` contract | `tests/verifiable_op_contract.rs` enumerates every mutating `Operation` variant via match-exhaustiveness; asserts `notional()` returns `Some` for order-placing/modifying, `None` for cancel/activate. |
| Dispatcher verify gate | `tests/dispatcher_verify_gate.rs` enumerates every mutating `Operation` variant via match-exhaustiveness; runs dispatcher with a tampered token; wiremock asserts zero HTTP calls reach the wire. Catches a future arm that omits `verify(...)?`. |
| Aggregate budget | `tests/aggregate_budget.rs`: mint token with `max_notional_aggregate=100:NOK`; submit two ops at 60 NOK each; second rejected with `AggregateNotionalExceeded`; wiremock confirms only one HTTP call. Concurrent variant uses two tasks racing on the same `<jti>.lock`. |
| Aggregate budget rejected-order semantics | `tests/aggregate_rejected.rs`: submit op at 60 NOK; wiremock returns 400; assert `spent` incremented to 60 (rejected orders consume budget). |
| Audit log schema | `tests/audit_log_schema.rs`: every `verify_result` variant + every API-status outcome serializes to expected JSONL shape. |
| Crypto roundtrip | `secure::crypto` encrypt → decrypt with fixed password. Argon2id params golden-tested to OWASP minimums. |
| `SessionInfo` serialization | Fixed key allowlist; `session_key` never appears. |

No live-API tests.

## Crypto and dependency stack (pinned in `Cargo.toml`)

| Concern | Crate | Version |
|---|---|---|
| TTY password prompt | `rpassword` | 7.5 |
| KDF | `argon2` | 0.5 |
| AEAD at rest | `chacha20poly1305` | 0.10 (XChaCha20Poly1305) |
| Token | `pasetors` | 0.7 |
| Ed25519 | `ed25519-dalek` | 2.2 (already present) |
| Secret wrappers | `secrecy` | 0.10 |
| Zeroize | `zeroize` | 1.8 |
| `mlock` | `memsec` | 0.7 |
| Async runtime | `tokio` | 1.x (already present) |
| File locking | `nix::fcntl::flock` | (already present for `prctl`/`memfd_secret`) |

`bincode` not added; IPC uses `serde_json`. `uuid` (`v4` feature)
added for `jti` minting.

No transitive `curve25519-dalek` conflict between `pasetors 0.7` and
`ed25519-dalek 2.2`: `pasetors` uses `ed25519-compact` (self-contained,
no dalek family), so there is no shared `curve25519-dalek` subgraph to
disagree about.

## Build / lint

- `nordnet-model`, `nordnet-api`, `nordnet-feed`,
  `nordnet-cli-unsafe`: `unsafe_code = "forbid"`.
- `nordnet-cli`: `unsafe_code = "deny"` (so `secure::mem` and
  `daemon::hardening` opt in via `#[allow(unsafe_code)]`).
- `nordnet-cli-unsafe` `main.rs` carries a top-of-file doc comment
  enumerating its transitive `unsafe` surface
  (`nordnet_cli::secure::mem::mlock`, `nordnet_cli::secure::crypto::*`).

### macOS hardened runtime

`nordnet-daemon` checks the `CS_RUNTIME` flag on its own binary at
startup via `SecCodeCopySigningInformation`. If unset, **refuses to
start** with:

```
Error: nordnet-daemon binary lacks hardened runtime — L4 defense
unavailable. Fix:
  codesign --force --options runtime --sign - $(which nordnet-daemon)
```

No `--allow-degraded` flag. Hardened runtime is required for the L4
guarantee on macOS.

### Linux

`nordnet-daemon` sets `PR_SET_DUMPABLE=0` at startup. This is the
primary L4 defense — `/proc/<pid>/mem` becomes root-owned and
`PTRACE_ATTACH` requires `CAP_SYS_PTRACE`. `kernel.yama.ptrace_scope`
is defense-in-depth on top; the daemon does not check or warn about
it.

`memfd_secret(2)` runtime-detected at daemon start; `ENOSYS` falls
back to `mlock` + `MADV_DONTDUMP`. Logs which path was taken to
stderr.

## Distribution

```bash
cargo install nordnet-cli
cargo install nordnet-cli-unsafe   # opt-in, separate crate
```

On macOS, `cargo install` produces unsigned binaries. The daemon
refuses to start (no `--allow-degraded` escape — hardened runtime
is required for the L4 guarantee). The error message contains the
exact `codesign` command. README documents the recovery as the next
step after `cargo install` on macOS. `cargo install --force`
strips codesign; user must re-run after every upgrade.

**Documented friction**: macOS users on the `cargo install` path pay
a manual `codesign` step at every install and every upgrade.
Acceptable trade — alternatives are (1) a custom install script
(rejected as out of scope) or (2) the `--allow-degraded` escape
(rejected because it silently downgrades L4). Users who don't want
the friction can switch to `nordnet-cli-unsafe`, which deliberately
does not protect L4 anyway.

A `nordnet-cli-unsafe` install requires the user to type the word
"unsafe". That is the entire signal-of-intent.

## What this defends and what it does not

> **Defended (`nordnet`):**
> - Buggy / curious / in-process malicious agent (L1–L3) blocked by
>   token gate on every write. Caveats (scope, account, per-trade
>   notional, aggregate notional, instrument, expiry) bound damage
>   when the token itself is replayed by an in-process agent.
> - Active extraction of session key (L4) blocked by daemon: session
>   key never on disk, never returned over IPC. On Linux 5.14+,
>   `memfd_secret`; otherwise `mlock` + `MADV_DONTDUMP`.
>   `PR_SET_DUMPABLE=0` is the primary defense — same-user attacker
>   blocked even on `kernel.yama.ptrace_scope=0`. macOS: hardened
>   runtime blocks `task_for_pid`.
> - Long-lived secrets at rest, encrypted with argon2id-derived KEK.
> - Token expiry enforced inside the daemon process. Daemon does not
>   inherit the CLI client's environment, so an agent that exec's
>   `LD_PRELOAD=libfaketime nordnet ...` cannot extend an expired
>   token — daemon re-runs `verify` against its own clock.
> - Aggregate notional cap enforced via daemon-held
>   `token_usage/<jti>.json`; same-uid agent cannot rewrite this file
>   while daemon holds the per-jti `flock(LOCK_EX)`.
> - Append-only audit log of every write attempt
>   (success + every failure variant) for forensic review.

> **Defended (`nordnet-unsafe`):**
> - L1–L3 only. Same token gate, same encrypted long-lived secrets,
>   same audit log.
> - L4 explicitly not defended. Session key cleartext in
>   `session.toml`.
> - Token expiry: in-process verify trusts the process clock. Agent
>   that exec's `LD_PRELOAD=libfaketime nordnet-unsafe ...` extends
>   expired tokens. By design — `nordnet-unsafe` is the explicitly
>   weaker binary.
> - Aggregate notional state (`token_usage/<jti>.json`) is on disk
>   under the user's UID; an agent with the same UID can `rm` it to
>   reset the counter. Bounded by per-trade cap; aggregate is
>   best-effort under this binary.

> **Not defended (either binary):**
> - Root on the host.
> - Tampered `nordnet` / `nordnet-daemon` binary capturing the
>   password.
> - `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES` / shell alias hijack of
>   the CLI binary at exec time (mitigated for `nordnet` by daemon
>   re-verification of expiry; not mitigated for `nordnet-unsafe`).
> - Social-engineering ("paste your password here").
> - Hibernation file leak without full-disk encryption.
> - Read-side data exfiltration: an agent can list orders, positions,
>   account balances. Trade made deliberately for autonomy.
> - macOS without hardened runtime (cargo-install path) — daemon
>   refuses to start; not a defended state.
> - Hardware-side compromise.
> - Audit-log tampering: log file is mode 0600 under the user's UID;
>   same-uid agent can truncate or rewrite. No tamper-evidence
>   (hash-chain, remote shipping) in v1.

## Required README posture

> nordnet-cli runs an unprivileged user-session daemon that holds
> your Nordnet session in memory and enforces user-minted, scoped,
> time-bound capability tokens on every write. Tokens carry per-trade
> and aggregate notional caps; aggregate spend is tracked daemon-side
> per token. Every write attempt — success or failure — is appended
> to `audit.log` for forensic review. The daemon sets
> `PR_SET_DUMPABLE=0` on Linux and requires hardened-runtime
> codesigning on macOS, so a misbehaving or curious AI agent
> running as your user cannot ptrace it or read its memory.
>
> **macOS install requires a one-time-per-upgrade codesign step.**
> `cargo install nordnet-cli` produces an unsigned binary; the
> daemon refuses to start until you run the `codesign --force
> --options runtime --sign - $(which nordnet-daemon)` command shown
> in the daemon's startup error. There is no `--allow-degraded`
> escape — hardened runtime is required for the L4 guarantee.
>
> nordnet-cli does **not** defend against root on the host, a
> tampered binary, or social-engineering. Read endpoints are not
> gated; an agent can enumerate your account state.
>
> A separately-named binary, `nordnet-unsafe` (from the
> `nordnet-cli-unsafe` package), gives you the same functionality
> without the daemon — full power, no isolation. Use it only if you
> understand the trade.

## Decided

- Two crates: `nordnet-cli` (lib + 2 bins) and `nordnet-cli-unsafe`
  (bin only).
- Two binaries: `nordnet` (daemon-mediated) and `nordnet-unsafe`
  (cleartext session). `nordnet-cli` also produces `nordnet-daemon`.
- Daemon-mediated session key for `nordnet`; cleartext on disk for
  `nordnet-unsafe`.
- Token = PASETO v4 public token, single file, replace-only on mint.
  Carries `jti: Uuid` for per-token state file keying.
- Caveats: scope, account, `max_notional_per_trade`,
  `max_notional_aggregate`, instrument, expires_at.
- Aggregate notional enforced via daemon-held
  `<state_dir>/nordnet/token_usage/<jti>.json`. Spend incremented on
  submit (rejected orders consume budget — conservative, fail-safe).
  Per-jti `flock(LOCK_EX)` serializes verify → check → increment →
  API call. `nordnet-unsafe` runs the same flow in-process; same-uid
  rm of the state file resets counter (documented weakness).
- Read ops never require a token; write ops always require one.
- Long-lived secrets encrypted at rest with argon2id (OWASP minimum)
  + XChaCha20-Poly1305.
- Linux + macOS in v1.
- IPC = serde_json over Unix socket, length-prefixed frames. Version
  carried as `version: u32` field on every frame; no separate
  handshake.
- Password carried as `SecretString` field on `AuthLogin` / `AuthMint`
  requests; zeroized on drop on both sides. No side-channel
  handshake.
- Single verify chokepoint (`nordnet_model::token::verify`) called
  per dispatcher arm. Caveat semantics on `VerifiableOp` trait,
  colocated with op type. Dispatcher-arm-without-verify gap caught
  at test time by `tests/dispatcher_verify_gate.rs` (enumerates
  every mutating variant via match exhaustiveness, runs dispatcher
  with tampered token, asserts wiremock receives zero requests).
- macOS daemon refuses to start without hardened runtime.
- Linux daemon sets `PR_SET_DUMPABLE=0`; does not check
  `ptrace_scope`.
- Audit log in v1. Append-only JSONL at
  `$XDG_STATE_HOME/nordnet/audit.log` (Linux) /
  `~/Library/Logs/nordnet/audit.log` (macOS). All write attempts —
  every verify outcome and API result — recorded. Best-effort: write
  failure does not block the operation. No rotation, no
  tamper-evidence in v1 (same-uid agent can truncate; documented).
- Daemon socket: `unlink → bind → chmod 0600` under
  `flock(daemon.lock)`. Mode 0600 in user-only dir is the access
  gate; no client-side peer-credential check.
- No IPC frame size cap. Same-uid attacker isn't bounded by
  protocol limits.
- No migration tooling. Manual procedure documented in README.
- `--password-stdin` flag for `auth login` and `auth mint` in
  non-TTY contexts.
- Defense against accidental daemon code in `nordnet-unsafe`: Cargo
  feature gating (`daemon`-feature gates `daemon::*` and `client::*`
  modules + the `nordnet daemon` clap subtree; `nordnet-cli-unsafe`
  depends on `nordnet-cli` with `default-features = false`) plus
  code review on the small (~400 line) bin source. No symbol-table
  xtask.
- `unsafe_code = "forbid"` on every crate except `nordnet-cli`,
  which is `deny` so `secure::mem` and `daemon::hardening` can opt
  in.

## Open forks (resolve in plan)

1. **systemd user unit / launchctl plist** shipped as optional
   templates installable via `nordnet daemon install-unit`, or
   documented manually.
2. **CONTRIBUTING.md update** — extend `xtask gen-mods` and
   per-resource-group convention sections to cover new `secure/`,
   `daemon/`, `client/` module roots.

## Out of scope for v1

- Windows.
- Hardware-key escalation (YubiKey, Secure Enclave, Touch ID).
- Multi-account simultaneous tokens.
- Token attenuation / delegation.
- Cross-host session sharing.
- Read-op gating.
