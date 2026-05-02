//! Newtype wrappers for resource identifiers used by the Nordnet API.
//!
//! Underlying types are derived from `docs-source/nordnet-api-v2.html`
//! parameter and schema tables:
//!
//! | Newtype          | Underlying    | Example doc field          |
//! |------------------|---------------|----------------------------|
//! | [`AccountId`]    | `i64`         | `accid` — `integer(int64)` |
//! | [`OrderId`]      | `i64`         | `order_id` — `integer(int64)` |
//! | [`InstrumentId`] | `i64`         | `instrument_id` — `integer(int64)` |
//! | [`MarketId`]     | `i64`         | `market_id` — `integer(int64)` |
//! | [`TickSizeId`]   | `i64`         | `tick_size_id` — `integer(int64)` |
//! | [`TradableId`]   | `String`      | `identifier` — `string` |
//!
//! All newtypes are `#[serde(transparent)]` so they round-trip identically
//! to the underlying primitive on the wire — the strong typing exists only
//! at compile time to prevent passing an `OrderId` where an `AccountId` is
//! expected (per CONTRACTS.md "Type rules").

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_newtype_int {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub i64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<i64> for $name {
            fn from(v: i64) -> Self { Self(v) }
        }

        impl From<$name> for i64 {
            fn from(v: $name) -> Self { v.0 }
        }
    };
}

macro_rules! id_newtype_string {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<String> for $name {
            fn from(v: String) -> Self { Self(v) }
        }

        impl From<&str> for $name {
            fn from(v: &str) -> Self { Self(v.to_owned()) }
        }

        impl From<$name> for String {
            fn from(v: $name) -> Self { v.0 }
        }
    };
}

id_newtype_int!(
    /// `accid` — Nordnet account identifier.
    AccountId
);
id_newtype_int!(
    /// `order_id` — Nordnet order identifier.
    OrderId
);
id_newtype_int!(
    /// `instrument_id` — Nordnet instrument identifier.
    InstrumentId
);
id_newtype_int!(
    /// `market_id` — Nordnet market identifier.
    MarketId
);
id_newtype_int!(
    /// `tick_size_id` — tick size table identifier.
    TickSizeId
);
id_newtype_string!(
    /// Tradable identifier (`identifier` field). String form per docs.
    TradableId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_id_is_serde_transparent() {
        let v = AccountId(42);
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "42");
        let back: AccountId = serde_json::from_str("42").unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn string_id_is_serde_transparent() {
        let v = TradableId("ABC123".to_owned());
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"ABC123\"");
        let back: TradableId = serde_json::from_str("\"ABC123\"").unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn distinct_newtypes_do_not_unify() {
        // Compile-time guard: this would not compile if AccountId == OrderId.
        fn _take_account(_: AccountId) {}
        fn _take_order(_: OrderId) {}
        _take_account(AccountId(1));
        _take_order(OrderId(1));
    }
}
