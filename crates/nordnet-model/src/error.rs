//! Errors raised by [`crate::auth`].
//!
//! These cover only what the auth module can fail at — there are no HTTP
//! variants. The REST crate (`nordnet-api`) wraps this enum into its own
//! `Error::Auth(#[from] AuthError)` variant.

use thiserror::Error;

/// All recoverable failures from [`crate::auth`].
#[derive(Debug, Error)]
pub enum AuthError {
    /// The supplied PEM did not parse as an OpenSSH private key, or the
    /// parser rejected the contents (e.g. malformed framing). The wrapped
    /// string carries the underlying parser's message for diagnostics.
    #[error("invalid private key: {0}")]
    InvalidKey(String),

    /// The OpenSSH key parsed successfully but is encrypted. Decrypt the
    /// key out-of-band before passing it back in.
    #[error("encrypted private keys are not supported")]
    EncryptedKey,

    /// The OpenSSH key uses an algorithm other than Ed25519 (RSA, ECDSA,
    /// DSA, …). Nordnet's external API v2 requires Ed25519 keys.
    #[error("wrong key algorithm: got {got}, expected {expected}")]
    WrongAlgorithm { got: String, expected: &'static str },

    /// The key declared Ed25519 but the embedded key data was a different
    /// shape — should not happen with well-formed `ssh-keygen` output, but
    /// surfaced explicitly so consumers can distinguish it from
    /// [`AuthError::InvalidKey`].
    #[error("ed25519 key data length mismatch")]
    KeyDataMismatch,
}
