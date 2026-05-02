//! Types reused across resource groups.
//!
//! Phase 0 locked the original surface (`ErrorResponse`, `Currency`, `Money`,
//! `Amount`, `Timestamp`). Phase 3X (cross-endpoint type consistency) is
//! the only later phase permitted to extend this module — it adds shared
//! types that were independently re-derived by ≥3 group implementers, plus
//! a small set of (de)serialization adapters that the same number of groups
//! had copy-pasted locally.
//!
//! Phase 3X additions are tagged with `// added by Phase 3X` in the source
//! and listed in PROCESS.md §"Locked decisions" item 11. After Phase 3X
//! this module is locked again — Phase 4 (CLI) treats it read-only.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Standard error description as defined under `#_errorresponse` in the
/// reference HTML.
///
/// `code` is required, `message` is optional (and human-translated).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

/// A monetary amount in a specific currency, using the field name `amount`.
///
/// `amount` is `rust_decimal::Decimal` — never `f64` (per CONTRACTS.md).
/// This type is provided for endpoints that nest a `{amount, currency}`
/// object literally; for the much more common Nordnet `{value, currency}`
/// shape (the documented `_definitions/Amount.md` schema), see
/// [`AmountWithCurrency`].
///
/// Currently unused — kept in case a `{amount, currency}` shape surfaces
/// in a later doc revision; removing it would require coordinated edits to
/// the foundation lock.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

/// A monetary amount with attached currency, encoded as
/// `{currency: <Currency>, value: <Decimal>}` per the documented
/// `_definitions/Amount.md` schema.
///
/// Added by Phase 3X. Two groups (`accounts`, `orders`) had independently
/// derived structurally-equivalent local types from the same `Amount`
/// schema (`accounts::Amount` and `orders::OrderAmount`), differing only
/// in whether `currency` was typed as `String` or [`Currency`]. They now
/// both use this shared type, normalizing on the [`Currency`] newtype.
///
/// Cannot derive [`Eq`] because `value` is a `Decimal` carried under the
/// `arbitrary_precision` adapter (`PartialEq` only).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AmountWithCurrency {
    /// The amount currency.
    pub currency: Currency,
    /// The amount value. `Decimal` per CONTRACTS.md (never `f64`).
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub value: Decimal,
}

/// Common timestamp type for fields that the docs mark as ISO 8601. Use
/// the [`time::serde::iso8601`] adapter at the field level:
///
/// ```ignore
/// #[serde(with = "time::serde::iso8601")]
/// pub created_at: Timestamp,
/// ```
pub type Timestamp = OffsetDateTime;

/// Serde adapter for `Option<Decimal>` that uses arbitrary-precision number
/// encoding (matches the `arbitrary_precision` adapter applied to
/// non-optional `Decimal` fields).
///
/// Added by Phase 3X. Four groups (`accounts`, `instruments`,
/// `instrument_search`, `main_search`) had each carried a byte-identical
/// private copy of this module. They now reference it from here via
/// `#[serde(with = "crate::models::shared::opt_arb_prec")]`.
///
/// `rust_decimal` exposes `arbitrary_precision_option` directly; this
/// wrapper exists so the import surface in the field attributes stays
/// uniform with the non-optional case (`with = "..."`) and so changes to
/// the underlying implementation can be made in one place.
pub mod opt_arb_prec {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapped<'a>(#[serde(with = "rust_decimal::serde::arbitrary_precision")] &'a Decimal);
        value.as_ref().map(Wrapped).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wrapped(#[serde(with = "rust_decimal::serde::arbitrary_precision")] Decimal);
        Ok(Option::<Wrapped>::deserialize(deserializer)?.map(|w| w.0))
    }
}

/// Serde adapter for `time::Date` that round-trips the documented
/// `string(date)` wire form (`"YYYY-MM-DD"`).
///
/// Added by Phase 3X. Three groups (`accounts`, `instruments`, `tradables`)
/// each carried `string`-typed date fields with TODO notes pointing here;
/// they now use `time::Date` via this adapter.
///
/// Use at the field level with `#[serde(with = "crate::models::shared::date_iso8601")]`
/// (or `date_iso8601::option` for `Option<Date>`).
pub mod date_iso8601 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::macros::format_description;
    use time::Date;

    const FORMAT: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day]");

    pub fn serialize<S>(value: &Date, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .format(&FORMAT)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Date::parse(&s, &FORMAT).map_err(serde::de::Error::custom)
    }

    /// `Option<Date>` flavor of the same `YYYY-MM-DD` adapter.
    pub mod option {
        use super::FORMAT;
        use serde::{Deserialize, Deserializer, Serialize, Serializer};
        use time::Date;

        pub fn serialize<S>(value: &Option<Date>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match value {
                Some(d) => d
                    .format(&FORMAT)
                    .map_err(serde::ser::Error::custom)?
                    .serialize(serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let opt = Option::<String>::deserialize(deserializer)?;
            match opt {
                Some(s) => Date::parse(&s, &FORMAT)
                    .map(Some)
                    .map_err(serde::de::Error::custom),
                None => Ok(None),
            }
        }
    }

    /// `Vec<Date>` flavor of the same `YYYY-MM-DD` adapter, used by
    /// schemas that document an array of dates (e.g.
    /// `LeverageFilter.expiration_dates`).
    pub mod vec {
        use super::FORMAT;
        use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serializer};
        use time::Date;

        pub fn serialize<S>(value: &[Date], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(value.len()))?;
            for d in value {
                let s = d.format(&FORMAT).map_err(serde::ser::Error::custom)?;
                seq.serialize_element(&s)?;
            }
            seq.end()
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Date>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let raws = Vec::<String>::deserialize(deserializer)?;
            raws.into_iter()
                .map(|s| Date::parse(&s, &FORMAT).map_err(serde::de::Error::custom))
                .collect()
        }
    }
}

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
    fn money_uses_decimal() {
        let m = Money {
            amount: Decimal::new(12345, 2),
            currency: Currency("SEK".into()),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: Money = serde_json::from_str(&s).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn amount_with_currency_round_trips() {
        let a = AmountWithCurrency {
            currency: Currency("SEK".into()),
            value: Decimal::new(98765, 2),
        };
        let s = serde_json::to_string(&a).unwrap();
        // Document the wire shape and field order produced by serde for
        // this struct (currency first, value second).
        assert_eq!(s, r#"{"currency":"SEK","value":987.65}"#);
        let back: AmountWithCurrency = serde_json::from_str(&s).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn opt_arb_prec_round_trips_some_and_none() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct W {
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                with = "opt_arb_prec"
            )]
            v: Option<Decimal>,
        }
        let some = W {
            v: Some(Decimal::new(31415, 4)),
        };
        let s = serde_json::to_string(&some).unwrap();
        assert_eq!(s, r#"{"v":3.1415}"#);
        let back: W = serde_json::from_str(&s).unwrap();
        assert_eq!(back, some);

        let none = W { v: None };
        let s = serde_json::to_string(&none).unwrap();
        assert_eq!(s, "{}");
    }

    #[test]
    fn date_iso8601_round_trip() {
        use time::macros::date;
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct W {
            #[serde(with = "date_iso8601")]
            d: time::Date,
        }
        let raw = r#"{"d":"2025-12-19"}"#;
        let parsed: W = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.d, date!(2025 - 12 - 19));
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw);
    }

    #[test]
    fn date_iso8601_option_round_trip() {
        use time::macros::date;
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct W {
            #[serde(default, with = "date_iso8601::option")]
            d: Option<time::Date>,
        }
        let raw_some = r#"{"d":"2026-05-02"}"#;
        let parsed: W = serde_json::from_str(raw_some).unwrap();
        assert_eq!(parsed.d, Some(date!(2026 - 05 - 02)));
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw_some);

        let raw_null = r#"{"d":null}"#;
        let parsed: W = serde_json::from_str(raw_null).unwrap();
        assert_eq!(parsed.d, None);
        // Serializing None goes back as `null` here because we did not set
        // `skip_serializing_if`. Group fields that mark the field optional
        // additionally use `skip_serializing_if = "Option::is_none"`.
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw_null);
    }

    #[test]
    fn date_iso8601_vec_round_trip() {
        use time::macros::date;
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct W {
            #[serde(with = "date_iso8601::vec")]
            ds: Vec<time::Date>,
        }
        let raw = r#"{"ds":["2025-12-19","2026-01-15"]}"#;
        let parsed: W = serde_json::from_str(raw).unwrap();
        assert_eq!(
            parsed.ds,
            vec![date!(2025 - 12 - 19), date!(2026 - 01 - 15)]
        );
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw);
    }

    #[test]
    fn date_iso8601_rejects_bad_format() {
        #[derive(Deserialize, Debug)]
        struct W {
            #[allow(dead_code)]
            #[serde(with = "date_iso8601")]
            d: time::Date,
        }
        // Too few components.
        let r: Result<W, _> = serde_json::from_str(r#"{"d":"2025-12"}"#);
        assert!(r.is_err());
        // Wrong separator.
        let r: Result<W, _> = serde_json::from_str(r#"{"d":"2025/12/19"}"#);
        assert!(r.is_err());
    }
}
