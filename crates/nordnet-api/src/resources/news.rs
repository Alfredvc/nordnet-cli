//! Resource methods for the `news` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|-----|------|
//! | GET | `get_news_item` | `/news/{item_id}` |
//! | GET | `list_news_sources` | `/news_sources` |
//!
//! ## Multi-ID lookups for `get_news_item`
//!
//! The Nordnet API path parameter for `get_news_item` accepts a
//! comma-separated list of news article IDs. This method covers the
//! single-ID case via the [`NewsId`] newtype, mirroring the
//! [`Client::get_market`] pattern. A future helper may be added for
//! multi-ID lookups when needed (Phase 3X).
//!
//! ## 204 No Content
//!
//! `GET /news/{item_id}` may return HTTP 204 (No Content) when no matching
//! article is found. The base [`Client::get`] method attempts to parse the
//! empty response body as JSON, which fails. `get_news_item` detects this
//! specific case (a [`Error::Decode`] with an empty body) and maps it to
//! an empty `Vec`, mirroring [`Client::get_country`] / [`Client::get_market`].
//!
//! ## `body_format` query parameter
//!
//! The `body_format` query parameter is documented as deprecated with no
//! effect on the response. It is intentionally NOT exposed on this method.

use crate::client::Client;
use crate::error::Error;
use nordnet_model::models::news::{NewsArticle, NewsId, NewsSource};

impl Client {
    /// `GET /news/{item_id}` — Retrieve a news article by ID.
    ///
    /// Returns an empty `Vec` when the API responds with 204 No Content
    /// (no matching article).
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) as documented.
    pub async fn get_news_item(&self, id: NewsId) -> Result<Vec<NewsArticle>, Error> {
        let path = format!("/news/{}", id.0);
        match self.get::<Vec<NewsArticle>>(&path).await {
            Ok(v) => Ok(v),
            // HTTP 204 No Content: the base client sees an empty body and
            // produces a Decode error. Map it to an empty Vec instead.
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /news_sources` — Returns the news sources the user has access to.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthorized`] (401), [`Error::TooManyRequests`]
    /// (429), or [`Error::ServiceUnavailable`] (503) as documented.
    pub async fn list_news_sources(&self) -> Result<Vec<NewsSource>, Error> {
        self.get("/news_sources").await
    }
}
