//! Ed25519 SSH-key login flow per Nordnet's official External API v2
//! examples (`nordnet/next-api-v2-examples`, Sep 2025) and the
//! "Getting Started" guide at <https://www.nordnet.se/externalapi/docs/getting_started>.
//!
//! ## Wire flow
//!
//! 1. `POST /api/2/login/start`
//!    - Body: [`ApiKeyStartLoginRequest`] `{ api_key }` (JSON)
//!    - 200 → [`ChallengeResponse`] `{ challenge }` — a short string
//!      (UUID in the live examples) valid for ~30s.
//! 2. The caller signs the raw UTF-8 bytes of `challenge` with their
//!    Ed25519 private key, then base64-encodes the 64-byte signature.
//! 3. `POST /api/2/login/verify`
//!    - Body: [`ApiKeyVerifyLoginRequest`] `{ api_key, service, signature }`
//!    - 200 → [`ApiKeyLoginResponse`] `{ session_key, expires_in, ... }`.
//! 4. Subsequent requests authenticate by setting
//!    `Authorization: Basic base64(session_key:session_key)`.
//!
//! ## Signature scheme
//!
//! Pure Ed25519 (EdDSA over Curve25519, no pre-hash, no context) on the
//! raw UTF-8 bytes of the challenge string. The 64-byte signature is
//! base64-encoded and sent verbatim.
//!
//! Verified against:
//! - Official Python: `nordnet/next-api-v2-examples/python3/sign.py`
//!   uses `cryptography.hazmat.primitives.serialization.load_ssh_private_key`
//!   followed by `private_key.sign(challenge.encode('utf-8'))`.
//! - Official docs: "ssh-keygen -t ed25519 -a 150" produces the key;
//!   "RAW signing with no namespace, base64 encode the result".
//!
//! Keys are loaded from the OpenSSH on-disk format
//! (`-----BEGIN OPENSSH PRIVATE KEY-----`) — the same format
//! `ssh-keygen -t ed25519` produces by default. PKCS#8 wrappers are not
//! accepted (none of the Python or doc examples use them).

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use ssh_key::{private::KeypairData, Algorithm, PrivateKey};

/// Request body for `POST /login/start`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyStartLoginRequest {
    pub api_key: String,
}

/// Response body from `POST /login/start`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
/// `private_feed`/`public_feed` are documented as `Feed` objects. They
/// are accepted but kept opaque here (typed by the `login` group's
/// follow-up if needed).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyLoginResponse {
    pub session_key: String,
    pub expires_in: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_feed: Option<serde_json::Value>,
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

/// Sign the challenge string with the caller's Ed25519 private key.
///
/// Pure Ed25519 over the raw UTF-8 bytes of `challenge`. Returns the
/// base64-encoded 64-byte signature — the format expected by
/// [`ApiKeyVerifyLoginRequest::signature`].
pub fn sign_challenge(
    private_key: &SigningKey,
    challenge: &str,
) -> Result<String, crate::error::Error> {
    let signature = private_key.sign(challenge.as_bytes());
    Ok(B64.encode(signature.to_bytes()))
}

/// Parse an unencrypted OpenSSH-format Ed25519 private key.
///
/// Accepts the on-disk format produced by `ssh-keygen -t ed25519`
/// (`-----BEGIN OPENSSH PRIVATE KEY-----`). Encrypted keys are
/// rejected with a descriptive error — decrypt them out-of-band first.
/// Non-Ed25519 algorithms (RSA, ECDSA, DSA) are also rejected.
pub fn parse_private_key_openssh(text: &str) -> Result<SigningKey, crate::error::Error> {
    let pk = PrivateKey::from_openssh(text)
        .map_err(|e| crate::error::Error::Auth(format!("invalid OpenSSH private key: {e}")))?;

    if pk.is_encrypted() {
        return Err(crate::error::Error::Auth(
            "encrypted SSH keys are not supported; decrypt the key first".to_owned(),
        ));
    }

    if pk.algorithm() != Algorithm::Ed25519 {
        return Err(crate::error::Error::Auth(format!(
            "expected Ed25519 SSH key, got {}",
            pk.algorithm().as_str()
        )));
    }

    match pk.key_data() {
        KeypairData::Ed25519(kp) => Ok(SigningKey::from_bytes(kp.private.as_ref())),
        _ => Err(crate::error::Error::Auth(
            "key declared Ed25519 but key_data is not Ed25519".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;
    use pretty_assertions::assert_eq;

    /// Deterministic Ed25519 signing key from a fixed 32-byte seed.
    /// No RNG dependency — Ed25519 keys are just 32 bytes of seed.
    fn fixed_test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn sign_challenge_is_deterministic_for_fixed_key() {
        let key = fixed_test_key();
        let challenge = "the-challenge-string";

        // Ed25519 is deterministic by spec: same key + same message →
        // same signature.
        let s1 = sign_challenge(&key, challenge).unwrap();
        let s2 = sign_challenge(&key, challenge).unwrap();
        assert_eq!(s1, s2, "Ed25519 must be deterministic");

        // Different challenge → different signature.
        let s3 = sign_challenge(&key, "other").unwrap();
        assert_ne!(s1, s3);

        // Output is base64 of 64 raw signature bytes → 88-char string
        // ending with one `=` pad character.
        let raw = B64.decode(&s1).unwrap();
        assert_eq!(raw.len(), 64);
    }

    #[test]
    fn sign_then_verify_with_public_key_succeeds() {
        let key = fixed_test_key();
        let public = key.verifying_key();
        let challenge = b"abc123";

        let b64 = sign_challenge(&key, std::str::from_utf8(challenge).unwrap()).unwrap();
        let raw = B64.decode(&b64).unwrap();
        let sig_bytes: [u8; 64] = raw.try_into().unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        public
            .verify(challenge, &signature)
            .expect("signature must verify under the matching public key");
    }

    /// Build a real OpenSSH-format Ed25519 PEM string from a fixed seed.
    /// Round-trips a known seed through ssh-key's serializer so the
    /// parse test exercises the actual on-disk format ssh-keygen
    /// produces — without needing an entropy source or a fixture file.
    fn fixed_test_key_openssh() -> String {
        use ssh_key::private::Ed25519Keypair;
        use ssh_key::LineEnding;
        let kp = Ed25519Keypair::from_seed(&[7u8; 32]);
        let pk = ssh_key::PrivateKey::from(kp);
        pk.to_openssh(LineEnding::LF).unwrap().to_string()
    }

    #[test]
    fn parse_private_key_openssh_round_trips_seed() {
        let pem = fixed_test_key_openssh();
        let parsed = parse_private_key_openssh(&pem).unwrap();
        // Parsed seed must equal the seed we serialized.
        assert_eq!(parsed.to_bytes(), [7u8; 32]);
        // And signing through the parsed key must verify.
        let public = parsed.verifying_key();
        let sig_b64 = sign_challenge(&parsed, "ping").unwrap();
        let raw = B64.decode(&sig_b64).unwrap();
        let sig_bytes: [u8; 64] = raw.try_into().unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        public.verify(b"ping", &signature).expect("verifies");
    }

    #[test]
    fn parse_private_key_openssh_rejects_garbage() {
        let r = parse_private_key_openssh("not a key");
        assert!(matches!(r, Err(crate::error::Error::Auth(_))));
    }

    #[test]
    fn parse_private_key_openssh_rejects_rsa_pem() {
        // A PKCS#8 RSA PEM is unambiguously not an OpenSSH private
        // key — the BEGIN tag differs. The from_openssh parse fails
        // before our algorithm check runs, but the user-visible error
        // is still an `Auth(...)` variant — which is what we want.
        let pem = "-----BEGIN PRIVATE KEY-----\nMIIE...\n-----END PRIVATE KEY-----\n";
        let r = parse_private_key_openssh(pem);
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
