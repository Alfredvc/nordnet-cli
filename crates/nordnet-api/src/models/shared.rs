//! Types reused across resource groups. Locked after Phase 0.
//!
//! Nothing in this file is added by Phase 3+ implementers — they either use
//! these types as-is or add private types in their own group module. See
//! CONTRACTS.md "Mod files" / "Type rules".

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Standard error description as defined under `#_errorresponse` in the
/// reference HTML.
///
/// `code` is required, `message` is optional (and human-translated).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// ISO 4217 currency code as it appears in Nordnet payloads (e.g. `"SEK"`,
/// `"EUR"`). The API encodes it as a plain JSON string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Currency(pub String);

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<&str> for Currency {
    fn from(v: &str) -> Self {
        Self(v.to_owned())
    }
}

/// A monetary amount in a specific currency.
///
/// `amount` is `rust_decimal::Decimal` — never `f64` (per CONTRACTS.md).
/// Many Nordnet endpoints return amounts and currencies as separate fields
/// rather than as a nested object; group implementers compose [`Money`]
/// from those fields where appropriate.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Money {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub amount: Decimal,
    pub currency: Currency,
}

/// A monetary amount without an attached currency. Some Nordnet response
/// shapes use a bare numeric field where the currency is implied by the
/// surrounding object. Use this rather than `Decimal` directly so the
/// "money is decimal, never float" invariant is visible at the type level.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Amount(#[serde(with = "rust_decimal::serde::arbitrary_precision")] pub Decimal);

/// Common timestamp type for fields that the docs mark as ISO 8601. Use
/// the [`time::serde::iso8601`] adapter at the field level:
///
/// ```ignore
/// #[serde(with = "time::serde::iso8601")]
/// pub created_at: Timestamp,
/// ```
pub type Timestamp = OffsetDateTime;

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn error_response_round_trip() {
        let raw = r#"{"code":"NEXT_BAD","message":"Nope."}"#;
        let parsed: ErrorResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.code, "NEXT_BAD");
        assert_eq!(parsed.message.as_deref(), Some("Nope."));
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw);
    }

    #[test]
    fn error_response_message_optional() {
        let raw = r#"{"code":"NO_MSG"}"#;
        let parsed: ErrorResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.message, None);
        // Serialization should omit `message` because we set
        // `skip_serializing_if = Option::is_none`.
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw);
    }

    #[test]
    fn error_response_rejects_unknown_fields() {
        let raw = r#"{"code":"X","extra":"y"}"#;
        let r: Result<ErrorResponse, _> = serde_json::from_str(raw);
        assert!(r.is_err());
    }

    #[test]
    fn money_uses_decimal() {
        let m = Money {
            amount: Decimal::new(12345, 2),
            currency: Currency("SEK".into()),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: Money = serde_json::from_str(&s).unwrap();
        assert_eq!(back, m);
    }
}
