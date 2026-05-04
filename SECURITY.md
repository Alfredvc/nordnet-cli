# Security

`nordnet-cli` wraps a brokerage REST API that includes write endpoints
which **move real money** (place / modify / activate / cancel orders).
This document describes the current security posture so users can decide
whether — and how — to expose the binary to an AI agent or any other
process they do not fully trust.

> **Short version**: the binary today provides no agent containment.
> An AI agent (or any process) running as your user can read every
> credential, mint a session, and call every write endpoint without
> further authorization. Treat the binary as equivalent to giving the
> caller direct access to your Nordnet account. Run only inside an
> isolation boundary you trust (separate UNIX user, container, VM, or
> hardware-isolated agent host).

## What is stored, where, and how

| Path | Contents | Permissions | Lifetime |
|---|---|---|---|
| `<config_dir>/nordnet/credentials.toml` | `api_key`, `key_path` (path to OpenSSH Ed25519 private key), `default_account`, `base_url` | **Not enforced by the CLI** — user-managed (`umask`, manual `chmod`) | Long-lived |
| Path referenced by `key_path` | OpenSSH Ed25519 private key registered with Nordnet | User-managed | Long-lived |
| `<config_dir>/nordnet/session.toml` | `session_key`, `expires_in`, `acquired_at` | `0600` enforced on Unix (atomic temp-file + rename) | Until logout, idle timeout, or `nordnet auth refresh` lapses |
| Environment variables (`NORDNET_API_KEY`, `NORDNET_KEY_PATH`, `NORDNET_SESSION_KEY`) | Same secrets, in-process | Inherited by every child process | Process lifetime |

`<config_dir>` resolves per-platform: `$XDG_CONFIG_HOME/nordnet/` on
Linux (default `~/.config/nordnet/`), `~/Library/Application
Support/nordnet/` on macOS.

**Nothing is encrypted at rest.** `session.toml` is mode `0600` so other
local users cannot read it; `credentials.toml` permissions are the
user's responsibility.

## Threat model

| # | Attacker | Defended? | Notes |
|---|---|---|---|
| T1 | Other local user (different UID) reads `session.toml` | ✓ | Mode `0600` enforced atomically. |
| T2 | Other local user reads `credentials.toml` / SSH key | ✗ | CLI does not chmod these. Set `chmod 600` yourself. |
| T3 | Same-UID process (any agent, shell, daemon, editor plugin) reads any of the above | ✗ | Filesystem ACL gates only inter-user access. |
| T4 | Same-UID process invokes `nordnet orders place ...` with crafted argv | ✗ | No per-call authorization. Session = full write surface. |
| T5 | Same-UID process exfiltrates account state via read endpoints | ✗ | Reads are not gated. |
| T6 | AI agent running as your user is buggy and calls a write endpoint by mistake | ✗ | Same as T4 — CLI cannot distinguish intended from accidental invocation. |
| T7 | AI agent is adversarial in-process (reads disk, calls API directly with the session key it just read) | ✗ | The session key on disk is sufficient to bypass any in-CLI gate. |
| T8 | Tampered `nordnet` binary captures password / session | ✗ | Out of scope. |
| T9 | Root on the host | ✗ | Out of scope. |

T3–T7 collapse to one fact: **same-UID = full account access**. An AI
agent invoked from the same shell that runs `nordnet auth login` shares
that UID and inherits that access.

## Nordnet API constraints

The Nordnet API offers no scopes, no read-only key, and no sandbox
([api.test.nordnet.se docs](https://api.test.nordnet.se/api-docs/index.html)).
There is no server-side enforcement we can opt into; any client-side
restriction would have to be added to this CLI itself, and is not
present today.

## Recommended mitigations

These are operator responsibilities; the CLI does not perform any of
them automatically.

1. **Do not run untrusted agents under the same UNIX user** that holds
   your Nordnet credentials. Isolate by:
   - a dedicated UNIX user for the agent (`useradd nordnet-agent`,
     credentials owned by your real user, agent has no read access);
   - a container or VM with credentials mounted only when needed;
   - a hardware-isolated agent host.
2. **Set tight permissions manually**:
   ```bash
   chmod 700 ~/.config/nordnet
   chmod 600 ~/.config/nordnet/credentials.toml
   chmod 600 "$NORDNET_KEY_PATH"
   ```
3. **Logout when done**: `nordnet auth logout` invalidates the
   server-side session and removes `session.toml`. Do not leave a live
   session next to an agent process.
4. **Prefer ephemeral sessions over long-lived `credentials.toml`** for
   one-off tasks: pass `NORDNET_SESSION_KEY` for a single invocation and
   skip persistence.
5. **Never log or pipe `nordnet config` output** in transcripts shared
   with third parties — secrets are redacted from the JSON, but the
   surrounding config (account IDs, key paths) is enumeration-grade
   information.
6. **Watch for write commands in agent tool calls**. The mutating
   surface is small and named consistently:
   - `nordnet orders place`
   - `nordnet orders modify`
   - `nordnet orders activate`
   - `nordnet orders cancel`
   - `nordnet auth login` / `auth refresh` / `auth logout`

   Allowlist tools at the agent layer if your harness supports it.

## What is *not* in this CLI today

The following defenses were considered (see
[`docs/specs/2026-05-03-security-layer-design.md`](docs/specs/2026-05-03-security-layer-design.md))
and **deliberately not built** in v1:

- User-controlled, per-task **capability tokens** with caveats (scope,
  account, per-trade and aggregate notional caps, instrument allowlist,
  expiry). Today every write endpoint is reachable as long as a session
  exists.
- A **user-session daemon** that keeps the session key in process
  memory instead of on disk.
- Encryption-at-rest for long-lived secrets (Argon2id-derived KEK +
  XChaCha20-Poly1305).
- An **append-only audit log** of write attempts.
- Hardening against same-UID memory inspection (`PR_SET_DUMPABLE=0`,
  `memfd_secret`, macOS hardened-runtime codesigning).

The spec is preserved as a record of what a future hardened mode would
look like, but **none of it is implemented**. Do not assume any of
those properties when threat-modelling your own deployment.

## Reporting

If you find a vulnerability in this CLI itself (memory unsafety,
incorrect signature verification, session-key leak via stdout/logs,
etc.), please open a GitHub security advisory at
<https://github.com/Alfredvc/nordnet-cli/security/advisories/new>
rather than a public issue.
