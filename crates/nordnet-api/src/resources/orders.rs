//! Resource methods for the `orders` API group.
//! # Operations
//! | Method | Op | Path |
//! |--------|----|------|
//! | GET | `list_orders` | `/accounts/{accid}/orders` |
//! | POST | `place_order` | `/accounts/{accid}/orders` |
//! | PUT | `modify_order` | `/accounts/{accid}/orders/{order_id}` |
//! | PUT | `activate_order` | `/accounts/{accid}/orders/{order_id}/activate` |
//! | DELETE | `cancel_order` | `/accounts/{accid}/orders/{order_id}` |
//!
//! ## 204 No Content (`list_orders`)
//! `GET /accounts/{accid}/orders` is documented to return 204 with no
//! body when there are no orders. The base [`Client::get`] surfaces an
//! empty body as [`Error::Decode`]; [`Client::list_orders`] maps that
//! specific case to an empty `Vec`, mirroring the
//! [`Client::get_tradable_info`] precedent.
//!
//! ## Body-less PUT (`activate_order`)
//! `activate_order` has no documented request body — we use
//! [`Client::put_empty`] so the wire request omits `Content-Type` and
//! sends a zero-length payload (precedent: `login::refresh_session`).
//!
//! ## Multi-account / multi-order paths
//! The Nordnet API path slots accept comma-separated lists of IDs (e.g.
//! `/accounts/1,2,3/orders`). The typed surface here stays single-id by
//! default — Phase 4 (or callers) can build comma lists into a `String`
//! and supply it via a future helper if needed.

use crate::client::Client;
use crate::error::Error;
use nordnet_model::ids::{AccountId, OrderId};
use nordnet_model::models::orders::{ModifyOrderRequest, Order, OrderReply, PlaceOrderRequest};

impl Client {
    /// `GET /accounts/{accid}/orders` — Returns all orders belonging to
    /// the given account.
    ///
    /// # Parameters
    ///
    /// - `accid` — the account identifier.
    /// - `deleted` — optional. When `Some(true)`, the response includes
    ///   orders that were deleted today. Defaults to `false` server-side.
    ///
    /// Returns an empty `Vec` when the API responds with 204 No Content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403; documented with empty body),
    /// [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) per the docs.
    pub async fn list_orders(
        &self,
        accid: AccountId,
        deleted: Option<bool>,
    ) -> Result<Vec<Order>, Error> {
        let path = match deleted {
            Some(d) => format!("/accounts/{accid}/orders?deleted={d}"),
            None => format!("/accounts/{accid}/orders"),
        };
        match self.get::<Vec<Order>>(&path).await {
            Ok(v) => Ok(v),
            // 204 No Content — base client surfaces this as a Decode error
            // over an empty body. Map it to an empty Vec.
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `POST /accounts/{accid}/orders` — Enters a new order for the
    /// tradable identified by the given market ID + tradable ID.
    ///
    /// The Nordnet docs mark every body parameter as Swagger 2.0
    /// `FormData`, so the request is sent as
    /// `application/x-www-form-urlencoded` via
    /// [`Client::post_form`]. JSON bodies are silently rejected by the
    /// live endpoint. See `` §"Locked decisions" item 9.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) per the docs.
    pub async fn place_order(
        &self,
        accid: AccountId,
        request: &PlaceOrderRequest,
    ) -> Result<OrderReply, Error> {
        let path = format!("/accounts/{accid}/orders");
        self.post_form(&path, request).await
    }

    /// `PUT /accounts/{accid}/orders/{order_id}` — Modifies the price
    /// and/or volume of an order.
    ///
    /// The Nordnet docs mark every body parameter as Swagger 2.0
    /// `FormData`, so the request is sent as
    /// `application/x-www-form-urlencoded` via [`Client::put_form`].
    /// See `` §"Locked decisions" item 9.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403),
    /// [`Error::UnexpectedStatus`] (404; documented "Order not found"),
    /// [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) per the docs.
    pub async fn modify_order(
        &self,
        accid: AccountId,
        order_id: OrderId,
        request: &ModifyOrderRequest,
    ) -> Result<OrderReply, Error> {
        let path = format!("/accounts/{accid}/orders/{order_id}");
        self.put_form(&path, request).await
    }

    /// `PUT /accounts/{accid}/orders/{order_id}/activate` — Activates
    /// an inactive order. Sends a body-less `PUT` per the docs.
    ///
    /// The doc parameter table notes that `order_id` accepts a
    /// comma-separated list and the response is therefore an array of
    /// [`OrderReply`]. The single-id call still receives an array (of
    /// length one).
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) per the docs.
    pub async fn activate_order(
        &self,
        accid: AccountId,
        order_id: OrderId,
    ) -> Result<Vec<OrderReply>, Error> {
        let path = format!("/accounts/{accid}/orders/{order_id}/activate");
        self.put_empty(&path).await
    }

    /// `DELETE /accounts/{accid}/orders/{order_id}` — Deletes an order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403), [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) per the docs.
    pub async fn cancel_order(
        &self,
        accid: AccountId,
        order_id: OrderId,
    ) -> Result<OrderReply, Error> {
        let path = format!("/accounts/{accid}/orders/{order_id}");
        self.delete(&path).await
    }
}
