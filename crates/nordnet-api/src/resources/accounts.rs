//! Resource methods for the `accounts` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|----|------|
//! | GET | `list_accounts` | `/accounts` |
//! | GET | `get_account_info` | `/accounts/{accid}/info` |
//! | GET | `list_ledgers` | `/accounts/{accid}/ledgers` |
//! | GET | `list_positions` | `/accounts/{accid}/positions` |
//! | GET | `get_returns_today` | `/accounts/{accid}/returns/transactions/today` |
//! | GET | `list_account_trades` | `/accounts/{accid}/trades` |
//!
//! ## Naming
//!
//! One op is renamed from its docs name so it can co-exist on [`Client`]
//! alongside same-named ops in other groups (Rust resolves all resource
//! methods onto a single `Client` impl):
//!
//! - `list_trades` -> `list_account_trades` — to coexist with
//!   [`Client::list_tradable_trades`] (`tradables` group) and
//!   [`Client::list_instrument_trades`] (`instruments` group). Mirrors the
//!   precedent set in `resources/instruments.rs`.
//!
//! Phase 3X may pick a uniform naming scheme.
//!
//! ## Path note
//!
//! `get_returns_today` uses path `/accounts/{accid}/returns/transactions/today`
//! per the Phase 1 docs extract. The Phase 3 task brief proposed
//! `/accounts/{accid}/returns/today` but the saved HTML schema is the
//! authoritative source per CONTRACTS.md priority #1.
//!
//! ## 204 No Content
//!
//! All ops except `get_account_info` document a 204 response. The base
//! [`Client::get`] surfaces an empty body as a [`Error::Decode`]; each
//! Vec-returning method here maps that case to an empty `Vec` (mirroring
//! the `instruments` precedent). `list_ledgers` returns a single
//! [`LedgerInformation`] object (not an array) — its 204 case is bubbled
//! up as the underlying decode error rather than fabricated, matching the
//! `instruments::get_leverage_filters` precedent for non-array returns.

use crate::client::Client;
use crate::error::Error;
use crate::ids::AccountId;
use crate::models::accounts::{
    Account, AccountInfo, AccountTransactionsToday, LedgerInformation, Position, Trade,
};

/// Optional query parameters for [`Client::list_accounts`].
#[derive(Debug, Clone, Default)]
pub struct ListAccountsQuery {
    /// `true` if credit accounts should be included in the response.
    /// Defaults to `false` server-side.
    pub include_credit_accounts: Option<bool>,
}

/// Optional query parameters for [`Client::get_account_info`].
#[derive(Debug, Clone, Default)]
pub struct AccountInfoQuery {
    /// `true` if `interest_rate` should be included in the response.
    /// Defaults to `true` server-side.
    pub include_interest_rate: Option<bool>,
    /// `true` if `short_position_margin` should be included in the
    /// response. Defaults to `true` server-side.
    pub include_short_pos_margin: Option<bool>,
}

/// Optional query parameters for [`Client::list_positions`].
#[derive(Debug, Clone, Default)]
pub struct ListPositionsQuery {
    /// `true` if instrument loan positions should be included.
    /// Defaults to `false` server-side.
    pub include_instrument_loans: Option<bool>,
    /// `true` if intraday limit should be included.
    /// Defaults to `false` server-side.
    pub include_intraday_limit: Option<bool>,
}

/// Optional query parameters for [`Client::get_returns_today`].
#[derive(Debug, Clone, Default)]
pub struct ReturnsTodayQuery {
    /// `true` if credit accounts should be included.
    /// Defaults to `true` server-side.
    pub include_credit_account: Option<bool>,
}

/// Optional query parameters for [`Client::list_account_trades`].
#[derive(Debug, Clone, Default)]
pub struct ListAccountTradesQuery {
    /// Number of days to look up trades for. Defaults to `0` (today only)
    /// server-side. Maximum is `7` per the docs.
    pub days: Option<i64>,
}

/// Build the encoded query string for the given pairs.
///
/// Uses `reqwest::Url::query_pairs_mut` so all percent-encoding follows
/// the standard URL form rules. The placeholder host is never sent
/// anywhere — only the encoded query suffix is extracted.
fn build_query(pairs: &[(&str, String)]) -> String {
    if pairs.is_empty() {
        return String::new();
    }
    let mut url = match reqwest::Url::parse("http://_/") {
        Ok(u) => u,
        // The literal above is a valid absolute URL — this branch is
        // unreachable in practice. Returning an empty string keeps the
        // function total without panicking.
        Err(_) => return String::new(),
    };
    {
        let mut qs = url.query_pairs_mut();
        for (k, v) in pairs {
            qs.append_pair(k, v);
        }
    }
    url.query().unwrap_or("").to_owned()
}

fn append_bool(pairs: &mut Vec<(&'static str, String)>, k: &'static str, v: Option<bool>) {
    if let Some(b) = v {
        pairs.push((k, b.to_string()));
    }
}

impl Client {
    /// `GET /accounts` — Returns a list of accounts to which the user has
    /// access.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::Unauthorized`] (401), [`Error::Forbidden`] (403),
    /// [`Error::TooManyRequests`] (429), [`Error::ServiceUnavailable`]
    /// (503).
    pub async fn list_accounts(&self, query: ListAccountsQuery) -> Result<Vec<Account>, Error> {
        let mut pairs = Vec::new();
        append_bool(
            &mut pairs,
            "include_credit_accounts",
            query.include_credit_accounts,
        );
        let qs = build_query(&pairs);
        let path = if qs.is_empty() {
            "/accounts".to_owned()
        } else {
            format!("/accounts?{qs}")
        };
        match self.get::<Vec<Account>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /accounts/{accid}/info` — Returns account information details
    /// for one or more accounts.
    ///
    /// `accid` is one or more account identifier(s). The single-account
    /// `accid: AccountId` parameter is the strongly-typed shape; multi-
    /// account lookups (the API accepts a comma-separated list in the
    /// path) are deferred to a higher-level helper.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn get_account_info(
        &self,
        accid: AccountId,
        query: AccountInfoQuery,
    ) -> Result<Vec<AccountInfo>, Error> {
        let mut pairs = Vec::new();
        append_bool(
            &mut pairs,
            "include_interest_rate",
            query.include_interest_rate,
        );
        append_bool(
            &mut pairs,
            "include_short_pos_margin",
            query.include_short_pos_margin,
        );
        let qs = build_query(&pairs);
        let path = if qs.is_empty() {
            format!("/accounts/{accid}/info")
        } else {
            format!("/accounts/{accid}/info?{qs}")
        };
        self.get::<Vec<AccountInfo>>(&path).await
    }

    /// `GET /accounts/{accid}/ledgers` — Returns information about the
    /// currency ledgers of an account.
    ///
    /// Note: 204 is documented but cannot be mapped to an empty default
    /// because [`LedgerInformation`] is a single object (not an array).
    /// In that case the underlying [`Error::Decode`] is returned.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn list_ledgers(&self, accid: AccountId) -> Result<LedgerInformation, Error> {
        let path = format!("/accounts/{accid}/ledgers");
        self.get::<LedgerInformation>(&path).await
    }

    /// `GET /accounts/{accid}/positions` — Returns all positions for the
    /// given account.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn list_positions(
        &self,
        accid: AccountId,
        query: ListPositionsQuery,
    ) -> Result<Vec<Position>, Error> {
        let mut pairs = Vec::new();
        append_bool(
            &mut pairs,
            "include_instrument_loans",
            query.include_instrument_loans,
        );
        append_bool(
            &mut pairs,
            "include_intraday_limit",
            query.include_intraday_limit,
        );
        let qs = build_query(&pairs);
        let path = if qs.is_empty() {
            format!("/accounts/{accid}/positions")
        } else {
            format!("/accounts/{accid}/positions?{qs}")
        };
        match self.get::<Vec<Position>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /accounts/{accid}/returns/transactions/today` — Returns
    /// today's withdrawal/deposit transaction amounts for the given
    /// account.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn get_returns_today(
        &self,
        accid: AccountId,
        query: ReturnsTodayQuery,
    ) -> Result<Vec<AccountTransactionsToday>, Error> {
        let mut pairs = Vec::new();
        append_bool(
            &mut pairs,
            "include_credit_account",
            query.include_credit_account,
        );
        let qs = build_query(&pairs);
        let path = if qs.is_empty() {
            format!("/accounts/{accid}/returns/transactions/today")
        } else {
            format!("/accounts/{accid}/returns/transactions/today?{qs}")
        };
        match self.get::<Vec<AccountTransactionsToday>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /accounts/{accid}/trades` — Returns all trades belonging to
    /// the given account.
    ///
    /// Renamed from the docs op `list_trades` to `list_account_trades` to
    /// coexist with [`Client::list_tradable_trades`] and
    /// [`Client::list_instrument_trades`] on the same `Client` impl.
    ///
    /// Returns an empty `Vec` on 204 No Content.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::NotFound`] (404; documented
    /// as "Account not found"; surfaced via [`Error::UnexpectedStatus`]
    /// because the foundation `Error` enum does not model 404 as a
    /// dedicated variant), [`Error::TooManyRequests`] (429),
    /// [`Error::ServiceUnavailable`] (503).
    pub async fn list_account_trades(
        &self,
        accid: AccountId,
        query: ListAccountTradesQuery,
    ) -> Result<Vec<Trade>, Error> {
        let mut pairs = Vec::new();
        if let Some(d) = query.days {
            pairs.push(("days", d.to_string()));
        }
        let qs = build_query(&pairs);
        let path = if qs.is_empty() {
            format!("/accounts/{accid}/trades")
        } else {
            format!("/accounts/{accid}/trades?{qs}")
        };
        match self.get::<Vec<Trade>>(&path).await {
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
    fn build_query_empty_when_no_pairs() {
        assert_eq!(build_query(&[]), "");
    }

    #[test]
    fn build_query_pairs_in_order_and_encoded() {
        let qs = build_query(&[
            ("days", "7".to_owned()),
            ("include_credit_accounts", "true".to_owned()),
            ("name", "a&b".to_owned()),
        ]);
        assert_eq!(qs, "days=7&include_credit_accounts=true&name=a%26b");
    }

    #[test]
    fn append_bool_skips_when_none() {
        let mut pairs = Vec::new();
        append_bool(&mut pairs, "include_credit_accounts", None);
        assert!(pairs.is_empty());
    }

    #[test]
    fn append_bool_emits_lowercase_when_some() {
        let mut pairs = Vec::new();
        append_bool(&mut pairs, "include_credit_accounts", Some(true));
        append_bool(&mut pairs, "include_intraday_limit", Some(false));
        assert_eq!(
            pairs,
            vec![
                ("include_credit_accounts", "true".to_owned()),
                ("include_intraday_limit", "false".to_owned()),
            ]
        );
    }
}
