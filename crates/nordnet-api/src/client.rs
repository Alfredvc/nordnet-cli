//! HTTP client for the Nordnet External API.
//!
//! Wraps `reqwest::Client` with:
//!   - Base URL composition (`{base}/{path}` with leading-slash tolerated).
//!   - `Authorization: Basic <session_key:session_key>` injection when a
//!     [`Session`] is attached.
//!   - Single response-parsing path so every method routes identical
//!     status-code handling.
//!
//! Non-2xx responses (including 429 Too Many Requests and 503 Service
//! Unavailable) surface to the caller as the matching [`Error`] variant.
//! Retry policy is deliberately a caller concern — the library does not
//! sleep, retry, or hide latency. POST/PUT operations on `/orders` are
//! non-idempotent; a hidden retry could double-place an order if a
//! response is lost in flight. Callers that want backoff should wrap
//! these methods explicitly.
//!
//! No method here calls a Nordnet host directly — callers supply the base
//! URL (production, test, or a `wiremock::MockServer`).

use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Method, Response,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::auth::Session;
use crate::error::Error;

/// Typed HTTP client for the Nordnet API. Cheap to clone — wraps a
/// `reqwest::Client` and a base URL + optional session.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    session: Option<Session>,
}

impl Client {
    /// Build a client for the given base URL (e.g.
    /// `https://public.nordnet.se/api/2`). The base URL is used verbatim;
    /// trailing slashes are stripped.
    pub fn new(base_url: impl Into<String>) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(Error::Transport)?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            session: None,
        })
    }

    /// Attach (or replace) the authenticated session used for the
    /// `Authorization` header on subsequent requests.
    pub fn with_session(mut self, session: Session) -> Self {
        self.session = Some(session);
        self
    }

    /// Active session, if any. Mostly useful for tests.
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Base URL the client targets.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// GET `<base_url><path>` and parse the JSON body as `T`.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.send::<T, ()>(Method::GET, path, None).await
    }

    /// POST a JSON body to `<base_url><path>` and parse the JSON response.
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.send(Method::POST, path, Some(Body::Json(body))).await
    }

    /// PUT a JSON body to `<base_url><path>` and parse the JSON response.
    pub async fn put<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.send(Method::PUT, path, Some(Body::Json(body))).await
    }

    /// POST a body to `<base_url><path>` encoded as
    /// `application/x-www-form-urlencoded`, and parse the JSON response.
    ///
    /// Required for endpoints whose Swagger 2.0 parameter table marks every
    /// body parameter as `FormData` (e.g. `POST /accounts/{accid}/orders`).
    /// JSON bodies are silently rejected by these endpoints.
    pub async fn post_form<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.send(Method::POST, path, Some(Body::Form(body))).await
    }

    /// PUT a body to `<base_url><path>` encoded as
    /// `application/x-www-form-urlencoded`, and parse the JSON response.
    ///
    /// Required for endpoints whose Swagger 2.0 parameter table marks every
    /// body parameter as `FormData` (e.g. `PUT /accounts/{accid}/orders/{order_id}`).
    pub async fn put_form<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.send(Method::PUT, path, Some(Body::Form(body))).await
    }

    /// PUT `<base_url><path>` with no request body. The wire request omits
    /// the `Content-Type` header and sends a zero-length body — this is the
    /// shape Nordnet's `PUT /login` (refresh session) expects.
    pub async fn put_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.send::<T, ()>(Method::PUT, path, None).await
    }

    /// DELETE `<base_url><path>` and parse the JSON response.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.send::<T, ()>(Method::DELETE, path, None).await
    }

    /// Compose the full URL for `path`. Public so tests and resource
    /// modules can build requests without re-implementing the rule.
    pub fn url(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }

    fn auth_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        if let Some(session) = &self.session {
            let value = session.basic_auth_header();
            let header =
                HeaderValue::from_str(&value).map_err(|e| Error::InvalidHeader(e.to_string()))?;
            headers.insert(AUTHORIZATION, header);
        }
        Ok(headers)
    }

    async fn send<T: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<Body<'_, B>>,
    ) -> Result<T, Error> {
        let url = self.url(path);
        let headers = self.auth_headers()?;
        let response = self.execute_once(method, &url, headers, body).await?;
        parse_response::<T>(response).await
    }

    async fn execute_once<B: Serialize>(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Option<Body<'_, B>>,
    ) -> Result<Response, Error> {
        let mut req = self.http.request(method, url).headers(headers);
        match body {
            Some(Body::Json(b)) => {
                req = req.header(CONTENT_TYPE, "application/json").json(b);
            }
            Some(Body::Form(b)) => {
                // Encode via serde_urlencoded directly (rather than
                // `RequestBuilder::form`, which is gated on a reqwest
                // feature we don't enable). Wire format is identical.
                let encoded =
                    serde_urlencoded::to_string(b).map_err(|e| Error::EncodeForm(e.to_string()))?;
                req = req
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(encoded);
            }
            None => {}
        }
        req.send().await.map_err(Error::Transport)
    }
}

/// Internal body wrapper used to thread the encoding choice from the public
/// helper (`post`, `put`, `post_form`, `put_form`) down to `execute_once`.
/// Kept private — callers only see the typed helpers.
enum Body<'a, B: Serialize> {
    Json(&'a B),
    Form(&'a B),
}

/// Single response-parsing path used by every method on [`Client`].
async fn parse_response<T: DeserializeOwned>(response: Response) -> Result<T, Error> {
    let status = response.status();
    let body = response.text().await.map_err(Error::Transport)?;

    if status.is_success() {
        serde_json::from_str::<T>(&body).map_err(|source| Error::Decode { source, body })
    } else {
        Err(Error::from_status(status.as_u16(), body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_handles_leading_slash() {
        let c = Client::new("http://example.com/api/2").unwrap();
        assert_eq!(c.url("/accounts"), "http://example.com/api/2/accounts");
        assert_eq!(c.url("accounts"), "http://example.com/api/2/accounts");
    }

    #[test]
    fn url_strips_trailing_slash_on_base() {
        let c = Client::new("http://example.com/api/2/").unwrap();
        assert_eq!(c.url("/x"), "http://example.com/api/2/x");
    }

    #[test]
    fn no_session_no_auth_header() {
        let c = Client::new("http://x").unwrap();
        let h = c.auth_headers().unwrap();
        assert!(!h.contains_key(AUTHORIZATION));
    }

    #[test]
    fn with_session_sets_basic_auth() {
        let c = Client::new("http://x").unwrap().with_session(Session {
            session_key: "abc".into(),
            expires_in: 60,
        });
        let h = c.auth_headers().unwrap();
        assert_eq!(
            h.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Basic YWJjOmFiYw=="
        );
    }
}
