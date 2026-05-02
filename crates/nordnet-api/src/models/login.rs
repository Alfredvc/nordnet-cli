//! Models for the `login` resource group.
//!
//! Schemas are derived from `docs-extract/_definitions/`:
//! [`ApiKeyStartLoginRequest`](../../../docs-extract/_definitions/ApiKeyStartLoginRequest.md),
//! [`ApiKeyVerifyLoginRequest`](../../../docs-extract/_definitions/ApiKeyVerifyLoginRequest.md),
//! [`ChallengeResponse`](../../../docs-extract/_definitions/ChallengeResponse.md),
//! [`ApiKeyLoginResponse`](../../../docs-extract/_definitions/ApiKeyLoginResponse.md),
//! [`Feed`](../../../docs-extract/_definitions/Feed.md), and
//! [`LoggedInStatus`](../../../docs-extract/_definitions/LoggedInStatus.md).
//!
//! ## Relationship to [`crate::auth`]
//!
//! [`crate::auth`] is foundation code (locked at Phase 0). It carries the
//! lower-level versions of [`ApiKeyStartLoginRequest`],
//! [`ApiKeyVerifyLoginRequest`], [`ChallengeResponse`], and
//! [`ApiKeyLoginResponse`] used by the [`crate::auth::Session`] /
//! [`crate::auth::sign_challenge`] helpers. The Phase 0 version of
//! `ApiKeyLoginResponse` types `private_feed` / `public_feed` as
//! `Option<serde_json::Value>` because the foundation pre-dated the typed
//! [`Feed`] shape.
//!
//! Per the contract, foundation cannot be edited. So the `login` resource
//! group re-defines the request/response types here in their fully typed
//! form, and exposes [`ApiKeyLoginResponse::to_session`] so callers can
//! still build a [`crate::auth::Session`] for [`crate::Client::with_session`].
//!
//! Phase 3X will reconcile the two definitions into a single canonical home.

use crate::auth::Session;
use serde::{Deserialize, Serialize};

/// Request body for `POST /login/start`.
///
/// Schema: `docs-extract/_definitions/ApiKeyStartLoginRequest.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyStartLoginRequest {
    /// The API key provided by Nordnet. Found on the user's profile page
    /// after uploading the matching public key.
    pub api_key: String,
}

/// Response body from `POST /login/start`.
///
/// Schema: `docs-extract/_definitions/ChallengeResponse.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChallengeResponse {
    /// The challenge string the caller must sign with their private key.
    /// Valid for 30 seconds only.
    pub challenge: String,
}

/// Request body for `POST /login/verify`.
///
/// Schema: `docs-extract/_definitions/ApiKeyVerifyLoginRequest.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyVerifyLoginRequest {
    /// The API key provided by Nordnet.
    pub api_key: String,
    /// The service name (provided by Nordnet).
    pub service: String,
    /// The signed and base64 encoded challenge string created by the user.
    /// See [`crate::auth::sign_challenge`] for the signing helper.
    pub signature: String,
}

/// Connection information for one of the streaming feeds.
///
/// Schema: `docs-extract/_definitions/Feed.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
///
/// Schema: `docs-extract/_definitions/ApiKeyLoginResponse.md`. This is the
/// fully typed Phase 3 version; the [`crate::auth::ApiKeyLoginResponse`]
/// foundation version types `private_feed` / `public_feed` as raw
/// [`serde_json::Value`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
    /// Bridges the typed Phase 3 response back to the foundation
    /// [`Session`] type so callers can attach it via
    /// [`crate::Client::with_session`].
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
///
/// Schema: `docs-extract/_definitions/LoggedInStatus.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LoggedInStatus {
    /// `true` if the session is valid.
    pub logged_in: bool,
}
