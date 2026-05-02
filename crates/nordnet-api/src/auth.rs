//! SSH-key login flow per `docs-source/nordnet-api-v2.html` §
//! "Start authentication challenge" / "Complete session (log in)".
//!
//! ## Wire flow
//!
//! 1. `POST /api/2/login/start`
//!    - Body: `ApiKeyStartLoginRequest { api_key }` (JSON)
//!    - 200 -> `ChallengeResponse { challenge }` — a string valid for 30s.
//! 2. The caller signs `challenge` with their RSA private key, then
//!    base64-encodes the raw signature bytes.
//! 3. `POST /api/2/login/verify`
//!    - Body: `ApiKeyVerifyLoginRequest { api_key, service, signature }`
//!    - 200 -> `ApiKeyLoginResponse { session_key, expires_in, ... }`.
//! 4. Subsequent requests authenticate by setting
//!    `Authorization: Basic base64(session_key:session_key)`.
//!
//! ## Signature scheme
//!
//! The published doc page (`#_apikeyverifyloginrequest`) only states that
//! `signature` is "the signed and base64 encoded challenge string". The
//! exact RSA scheme + hash live in the linked **Getting Started** guide
//! which is not part of `docs-source/nordnet-api-v2.html`. This crate uses
//! **RSA PKCS#1 v1.5 with SHA-256** because:
//!   - It is the standard signing scheme paired with `ssh-keygen -t rsa`
//!     (alongside `rsa-sha2-256` in OpenSSH);
//!   - It is deterministic — a unit test against a fixed key vector can
//!     pin the exact byte output (`Pkcs1v15Sign` is non-randomized);
//!   - It is what `rsa::pkcs1v15::SigningKey<Sha256>` produces by default.
//!
//! If the live API rejects this scheme, swap [`sign_challenge`] for the
//! correct algorithm and re-pin [`tests::sign_challenge_is_deterministic`].
//! The structural code (start/verify/session header) does not change.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rsa::{
    pkcs1v15::{Pkcs1v15Sign, SigningKey},
    pkcs8::DecodePrivateKey,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Request body for `POST /login/start`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyStartLoginRequest {
    pub api_key: String,
}

/// Response body from `POST /login/start`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChallengeResponse {
    pub challenge: String,
}

/// Request body for `POST /login/verify`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyVerifyLoginRequest {
    pub api_key: String,
    pub service: String,
    pub signature: String,
}

/// Response body from `POST /login/verify`.
///
/// `private_feed`/`public_feed` are documented as `Feed` objects. In
/// Phase 0 we keep the response minimal — Phase 3's `login` group adds
/// the full `Feed` type. The fields are accepted but ignored here so
/// the Phase 0 wiremock test doesn't depend on a Phase 3 type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyLoginResponse {
    pub session_key: String,
    pub expires_in: i64,
    /// Skipped in Phase 0; typed by Phase 3 `login` group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_feed: Option<serde_json::Value>,
    /// Skipped in Phase 0; typed by Phase 3 `login` group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_feed: Option<serde_json::Value>,
}

/// An authenticated session — what subsequent client calls need to set
/// the `Authorization` header.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_key: String,
    pub expires_in: i64,
}

impl Session {
    /// Build the `Authorization: Basic <base64(key:key)>` header value.
    pub fn basic_auth_header(&self) -> String {
        let raw = format!("{0}:{0}", self.session_key);
        format!("Basic {}", B64.encode(raw.as_bytes()))
    }
}

/// Sign the challenge string with the caller's RSA private key.
///
/// Uses PKCS#1 v1.5 + SHA-256 (deterministic). Returns the base64-encoded
/// raw signature bytes — the format expected by
/// `ApiKeyVerifyLoginRequest::signature`.
pub fn sign_challenge(
    private_key: &RsaPrivateKey,
    challenge: &str,
) -> Result<String, crate::error::Error> {
    let signing_key: SigningKey<Sha256> = SigningKey::new(private_key.clone());
    let signature = signing_key.sign(challenge.as_bytes());
    Ok(B64.encode(signature.to_bytes()))
}

/// Lower-level signer used by tests and by callers that want to assemble
/// a non-`SigningKey` flow. Equivalent to [`sign_challenge`].
pub fn sign_challenge_raw(
    private_key: &RsaPrivateKey,
    challenge: &[u8],
) -> Result<Vec<u8>, crate::error::Error> {
    let mut hasher = Sha256::new();
    hasher.update(challenge);
    let digest = hasher.finalize();
    private_key
        .sign(Pkcs1v15Sign::new::<Sha256>(), &digest)
        .map_err(|e| crate::error::Error::Auth(format!("rsa sign failed: {e}")))
}

/// Parse a PEM-encoded PKCS#8 RSA private key. Convenience wrapper over
/// `RsaPrivateKey::from_pkcs8_pem` so callers don't need to depend on
/// `rsa` directly.
pub fn parse_private_key_pem(pem: &str) -> Result<RsaPrivateKey, crate::error::Error> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .map_err(|e| crate::error::Error::Auth(format!("invalid PKCS#8 PEM: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rsa::{pkcs8::EncodePrivateKey, RsaPrivateKey};

    /// Build a deterministic 2048-bit RSA key from a fixed seed. We use
    /// `rand::rngs::StdRng` with a known seed so the test does not depend
    /// on system randomness.
    ///
    /// We avoid pulling `rand` as a workspace dep by re-using the `rand`
    /// re-export bundled with `rsa` itself.
    fn fixed_test_key() -> RsaPrivateKey {
        // Deterministic 2048-bit RSA key from a fixed ChaCha20 seed —
        // gives the test reproducibility without depending on system
        // entropy. `rand_chacha` 0.3 implements `rand_core` 0.6 which
        // is what `rsa` 0.9 expects.
        use rand_chacha::rand_core::SeedableRng;
        let mut rng = rand_chacha::ChaCha20Rng::from_seed([7u8; 32]);
        RsaPrivateKey::new(&mut rng, 2048).expect("rsa keygen")
    }

    #[test]
    fn sign_challenge_is_deterministic_for_fixed_key() {
        let key = fixed_test_key();
        let challenge = "the-challenge-string";

        // Two signs with the same input produce the same bytes
        // (PKCS#1 v1.5 is deterministic, by design).
        let s1 = sign_challenge_raw(&key, challenge.as_bytes()).unwrap();
        let s2 = sign_challenge_raw(&key, challenge.as_bytes()).unwrap();
        assert_eq!(s1, s2, "PKCS#1 v1.5 + SHA-256 must be deterministic");

        // Helper signature equals the high-level helper.
        let b64 = sign_challenge(&key, challenge).unwrap();
        assert_eq!(b64, B64.encode(&s1));

        // A different challenge produces different bytes.
        let s3 = sign_challenge_raw(&key, b"other").unwrap();
        assert_ne!(s1, s3);
    }

    #[test]
    fn sign_then_verify_with_public_key_succeeds() {
        use rsa::pkcs1v15::VerifyingKey;
        use rsa::signature::Verifier;
        use rsa::RsaPublicKey;

        let key = fixed_test_key();
        let public: RsaPublicKey = key.to_public_key();
        let challenge = b"abc123";

        let raw_sig = sign_challenge_raw(&key, challenge).unwrap();
        let signature: rsa::pkcs1v15::Signature = raw_sig.as_slice().try_into().unwrap();
        let verifier: VerifyingKey<Sha256> = VerifyingKey::new(public);
        verifier
            .verify(challenge, &signature)
            .expect("signature must verify under the matching public key");
    }

    #[test]
    fn parse_private_key_pem_round_trips() {
        let key = fixed_test_key();
        let pem = key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).unwrap();
        let parsed = parse_private_key_pem(&pem).unwrap();
        assert_eq!(parsed, key);
    }

    #[test]
    fn parse_private_key_pem_rejects_garbage() {
        let r = parse_private_key_pem("not a pem");
        assert!(matches!(r, Err(crate::error::Error::Auth(_))));
    }

    #[test]
    fn session_basic_auth_header_format() {
        let s = Session {
            session_key: "abc".to_owned(),
            expires_in: 60,
        };
        // base64("abc:abc") = "YWJjOmFiYw=="
        assert_eq!(s.basic_auth_header(), "Basic YWJjOmFiYw==");
    }

    #[test]
    fn challenge_response_round_trip() {
        let raw = r#"{"challenge":"abc"}"#;
        let parsed: ChallengeResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.challenge, "abc");
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw);
    }

    #[test]
    fn api_key_login_response_minimal() {
        let raw = r#"{"session_key":"S","expires_in":300}"#;
        let parsed: ApiKeyLoginResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.session_key, "S");
        assert_eq!(parsed.expires_in, 300);
    }
}
