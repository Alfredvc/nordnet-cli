//! Resource methods for the `login` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|------------------|--------------------|
//! | POST   | `start_login`    | `/login/start`     |
//! | POST   | `verify_login`   | `/login/verify`    |
//! | PUT    | `refresh_session`| `/login`           |
//! | DELETE | `logout`         | `/login`           |
//!
//!
//! ## Body-less PUT
//!
//! `refresh_session` is documented with no request body. We use the
//! foundation [`Client::put_empty`] helper, which omits the
//! `Content-Type` header and sends a zero-length payload — the shape
//! Nordnet's `PUT /login` expects.

use crate::client::Client;
use crate::error::Error;
use nordnet_model::auth::{ApiKeyStartLoginRequest, ApiKeyVerifyLoginRequest, ChallengeResponse};
use nordnet_model::models::login::{ApiKeyLoginResponse, LoggedInStatus};

impl Client {
    /// `POST /login/start` — Start the authentication challenge.
    ///
    /// Returns a [`ChallengeResponse`] whose `challenge` field must be
    /// signed with the caller's RSA private key (see
    /// [`nordnet_model::auth::sign_challenge`]) before being passed to
    /// [`Client::verify_login`]. The challenge is valid for 30 seconds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::TooManyRequests`]
    /// (429), or [`Error::ServiceUnavailable`] (503) per the docs.
    pub async fn start_login(
        &self,
        request: &ApiKeyStartLoginRequest,
    ) -> Result<ChallengeResponse, Error> {
        self.post("/login/start", request).await
    }

    /// `POST /login/verify` — Complete the login flow with a signed
    /// challenge.
    ///
    /// On success the returned [`ApiKeyLoginResponse`] carries the
    /// `session_key`. Convert it to a [`nordnet_model::auth::Session`] via
    /// [`ApiKeyLoginResponse::to_session`] and attach it with
    /// [`Client::with_session`] for subsequent authenticated calls.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) per the docs.
    pub async fn verify_login(
        &self,
        request: &ApiKeyVerifyLoginRequest,
    ) -> Result<ApiKeyLoginResponse, Error> {
        self.post("/login/verify", request).await
    }

    /// `PUT /login` — Touch the session to keep it alive.
    ///
    /// Any other authenticated call also touches the session, so this is
    /// only needed when the application is otherwise idle for the full
    /// session timeout interval.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) per the docs.
    pub async fn refresh_session(&self) -> Result<LoggedInStatus, Error> {
        self.put_empty("/login").await
    }

    /// `DELETE /login` — Invalidate the current session.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) per the docs.
    pub async fn logout(&self) -> Result<LoggedInStatus, Error> {
        self.delete("/login").await
    }
}
