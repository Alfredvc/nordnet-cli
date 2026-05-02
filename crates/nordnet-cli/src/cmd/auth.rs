//! `nordnet auth` — high-level authentication subcommands.
//!
//! Replaces the lower-level 1:1-with-API `nordnet login start/verify`
//! shape. The unified `auth login` reads `api_key` and `key_path` from
//! the resolved config, runs the full start → sign → verify flow, and
//! persists the resulting session to disk
//! (`<config_dir>/nordnet/session.toml`, mode `0600` on Unix). All other
//! `nordnet` commands then transparently load that session — no need to
//! pass `--session-key` on every call.
//!
//! # Subcommands
//!
//! | Cmd | API call(s)                       | Disk effect          |
//! |-----|-----------------------------------|----------------------|
//! | `login`   | `POST /login/start` + `POST /login/verify` | writes session file  |
//! | `logout`  | `DELETE /login`                            | deletes session file |
//! | `refresh` | `PUT /login`                               | updates session file |
//! | `status`  | none                                       | reads session file   |
//!
//! # Override order for the active session
//!
//! See [`crate::session`] module docs. `--session-key` (CLI) overrides
//! `NORDNET_SESSION_KEY` (env), which overrides the disk file.

use anyhow::Context;
use clap::Subcommand;
use nordnet_api::auth::{parse_private_key_openssh, sign_challenge, Session};
use nordnet_api::models::login::{ApiKeyStartLoginRequest, ApiKeyVerifyLoginRequest};
use serde_json::json;
use time::OffsetDateTime;

use crate::config::Config;
use crate::session::{self, StoredSession};

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Run the full login flow and persist the session to disk.
    ///
    /// Reads `api_key` and `key_path` from the resolved config (env vars
    /// first, then `~/.config/nordnet/credentials.toml`). The signed
    /// challenge round-trip is performed automatically; on success the
    /// session is written to `~/.config/nordnet/session.toml` with mode
    /// `0600` (Unix) so subsequent `nordnet <group> <op>` calls are
    /// authenticated transparently.
    Login,
    /// Invalidate the current session (DELETE /login) and remove the
    /// local session file.
    Logout,
    /// Touch the current session (PUT /login) and refresh the local
    /// `acquired_at` timestamp so `auth status` reflects the renewed
    /// lifetime.
    ///
    /// Caveat: `PUT /login` returns `LoggedInStatus { logged_in: bool }`
    /// only — the API does not send back a fresh `expires_in`. This
    /// command therefore assumes the server resets the session timer to
    /// the same `expires_in` that `POST /login/verify` originally
    /// reported. If Nordnet ever changes the per-refresh lifetime, the
    /// `seconds_remaining` shown by `auth status` will drift; actual
    /// authenticated calls remain authoritative — they'll surface
    /// `Error::Unauthorized` (HTTP 401) the moment the server-side
    /// session lapses.
    Refresh,
    /// Print local session metadata (path, expiry, time remaining)
    /// without contacting the API.
    Status,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Login => run_login(client, fields).await,
            Cmd::Logout => run_logout(client, fields).await,
            Cmd::Refresh => run_refresh(client, fields).await,
            Cmd::Status => run_status(fields),
        }
    }
}

async fn run_login(client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let api_key = cfg.require_api_key()?;
    let key_path = cfg.require_key_path()?;

    let key_text = std::fs::read_to_string(key_path)
        .with_context(|| format!("could not read SSH key at {}", key_path.display()))?;
    let parsed = parse_private_key_openssh(&key_text)?;

    let challenge = client
        .start_login(&ApiKeyStartLoginRequest {
            api_key: api_key.to_owned(),
        })
        .await?;
    let signature = sign_challenge(&parsed, &challenge.challenge)?;
    let verify = client
        .verify_login(&ApiKeyVerifyLoginRequest {
            api_key: api_key.to_owned(),
            service: cfg.service.clone(),
            signature,
        })
        .await?;

    let stored = StoredSession {
        session_key: verify.session_key.clone(),
        expires_in: verify.expires_in,
        acquired_at: OffsetDateTime::now_utc(),
    };
    let path = session::save(&stored)?;

    let view = json!({
        "status": "logged_in",
        "session_path": path,
        "expires_in": stored.expires_in,
        "expires_at": stored.expires_at(),
    });
    crate::output::emit(&view, fields)?;
    Ok(())
}

async fn run_logout(client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
    let stored = session::load()?.ok_or(session::SessionError::Missing)?;
    let auth_client = client.clone().with_session(Session {
        session_key: stored.session_key,
        expires_in: stored.expires_in,
    });
    let api_response = auth_client.logout().await?;
    let removed = session::delete()?;
    let view = json!({
        "status": "logged_out",
        "session_file_removed": removed,
        "api_response": api_response,
    });
    crate::output::emit(&view, fields)?;
    Ok(())
}

async fn run_refresh(client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
    let mut stored = session::load()?.ok_or(session::SessionError::Missing)?;
    let auth_client = client.clone().with_session(Session {
        session_key: stored.session_key.clone(),
        expires_in: stored.expires_in,
    });
    let api_response = auth_client.refresh_session().await?;
    stored.acquired_at = OffsetDateTime::now_utc();
    let path = session::save(&stored)?;
    let view = json!({
        "status": "refreshed",
        "session_path": path,
        "expires_in": stored.expires_in,
        "expires_at": stored.expires_at(),
        "api_response": api_response,
    });
    crate::output::emit(&view, fields)?;
    Ok(())
}

fn run_status(fields: &[String]) -> anyhow::Result<()> {
    let path = session::default_session_path();
    let stored = session::load()?;
    let view = match stored {
        None => json!({
            "status": "not_logged_in",
            "session_path": path,
        }),
        Some(s) => {
            let expires_at = s.expires_at();
            let remaining = expires_at - OffsetDateTime::now_utc();
            json!({
                "status": "logged_in",
                "session_path": path,
                "acquired_at": s.acquired_at,
                "expires_in": s.expires_in,
                "expires_at": expires_at,
                "seconds_remaining": remaining.whole_seconds(),
            })
        }
    };
    crate::output::emit(&view, fields)?;
    Ok(())
}
