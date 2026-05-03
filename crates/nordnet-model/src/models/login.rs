//! Models for the `login` resource group.
//!
//! Derived from the Nordnet `ApiKeyLoginResponse`, `Feed`, and
//! `LoggedInStatus` schemas.
//!
//!
//! ## Related types in [`crate::auth`]
//!
//! The auth-flow request and intermediate response types
//! ([`crate::auth::ApiKeyStartLoginRequest`],
//! [`crate::auth::ApiKeyVerifyLoginRequest`],
//! [`crate::auth::ChallengeResponse`]) live alongside the signing
//! primitives in [`crate::auth`] — they are the inputs the signing
//! helpers consume and produce, so the spec keeps them next to
//! [`crate::auth::sign_challenge`] / [`crate::auth::parse_private_key_openssh`].
//!
//!
//! ## Canonical [`ApiKeyLoginResponse`]
//!
//! This is the single canonical [`ApiKeyLoginResponse`] for the workspace.
//! Older revisions of the codebase shipped a loose duplicate in
//! [`crate::auth`] with `private_feed` / `public_feed` typed as
//! `Option<serde_json::Value>`. That duplicate has been deleted in favor
//! of this fully-typed version. Build a [`crate::auth::Session`] via
//! [`ApiKeyLoginResponse::to_session`] when attaching to an HTTP client.

use crate::auth::Session;
use serde::{Deserialize, Serialize};

/// Connection information for one of the streaming feeds.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Feed {
    /// `true` if the feed is encrypted (TLS).
    pub encrypted: bool,
    /// The feed hostname.
    pub hostname: String,
    /// The feed port. The schema models this as `integer(int64)`, so we
    /// keep it as `i64` rather than narrowing to a port-sized integer.
    pub port: i64,
}

/// Response body from `POST /login/verify`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ApiKeyLoginResponse {
    /// The session expiration interval in seconds. This is the entire
    /// interval — not the remaining time until session time-out.
    pub expires_in: i64,
    /// Connection information for the Private Feed.
    pub private_feed: Feed,
    /// Connection information for the Public Feed.
    pub public_feed: Feed,
    /// The session key used for identification in all other requests.
    pub session_key: String,
}

impl ApiKeyLoginResponse {
    /// Build an authenticated [`Session`] from this login response.
    ///
    /// Bridges this typed response to the [`Session`] type so callers can
    /// attach it via the HTTP client's session-injection method.
    pub fn to_session(&self) -> Session {
        Session {
            session_key: self.session_key.clone(),
            expires_in: self.expires_in,
        }
    }
}

impl From<&ApiKeyLoginResponse> for Session {
    fn from(response: &ApiKeyLoginResponse) -> Self {
        response.to_session()
    }
}

/// Response body from `PUT /login` (refresh) and `DELETE /login` (logout).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LoggedInStatus {
    /// `true` if the session is valid.
    pub logged_in: bool,
}
