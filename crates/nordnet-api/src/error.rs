//! Error type for the Nordnet API client.
//!
//! Mirrors the documented HTTP status codes (400, 401, 403, 429, 503) plus
//! transport-level failures. Every non-2xx response carries the raw response
//! body string so callers can surface the documented `ErrorResponse` shape
//! (`{"code": ..., "message": ...}`) without re-parsing in this layer.
//!
//! Status mapping (per `docs-source/nordnet-api-v2.html`):
//! - 400 -> [`Error::BadRequest`] ("Invalid parameter.")
//! - 401 -> [`Error::Unauthorized`] ("Unauthorized to log in ...")
//! - 403 -> [`Error::Forbidden`]
//! - 429 -> [`Error::TooManyRequests`] (caller decides backoff; the docs
//!   suggest 10s, but the library never sleeps or retries — POST/PUT
//!   on `/orders` is non-idempotent and a hidden retry could double-place)
//! - 503 -> [`Error::ServiceUnavailable`] (caller decides backoff; the
//!   server's `Retry-After` header is preserved on the underlying response
//!   but the library does not honor it automatically)
//! - any other non-2xx -> [`Error::UnexpectedStatus`]

use thiserror::Error;

/// All recoverable failures from the Nordnet API client.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP 400 — invalid parameter per docs.
    #[error("400 Bad Request: {body}")]
    BadRequest { body: String },

    /// HTTP 401 — unauthorized (typically rejected credentials).
    #[error("401 Unauthorized: {body}")]
    Unauthorized { body: String },

    /// HTTP 403 — forbidden.
    #[error("403 Forbidden: {body}")]
    Forbidden { body: String },

    /// HTTP 429 — Too Many Requests. The library does not retry; the
    /// caller chooses whether to back off and re-issue.
    #[error("429 Too Many Requests: {body}")]
    TooManyRequests { body: String },

    /// HTTP 503 — Service Unavailable. The library does not retry; the
    /// caller chooses whether to back off and re-issue (and is responsible
    /// for honoring `Retry-After` if present in the underlying response).
    #[error("503 Service Unavailable: {body}")]
    ServiceUnavailable { body: String },

    /// Any non-2xx response not specifically modelled above.
    #[error("HTTP {status}: {body}")]
    UnexpectedStatus { status: u16, body: String },

    /// Underlying reqwest transport failure (DNS, connect, TLS, timeout, ...).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Response body was not valid JSON for the expected type.
    #[error("response body did not match expected schema: {source}; body was: {body}")]
    Decode {
        #[source]
        source: serde_json::Error,
        body: String,
    },

    /// Failure during the SSH-key login flow (key parsing, algorithm
    /// mismatch, encrypted-key rejection, …). The wrapped
    /// [`nordnet_model::AuthError`] carries the specific failure mode.
    #[error("authentication failure: {0}")]
    Auth(#[from] nordnet_model::AuthError),

    /// Header value construction failed (typically because credentials
    /// contain bytes that are not valid for an HTTP header).
    #[error("invalid header value: {0}")]
    InvalidHeader(String),

    /// Form-urlencoded serialization failed (used by `Client::post_form`
    /// and `Client::put_form` for endpoints whose Swagger 2.0 parameters
    /// are marked `FormData`).
    #[error("form-urlencoded serialization failed: {0}")]
    EncodeForm(String),
}

impl Error {
    /// Build the appropriate variant from an HTTP status code + response body.
    pub(crate) fn from_status(status: u16, body: String) -> Self {
        match status {
            400 => Error::BadRequest { body },
            401 => Error::Unauthorized { body },
            403 => Error::Forbidden { body },
            429 => Error::TooManyRequests { body },
            503 => Error::ServiceUnavailable { body },
            other => Error::UnexpectedStatus {
                status: other,
                body,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_statuses() {
        assert!(matches!(
            Error::from_status(400, "x".into()),
            Error::BadRequest { .. }
        ));
        assert!(matches!(
            Error::from_status(401, "x".into()),
            Error::Unauthorized { .. }
        ));
        assert!(matches!(
            Error::from_status(403, "x".into()),
            Error::Forbidden { .. }
        ));
        assert!(matches!(
            Error::from_status(429, "x".into()),
            Error::TooManyRequests { .. }
        ));
        assert!(matches!(
            Error::from_status(503, "x".into()),
            Error::ServiceUnavailable { .. }
        ));
        assert!(matches!(
            Error::from_status(418, "x".into()),
            Error::UnexpectedStatus { status: 418, .. }
        ));
    }
}
