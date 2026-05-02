//! Resource methods for the `instruments` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|----|------|
//! | GET | `lookup` | `/instruments/lookup/{lookup_type}/{lookup}` |
//! | GET | `list_types` | `/instruments/types` |
//! | GET | `get_type` | `/instruments/types/{instrument_type}` |
//! | GET | `list_underlyings` | `/instruments/underlyings/{derivative_type}/{currency}` |
//! | GET | `get_instrument_suitability` | `/instruments/validation/suitability/{instrument_id}` |
//! | GET | `get_instrument` | `/instruments/{instrument_id}` |
//! | GET | `list_leverages` | `/instruments/{instrument_id}/leverages` |
//! | GET | `get_leverage_filters` | `/instruments/{instrument_id}/leverages/filters` |
//! | GET | `list_instrument_trades` | `/instruments/{instrument_id}/trades` |
//!
//! ## Naming
//!
//! Two ops are renamed from their docs name so they can co-exist on
//! [`Client`] alongside same-named ops in other groups (Rust resolves all
//! resource methods onto a single `Client` impl):
//!
//! - `list_trades` -> `list_instrument_trades` — to coexist with
//!   `tradables::list_tradable_trades` and the future
//!   `accounts::list_trades`.
//! - `get_suitability` -> `get_instrument_suitability` — to coexist with
//!   `tradables::get_suitability`.
//!
//! Phase 3X may pick a uniform naming scheme.
//!
//! ## 204 No Content
//!
//! All ops except `list_types` and `get_leverage_filters` document a 204
//! response. The base [`Client::get`] surfaces an empty body as a
//! [`Error::Decode`]; each method here maps that case to an empty `Vec`
//! (mirroring the `tradables` precedent). `get_leverage_filters` returns a
//! single `LeverageFilter` (no array shape) so 204 is not mapped.

use crate::client::Client;
use crate::error::Error;
use crate::ids::InstrumentId;
use crate::models::instruments::{
    Instrument, InstrumentEligibility, InstrumentPublicTrades, InstrumentType, IssuerId,
    LeverageFilter,
};

/// Optional query parameters for [`Client::list_leverages`] and
/// [`Client::get_leverage_filters`].
///
/// All six fields are documented as optional. Built with
/// `LeveragesQuery::default()` and field-by-field assignment, then passed
/// by reference to the resource methods.
#[derive(Debug, Clone, Default)]
pub struct LeveragesQuery<'a> {
    /// Show only leverage instruments with a specific currency.
    pub currency: Option<&'a str>,
    /// Show only leverage instruments with a specific expiration date
    /// (`YYYY-MM-DD`).
    pub expiration_date: Option<&'a str>,
    /// Show only instruments with a specific instrument group type.
    pub instrument_group_type: Option<&'a str>,
    /// Show only instruments with a specific instrument type.
    pub instrument_type: Option<&'a str>,
    /// Show only leverage instruments from a specific issuer.
    pub issuer_id: Option<IssuerId>,
    /// Show only leverage instruments with a specific market view
    /// (`D` or `U`).
    pub market_view: Option<&'a str>,
}

/// Build the encoded query string for the leverages endpoints.
///
/// Uses `reqwest::Url::query_pairs_mut` so all percent-encoding follows
/// the standard URL form rules. The placeholder host is never sent
/// anywhere — only the encoded query suffix is extracted.
fn build_leverages_query(filters: &LeveragesQuery<'_>) -> String {
    let mut url = match reqwest::Url::parse("http://_/") {
        Ok(u) => u,
        // The literal above is a valid absolute URL — this branch is
        // unreachable in practice. Returning an empty string keeps the
        // function total without panicking.
        Err(_) => return String::new(),
    };
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(v) = filters.currency {
            pairs.append_pair("currency", v);
        }
        if let Some(v) = filters.expiration_date {
            pairs.append_pair("expiration_date", v);
        }
        if let Some(v) = filters.instrument_group_type {
            pairs.append_pair("instrument_group_type", v);
        }
        if let Some(v) = filters.instrument_type {
            pairs.append_pair("instrument_type", v);
        }
        if let Some(v) = filters.issuer_id {
            pairs.append_pair("issuer_id", &v.0.to_string());
        }
        if let Some(v) = filters.market_view {
            pairs.append_pair("market_view", v);
        }
    }
    url.query().unwrap_or("").to_owned()
}

impl Client {
    /// `GET /instruments/lookup/{lookup_type}/{lookup}` — Lookup specific
    /// instruments with predefined fields.
    ///
    /// `lookup_type` must be one of the documented enum values:
    /// `market_id_identifier` or `isin_code_currency_market_id`.
    /// `lookup` is formatted as `[market_id]:[identifier]` or
    /// `[isin]:[currency]:[market_id]` respectively. Multiple entries are
    /// comma-separated. Pass-through `&str` for now.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn lookup(&self, lookup_type: &str, lookup: &str) -> Result<Vec<Instrument>, Error> {
        let path = format!("/instruments/lookup/{lookup_type}/{lookup}");
        match self.get::<Vec<Instrument>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /instruments/types` — Returns all Nordnet instrument types.
    ///
    /// # Errors
    ///
    /// [`Error::Unauthorized`] (401), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn list_types(&self) -> Result<Vec<InstrumentType>, Error> {
        self.get::<Vec<InstrumentType>>("/instruments/types").await
    }

    /// `GET /instruments/types/{instrument_type}` — Returns information
    /// about one or more Nordnet instrument types.
    ///
    /// `instrument_type` is one or more comma-separated type codes
    /// (pass-through `&str`).
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::Unauthorized`] (401), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn get_type(&self, instrument_type: &str) -> Result<Vec<InstrumentType>, Error> {
        let path = format!("/instruments/types/{instrument_type}");
        match self.get::<Vec<InstrumentType>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /instruments/underlyings/{derivative_type}/{currency}` —
    /// Returns instruments that are underlyings for a specific type of
    /// instruments.
    ///
    /// `derivative_type` is one of `leverage` or `option_pair`. `currency`
    /// is the derivative currency (note the underlying may have a
    /// different currency). Both pass-through `&str`.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn list_underlyings(
        &self,
        derivative_type: &str,
        currency: &str,
    ) -> Result<Vec<Instrument>, Error> {
        let path = format!("/instruments/underlyings/{derivative_type}/{currency}");
        match self.get::<Vec<Instrument>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /instruments/validation/suitability/{instrument_id}` —
    /// Returns the customer's trading eligibility for the given
    /// instrument(s).
    ///
    /// Renamed from the docs op `get_suitability` to
    /// `get_instrument_suitability` to coexist with
    /// `tradables::get_suitability` on the same `Client` impl.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403; documented for anonymous sessions and
    /// returned with an empty body), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn get_instrument_suitability(
        &self,
        instrument_id: InstrumentId,
    ) -> Result<Vec<InstrumentEligibility>, Error> {
        let path = format!("/instruments/validation/suitability/{instrument_id}");
        match self.get::<Vec<InstrumentEligibility>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /instruments/{instrument_id}` — Returns instrument information
    /// for the given instrument ID(s).
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn get_instrument(
        &self,
        instrument_id: InstrumentId,
    ) -> Result<Vec<Instrument>, Error> {
        let path = format!("/instruments/{instrument_id}");
        match self.get::<Vec<Instrument>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /instruments/{instrument_id}/leverages` — Returns a list of
    /// leverage instruments that have the current instrument as
    /// underlying.
    ///
    /// Filters are passed as a [`LeveragesQuery`]; pass
    /// `&LeveragesQuery::default()` for "no filters".
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn list_leverages(
        &self,
        instrument_id: InstrumentId,
        filters: LeveragesQuery<'_>,
    ) -> Result<Vec<Instrument>, Error> {
        let qs = build_leverages_query(&filters);
        let path = if qs.is_empty() {
            format!("/instruments/{instrument_id}/leverages")
        } else {
            format!("/instruments/{instrument_id}/leverages?{qs}")
        };
        match self.get::<Vec<Instrument>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /instruments/{instrument_id}/leverages/filters` — Returns
    /// valid leverage instruments filter values for the given underlying.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn get_leverage_filters(
        &self,
        instrument_id: InstrumentId,
    ) -> Result<LeverageFilter, Error> {
        let path = format!("/instruments/{instrument_id}/leverages/filters");
        self.get::<LeverageFilter>(&path).await
    }

    /// `GET /instruments/{instrument_id}/trades` — Returns the public
    /// trades belonging to one or more instruments.
    ///
    /// Renamed from the docs op `list_trades` to `list_instrument_trades`
    /// to coexist with `tradables::list_tradable_trades` and the future
    /// `accounts::list_trades` on the same `Client` impl.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn list_instrument_trades(
        &self,
        instrument_id: InstrumentId,
    ) -> Result<Vec<InstrumentPublicTrades>, Error> {
        let path = format!("/instruments/{instrument_id}/trades");
        match self.get::<Vec<InstrumentPublicTrades>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_empty_when_no_filters() {
        let qs = build_leverages_query(&LeveragesQuery::default());
        assert_eq!(qs, "");
    }

    #[test]
    fn build_query_includes_all_filters_in_order() {
        let qs = build_leverages_query(&LeveragesQuery {
            currency: Some("SEK"),
            expiration_date: Some("2025-12-19"),
            instrument_group_type: Some("LEVERAGE"),
            instrument_type: Some("WNT"),
            issuer_id: Some(IssuerId(42)),
            market_view: Some("U"),
        });
        assert_eq!(
            qs,
            "currency=SEK&expiration_date=2025-12-19&instrument_group_type=LEVERAGE&instrument_type=WNT&issuer_id=42&market_view=U"
        );
    }

    #[test]
    fn build_query_percent_encodes_special_chars() {
        let qs = build_leverages_query(&LeveragesQuery {
            currency: Some("a&b"),
            ..LeveragesQuery::default()
        });
        assert_eq!(qs, "currency=a%26b");
    }
}
