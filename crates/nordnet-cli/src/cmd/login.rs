//! `nordnet login` — authentication subcommands.
//!
//! `start` -> `verify` flow uses the api_key + private-key PEM path
//! from the resolved config (env vars first, then ~/.config/nordnet/credentials.toml).
//! `refresh` and `logout` take a `--session-key` because the CLI does
//! not yet persist sessions to disk.

use anyhow::Context;
use clap::{Args, Subcommand};
use nordnet_api::auth::{parse_private_key_pem, sign_challenge, Session};
use nordnet_api::models::login::{ApiKeyStartLoginRequest, ApiKeyVerifyLoginRequest};

use crate::config::Config;

/// Subcommands for `nordnet login`.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Start the login challenge (`POST /login/start`).
    ///
    /// Reads `api_key` from config (env var `NORDNET_API_KEY` or
    /// `~/.config/nordnet/credentials.toml`). Prints the `challenge` string
    /// that must be passed to `nordnet login verify --challenge <VALUE>`.
    /// The challenge is only valid for 30 seconds.
    Start,

    /// Sign a challenge with the configured private key and verify (`POST /login/verify`).
    ///
    /// Reads `api_key` and `key_path` from config. Signs the challenge
    /// with RSA PKCS#1 v1.5 + SHA-256 and completes the login flow.
    /// The returned `session_key` is needed for `refresh` and `logout`.
    Verify(VerifyArgs),

    /// Touch the session to keep it alive (`PUT /login`).
    ///
    /// Pass the `session_key` returned by `nordnet login verify`.
    /// Any authenticated API call also resets the session timer, so
    /// this is only needed when the application would otherwise be idle
    /// for the full session timeout interval.
    Refresh(SessionArgs),

    /// Invalidate the current session (`DELETE /login`).
    ///
    /// Pass the `session_key` returned by `nordnet login verify`.
    /// After logout the session key is no longer valid.
    Logout(SessionArgs),
}

/// Arguments for `nordnet login verify`.
#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// Challenge string returned by `nordnet login start`.
    /// Must be used within 30 seconds of being issued.
    #[arg(long)]
    pub challenge: String,
}

/// Arguments for `nordnet login refresh` and `nordnet login logout`.
#[derive(Debug, Args)]
pub struct SessionArgs {
    /// Session key returned by `nordnet login verify`.
    #[arg(long)]
    pub session_key: String,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Start => {
                let cfg = Config::load()?;
                let api_key = cfg.require_api_key()?;
                let req = ApiKeyStartLoginRequest {
                    api_key: api_key.to_owned(),
                };
                let r = client.start_login(&req).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Verify(a) => {
                let cfg = Config::load()?;
                let api_key = cfg.require_api_key()?;
                let key_path = cfg.require_key_path()?;
                let pem = std::fs::read_to_string(key_path)
                    .with_context(|| format!("could not read PEM at {}", key_path.display()))?;
                let parsed = parse_private_key_pem(&pem)?;
                let signature = sign_challenge(&parsed, &a.challenge)?;
                let req = ApiKeyVerifyLoginRequest {
                    api_key: api_key.to_owned(),
                    service: cfg.service.clone(),
                    signature,
                };
                let r = client.verify_login(&req).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Refresh(a) => {
                let session = Session {
                    session_key: a.session_key,
                    expires_in: 0,
                };
                let auth_client = client.clone().with_session(session);
                let r = auth_client.refresh_session().await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Logout(a) => {
                let session = Session {
                    session_key: a.session_key,
                    expires_in: 0,
                };
                let auth_client = client.clone().with_session(session);
                let r = auth_client.logout().await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
