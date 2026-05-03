//! Resource methods for the `main_search` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|----|------|
//! | GET | `search` | `/main_search` |
//!
//!
//! ## Query parameters
//!
//! `GET /main_search` takes one required and four optional query params:
//!
//! | Param | Form | Notes |
//! |-------|------|-------|
//! | `query` | string, required | Search string. |
//! | `instrument_group` | enum, multi | Repeats once per value (`?instrument_group=EQUITY&instrument_group=ETF`). |
//! | `limit` | int32, default 5 | |
//! | `offset` | int32, default 0 | |
//! | `search_space` | enum, default ALL | |
//!
//! `instrument_group` and `search_space` enum values are passed through
//! as `&str` for now (pragmatic; documented in the task contract). A
//! later phase may introduce typed enums.
//!
//!
//! ## 204 / 404 No Content
//!
//! Both 204 and 404 responses carry no body. The base [`Client::get`]
//! treats those as a [`Error::Decode`] over an empty body; this method
//! maps them to an empty `Vec<MainSearchResponse>`, mirroring the
//! `get_country` pattern.

use crate::client::Client;
use crate::error::Error;
use nordnet_model::models::main_search::MainSearchResponse;

/// Build the encoded query string for [`Client::search`].
///
/// Uses `reqwest::Url::query_pairs_mut` so all percent-encoding follows
/// the standard URL form rules (matching what the Nordnet API expects
/// from a browser-typed request). The placeholder host is never sent
/// anywhere — only the encoded query suffix is extracted.
fn build_search_query(
    query: &str,
    instrument_group: Option<&[&str]>,
    limit: Option<i32>,
    offset: Option<i32>,
    search_space: Option<&str>,
) -> String {
    // Placeholder host; only the query string is extracted.
    let mut url = match reqwest::Url::parse("http://_/") {
        Ok(u) => u,
        // The literal above is a valid absolute URL — this branch is
        // unreachable. Returning a bare `?query=...` here keeps the
        // function total without panicking.
        Err(_) => return format!("query={}", urlencoding_minimal(query)),
    };
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("query", query);
        if let Some(groups) = instrument_group {
            for g in groups {
                pairs.append_pair("instrument_group", g);
            }
        }
        if let Some(l) = limit {
            pairs.append_pair("limit", &l.to_string());
        }
        if let Some(o) = offset {
            pairs.append_pair("offset", &o.to_string());
        }
        if let Some(s) = search_space {
            pairs.append_pair("search_space", s);
        }
    }
    url.query().unwrap_or("").to_owned()
}

/// Minimal fallback percent-encoder used only on the unreachable URL
/// parse-failure path inside [`build_search_query`]. Encodes anything
/// outside the unreserved set (`A-Z`, `a-z`, `0-9`, `-_.~`) as `%HH`.
fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

impl Client {
    /// `GET /main_search` — Returns the instruments, news, and pages
    /// matching the given search criteria.
    ///
    /// # Parameters
    ///
    /// - `query` — required search string.
    /// - `instrument_group` — optional list of instrument groups to
    ///   restrict the search to (`EQUITY`, `PINV`, `FUND`, `ETF`,
    ///   `ETC`, `WARRANT`, `DERIVATIVES`, `INDICATOR`, `OTHER`). When
    ///   `None`, results from every group are returned. Encoded as
    ///   repeated `instrument_group=...` pairs.
    /// - `limit` — optional per-group result limit (default 5).
    /// - `offset` — optional per-group result offset (default 0).
    /// - `search_space` — optional search space (`ALL`, `INSTRUMENTS`,
    ///   `NEWS`, `CMS`, `BLOG`, `INSTRUMENTS_NEWS`, `INSTRUMENTS_CMS`,
    ///   `NEWS_CMS`, `NEWS_BLOG`, `NEWS_BLOG_CMS`). Default `ALL`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`]
    /// (401), [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) as documented.
    ///
    /// HTTP 204 (No Content) and 404 (No Content) responses are mapped
    /// to an empty `Vec<MainSearchResponse>` (per the `get_country`
    /// precedent).
    #[doc(alias = "GET /main_search")]
    pub async fn search(
        &self,
        query: &str,
        instrument_group: Option<&[&str]>,
        limit: Option<i32>,
        offset: Option<i32>,
        search_space: Option<&str>,
    ) -> Result<Vec<MainSearchResponse>, Error> {
        let qs = build_search_query(query, instrument_group, limit, offset, search_space);
        let path = format!("/main_search?{qs}");
        match self.get::<Vec<MainSearchResponse>>(&path).await {
            Ok(v) => Ok(v),
            // 204 / 404 No Content — base client surfaces this as a
            // Decode error over an empty body. Map it to an empty Vec.
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_required_only() {
        let qs = build_search_query("Volvo", None, None, None, None);
        assert_eq!(qs, "query=Volvo");
    }

    #[test]
    fn build_query_all_params() {
        let qs = build_search_query(
            "Volvo B",
            Some(&["EQUITY", "ETF"]),
            Some(10),
            Some(5),
            Some("INSTRUMENTS"),
        );
        // `application/x-www-form-urlencoded` encodes spaces as `+`.
        assert_eq!(
            qs,
            "query=Volvo+B&instrument_group=EQUITY&instrument_group=ETF&limit=10&offset=5&search_space=INSTRUMENTS"
        );
    }

    #[test]
    fn build_query_percent_encodes_special_chars() {
        let qs = build_search_query("a&b=c", None, None, None, None);
        // `&` and `=` must be percent-encoded so they don't fracture the
        // query string. `reqwest::Url` uses form-url-encoding rules.
        assert_eq!(qs, "query=a%26b%3Dc");
    }

    #[test]
    fn build_query_empty_instrument_group_omits_param() {
        let qs = build_search_query("x", Some(&[]), None, None, None);
        assert_eq!(qs, "query=x");
    }
}
