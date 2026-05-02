//! Resource methods for the `instrument_search` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|----|------|
//! | GET | `get_attributes` | `/instrument_search/attributes` |
//! | GET | `search_stocklist` | `/instrument_search/query/stocklist` |
//! | GET | `search_bullbearlist` | `/instrument_search/query/bullbearlist` |
//! | GET | `search_minifuturelist` | `/instrument_search/query/minifuturelist` |
//! | GET | `search_unlimitedturbolist` | `/instrument_search/query/unlimitedturbolist` |
//! | GET | `search_optionlist_pairs` | `/instrument_search/query/optionlist/pairs` |
//!
//! ## Query parameters
//!
//! Each search op carries a sizeable set of optional query parameters. The
//! parameters are gathered into per-op `*Query` structs (with
//! `Default::default()` producing the documented "no filters" form) and
//! forwarded via [`reqwest::Url::query_pairs_mut`] for proper percent
//! encoding. Required params (only `search_optionlist_pairs` has any) are
//! plain method arguments.
//!
//! ## 204 No Content
//!
//! Only `search_bullbearlist` documents a 204 response. The base
//! [`Client::get`] surfaces an empty body as a [`Error::Decode`]; the
//! method here maps that case to an empty [`BullBearListResults`]. The
//! other ops do not document 204 and so do not perform the mapping.
//!
//! Vec-returning ops in this group: none — every op returns a wrapper
//! struct, so the "204 -> empty Vec" mirror used by the `instruments`
//! group does not apply directly. The bullbear case maps to an empty
//! results wrapper instead.

use crate::client::Client;
use crate::error::Error;
use crate::models::instrument_search::{
    AttributeResults, BullBearListResults, MinifutureListResults, OptionListResults,
    StocklistResults, UnlimitedTurboListResults,
};

// ---------------------------------------------------------------------------
// Query-builder helpers
// ---------------------------------------------------------------------------

/// Append a query string to a path, omitting the `?` when the query is
/// empty. Centralises the small piece of formatting shared by every search
/// op.
fn with_query(path: &str, qs: &str) -> String {
    if qs.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{qs}")
    }
}

/// Build the encoded query string from a list of `(name, optional value)`
/// pairs. `None` values are skipped. Multi-value fields are flattened into
/// repeated `name=value` pairs (mirrors how `reqwest` encodes a vec). All
/// values are percent-encoded by `reqwest::Url::query_pairs_mut`.
fn encode_pairs(pairs: &[(&str, Option<&str>)], multi: &[(&str, &[String])]) -> String {
    let mut url = match reqwest::Url::parse("http://_/") {
        Ok(u) => u,
        // The literal above is a valid absolute URL — this branch is
        // unreachable. Returning an empty string keeps the function total
        // without panicking.
        Err(_) => return String::new(),
    };
    {
        let mut q = url.query_pairs_mut();
        for (name, value) in pairs {
            if let Some(v) = value {
                q.append_pair(name, v);
            }
        }
        for (name, values) in multi {
            for v in *values {
                q.append_pair(name, v);
            }
        }
    }
    url.query().unwrap_or("").to_owned()
}

// ---------------------------------------------------------------------------
// get_attributes query
// ---------------------------------------------------------------------------

/// Optional query parameters for [`Client::get_attributes`].
///
/// Every field is documented as optional. Build via
/// `AttributesQuery::default()` and field-by-field assignment.
#[derive(Debug, Clone, Default)]
pub struct AttributesQuery<'a> {
    /// Specifies which filters to apply to the search.
    pub apply_filters: Option<&'a str>,
    /// Returns only attributes belonging to the specified attribute groups
    /// (free string per the doc — must be one of the documented enum
    /// values, e.g. `EXCHANGE_INFO`, `PRICE_INFO`).
    pub attribute_group: Vec<String>,
    /// Returns only attributes belonging to the specified entity type
    /// (free string per the doc — must be one of the documented enum
    /// values, e.g. `STOCKLIST`, `OPTIONLIST`).
    pub entity_type: Option<&'a str>,
    /// Expand attribute values only for the listed attributes. The default
    /// expand value is `all`.
    pub expand: Vec<String>,
    /// Returns minimum and maximum values for the specified attributes.
    pub minmax: Vec<String>,
    /// Returns only filterable attributes.
    pub only_filterable: Option<bool>,
    /// Returns only returnable attributes.
    pub only_returnable: Option<bool>,
    /// Returns only sortable attributes.
    pub only_sortable: Option<bool>,
}

fn build_attributes_query(q: &AttributesQuery<'_>) -> String {
    let only_filterable = q.only_filterable.map(bool_str);
    let only_returnable = q.only_returnable.map(bool_str);
    let only_sortable = q.only_sortable.map(bool_str);
    encode_pairs(
        &[
            ("apply_filters", q.apply_filters),
            ("entity_type", q.entity_type),
            ("only_filterable", only_filterable),
            ("only_returnable", only_returnable),
            ("only_sortable", only_sortable),
        ],
        &[
            ("attribute_group", &q.attribute_group),
            ("expand", &q.expand),
            ("minmax", &q.minmax),
        ],
    )
}

fn bool_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

// ---------------------------------------------------------------------------
// search_stocklist query
// ---------------------------------------------------------------------------

/// Optional query parameters for [`Client::search_stocklist`].
///
/// Every field is documented as optional. Defaults match the documented
/// API defaults (`limit=50`, `offset=0`, `sort_attribute="name"`,
/// `sort_order="asc"`); pass `None` to omit a parameter and let the
/// server apply its default.
#[derive(Debug, Clone, Default)]
pub struct StocklistQuery<'a> {
    /// Defines which filters to apply to the search.
    pub apply_filters: Option<&'a str>,
    /// Returns only attributes for the given attribute groups.
    pub attribute_groups: Vec<String>,
    /// Returns only the given attributes.
    pub attributes: Vec<String>,
    /// Free-text search string (instrument name, symbol, or ISIN).
    pub free_text_search: Option<&'a str>,
    /// Limits the search results to `limit`.
    pub limit: Option<i32>,
    /// Skips the first `offset` search results per group.
    pub offset: Option<i32>,
    /// Defines the attribute to sort by (default `name`).
    pub sort_attribute: Option<&'a str>,
    /// Defines the sort order (`asc` or `desc`; default `asc`).
    pub sort_order: Option<&'a str>,
}

fn build_stocklist_query(q: &StocklistQuery<'_>) -> String {
    let limit = q.limit.map(|v| v.to_string());
    let offset = q.offset.map(|v| v.to_string());
    encode_pairs(
        &[
            ("apply_filters", q.apply_filters),
            ("free_text_search", q.free_text_search),
            ("limit", limit.as_deref()),
            ("offset", offset.as_deref()),
            ("sort_attribute", q.sort_attribute),
            ("sort_order", q.sort_order),
        ],
        &[
            ("attribute_groups", &q.attribute_groups),
            ("attributes", &q.attributes),
        ],
    )
}

// ---------------------------------------------------------------------------
// Bull/Bear, Mini-future, Unlimited-turbo shared list-style query
// ---------------------------------------------------------------------------

/// Optional query parameters shared by [`Client::search_bullbearlist`],
/// [`Client::search_minifuturelist`], and
/// [`Client::search_unlimitedturbolist`].
///
/// All three endpoints document the same parameter set (apply_filters,
/// free_text_search, limit, offset, sort_attribute, sort_order). One
/// query type is used for all three to avoid trivial duplication.
#[derive(Debug, Clone, Default)]
pub struct ListSearchQuery<'a> {
    /// Specifies which filters to apply to the search.
    pub apply_filters: Option<&'a str>,
    /// Free text search for name, symbol and ISIN.
    pub free_text_search: Option<&'a str>,
    /// Limits the search results to `limit` instruments.
    pub limit: Option<i32>,
    /// Skips the first `offset` search results.
    pub offset: Option<i32>,
    /// Defines the attribute to sort by. (`bullbearlist` defaults to
    /// `name`; the other two endpoints have no documented default.)
    pub sort_attribute: Option<&'a str>,
    /// Defines the sort order (`asc` or `desc`).
    pub sort_order: Option<&'a str>,
}

fn build_list_search_query(q: &ListSearchQuery<'_>) -> String {
    let limit = q.limit.map(|v| v.to_string());
    let offset = q.offset.map(|v| v.to_string());
    encode_pairs(
        &[
            ("apply_filters", q.apply_filters),
            ("free_text_search", q.free_text_search),
            ("limit", limit.as_deref()),
            ("offset", offset.as_deref()),
            ("sort_attribute", q.sort_attribute),
            ("sort_order", q.sort_order),
        ],
        &[],
    )
}

// ---------------------------------------------------------------------------
// Resource methods
// ---------------------------------------------------------------------------

impl Client {
    /// `GET /instrument_search/attributes` — Search for attributes
    /// available in the instrument search APIs.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn get_attributes(
        &self,
        filters: AttributesQuery<'_>,
    ) -> Result<AttributeResults, Error> {
        let qs = build_attributes_query(&filters);
        let path = with_query("/instrument_search/attributes", &qs);
        self.get::<AttributeResults>(&path).await
    }

    /// `GET /instrument_search/query/stocklist` — Search for stocks.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn search_stocklist(
        &self,
        filters: StocklistQuery<'_>,
    ) -> Result<StocklistResults, Error> {
        let qs = build_stocklist_query(&filters);
        let path = with_query("/instrument_search/query/stocklist", &qs);
        self.get::<StocklistResults>(&path).await
    }

    /// `GET /instrument_search/query/bullbearlist` — Search, filter and
    /// sort instruments within the Bull & Bear entity type.
    ///
    /// 204 No Content is documented for this op; it is mapped to an empty
    /// [`BullBearListResults`] (every field defaulted to `None`).
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn search_bullbearlist(
        &self,
        filters: ListSearchQuery<'_>,
    ) -> Result<BullBearListResults, Error> {
        let qs = build_list_search_query(&filters);
        let path = with_query("/instrument_search/query/bullbearlist", &qs);
        match self.get::<BullBearListResults>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => {
                Ok(BullBearListResults {
                    results: None,
                    rows: None,
                    total_hits: None,
                    underlying_instrument_id: None,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// `GET /instrument_search/query/minifuturelist` — Search, filter and
    /// sort instruments within the Mini Future entity type.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn search_minifuturelist(
        &self,
        filters: ListSearchQuery<'_>,
    ) -> Result<MinifutureListResults, Error> {
        let qs = build_list_search_query(&filters);
        let path = with_query("/instrument_search/query/minifuturelist", &qs);
        self.get::<MinifutureListResults>(&path).await
    }

    /// `GET /instrument_search/query/unlimitedturbolist` — Search, filter
    /// and sort instruments within the Unlimited Turbo entity type.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn search_unlimitedturbolist(
        &self,
        filters: ListSearchQuery<'_>,
    ) -> Result<UnlimitedTurboListResults, Error> {
        let qs = build_list_search_query(&filters);
        let path = with_query("/instrument_search/query/unlimitedturbolist", &qs);
        self.get::<UnlimitedTurboListResults>(&path).await
    }

    /// `GET /instrument_search/query/optionlist/pairs` — Search for the
    /// Option Pair (Put-Call) given an underlying instrument and the
    /// expiration date.
    ///
    /// All three parameters are required per the doc:
    /// - `currency`: option currency (e.g. `"SEK"`).
    /// - `expire_date`: expiration date as a Nordnet UNIX-millis epoch
    ///   timestamp (`integer(int64)` per the doc).
    /// - `underlying_symbol`: underlying instrument symbol (e.g. `"ERIC B"`).
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn search_optionlist_pairs(
        &self,
        currency: &str,
        expire_date: i64,
        underlying_symbol: &str,
    ) -> Result<OptionListResults, Error> {
        let expire_date = expire_date.to_string();
        let qs = encode_pairs(
            &[
                ("currency", Some(currency)),
                ("expire_date", Some(&expire_date)),
                ("underlying_symbol", Some(underlying_symbol)),
            ],
            &[],
        );
        let path = with_query("/instrument_search/query/optionlist/pairs", &qs);
        self.get::<OptionListResults>(&path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_query_empty_omits_question_mark() {
        assert_eq!(with_query("/x", ""), "/x");
        assert_eq!(with_query("/x", "a=1"), "/x?a=1");
    }

    #[test]
    fn attributes_query_empty_when_default() {
        let qs = build_attributes_query(&AttributesQuery::default());
        assert_eq!(qs, "");
    }

    #[test]
    fn attributes_query_includes_scalar_and_repeated() {
        let qs = build_attributes_query(&AttributesQuery {
            apply_filters: Some("nordnet_markets=true"),
            attribute_group: vec!["PRICE_INFO".to_owned(), "EXCHANGE_INFO".to_owned()],
            entity_type: Some("STOCKLIST"),
            expand: vec!["market_id".to_owned()],
            minmax: vec![],
            only_filterable: Some(true),
            only_returnable: Some(false),
            only_sortable: None,
        });
        assert_eq!(
            qs,
            "apply_filters=nordnet_markets%3Dtrue&entity_type=STOCKLIST&only_filterable=true&only_returnable=false&attribute_group=PRICE_INFO&attribute_group=EXCHANGE_INFO&expand=market_id"
        );
    }

    #[test]
    fn stocklist_query_default_empty() {
        let qs = build_stocklist_query(&StocklistQuery::default());
        assert_eq!(qs, "");
    }

    #[test]
    fn stocklist_query_with_all_fields() {
        let qs = build_stocklist_query(&StocklistQuery {
            apply_filters: Some("instrument_type=ESH"),
            attribute_groups: vec!["PRICE_INFO".to_owned()],
            attributes: vec!["name".to_owned(), "isin".to_owned()],
            free_text_search: Some("erics"),
            limit: Some(25),
            offset: Some(50),
            sort_attribute: Some("name"),
            sort_order: Some("desc"),
        });
        assert_eq!(
            qs,
            "apply_filters=instrument_type%3DESH&free_text_search=erics&limit=25&offset=50&sort_attribute=name&sort_order=desc&attribute_groups=PRICE_INFO&attributes=name&attributes=isin"
        );
    }

    #[test]
    fn list_search_query_default_empty() {
        let qs = build_list_search_query(&ListSearchQuery::default());
        assert_eq!(qs, "");
    }

    #[test]
    fn list_search_query_includes_pagination() {
        let qs = build_list_search_query(&ListSearchQuery {
            apply_filters: None,
            free_text_search: Some("ERIC"),
            limit: Some(10),
            offset: Some(0),
            sort_attribute: None,
            sort_order: Some("asc"),
        });
        assert_eq!(qs, "free_text_search=ERIC&limit=10&offset=0&sort_order=asc");
    }
}
