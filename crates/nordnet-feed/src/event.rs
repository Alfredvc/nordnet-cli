//! Feed event envelopes.
//!
//! Both feeds share the same wire envelope `{"type":"<kind>","data":{...}}`
//! but the `data` payload schema differs per feed kind (public's `trade`
//! is a market trade, private's `trade` is an own-account fill). To keep
//! deserialization unambiguous, each feed has its own event enum.

use serde::Deserialize;
use serde_json::Value;

use crate::error::ServerError;
use crate::private;
use crate::public;

/// Inbound event on the public market-data feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicEvent {
    /// Server-to-client keep-alive (every 5s when idle). Empty payload;
    /// any extra fields in `data` are forward-compat-ignored.
    Heartbeat,
    /// Server-side error for a previous command. The connection is
    /// still alive — caller decides whether to reconnect/abort.
    Error(ServerError),
    Price(public::Price),
    Depth(public::Depth),
    Trade(public::Trade),
    TradingStatus(public::TradingStatus),
    Indicator(public::Indicator),
    News(public::News),
    /// Unknown wire `type`. Forward-compat: future event kinds — and
    /// malformed `err` frames missing the required `msg` field — land
    /// here without erroring out.
    Unknown {
        kind: String,
        data: Value,
    },
    /// A known event `type` whose payload failed to deserialize into the
    /// typed struct. Carries the raw payload plus the rendered serde
    /// error so consumers can log and continue.
    ///
    /// This is a non-terminal soft-fail: the connection stays open and
    /// the next [`crate::PublicFeedClient::recv`] call will return the
    /// next frame. Compare with [`crate::FeedError::Decode`] which
    /// signals a fundamentally broken envelope and is terminal.
    DecodeFailed {
        kind: String,
        data: Value,
        error: String,
    },
}

/// Wire envelope for either feed. Used during initial decode.
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub data: Value,
}

/// Try to extract a [`ServerError`] from a raw `serde_json::Value`.
///
/// Requires `data` to be an object with a string `msg` field. On
/// anything else, returns the original `data` back so the caller can
/// route to `Unknown { kind: "err", data }` — preserves diagnostic
/// signal instead of silently producing `ServerError { msg: "", cmd: Null }`.
pub(crate) fn parse_server_error(data: Value) -> Result<ServerError, Value> {
    let Some(obj) = data.as_object() else {
        return Err(data);
    };
    let Some(msg) = obj.get("msg").and_then(|v| v.as_str()).map(str::to_string) else {
        return Err(data);
    };
    let cmd = obj.get("cmd").cloned().unwrap_or(Value::Null);
    Ok(ServerError { msg, cmd })
}

/// Deserialize a typed payload from a raw value, building a
/// [`PublicEvent::DecodeFailed`] on failure rather than aborting the
/// connection. The caller hands a constructor that wraps the typed
/// payload in the matching event variant.
fn decode_or_soft_fail<T, F>(kind: &str, data: Value, wrap: F) -> PublicEvent
where
    T: serde::de::DeserializeOwned,
    F: FnOnce(T) -> PublicEvent,
{
    match serde_json::from_value::<T>(data.clone()) {
        Ok(payload) => wrap(payload),
        Err(e) => PublicEvent::DecodeFailed {
            kind: kind.to_owned(),
            data,
            error: e.to_string(),
        },
    }
}

impl PublicEvent {
    /// Decode one wire-line into a typed event. Unknown `type` values
    /// land in [`PublicEvent::Unknown`]; payload-shape mismatches inside
    /// known types land in [`PublicEvent::DecodeFailed`] (non-terminal).
    /// Unknown fields inside known payloads are silently ignored
    /// (forward compat).
    pub(crate) fn from_envelope(env: Envelope) -> Self {
        match env.kind.as_str() {
            "heartbeat" => PublicEvent::Heartbeat,
            "err" => match parse_server_error(env.data) {
                Ok(se) => PublicEvent::Error(se),
                Err(data) => PublicEvent::Unknown {
                    kind: env.kind,
                    data,
                },
            },
            "price" => decode_or_soft_fail(&env.kind, env.data, PublicEvent::Price),
            "depth" => decode_or_soft_fail(&env.kind, env.data, PublicEvent::Depth),
            "trade" => decode_or_soft_fail(&env.kind, env.data, PublicEvent::Trade),
            "trading_status" => {
                decode_or_soft_fail(&env.kind, env.data, PublicEvent::TradingStatus)
            }
            "indicator" => decode_or_soft_fail(&env.kind, env.data, PublicEvent::Indicator),
            "news" => decode_or_soft_fail(&env.kind, env.data, PublicEvent::News),
            _ => PublicEvent::Unknown {
                kind: env.kind,
                data: env.data,
            },
        }
    }
}

/// Inbound event on the private account/order feed.
///
/// The private feed is auto-pushed after [`crate::PrivateFeedClient::login`] —
/// no subscription is required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateEvent {
    /// Server-to-client keep-alive (every 5s when idle).
    Heartbeat,
    /// Server-side error. Connection still alive.
    Error(ServerError),
    /// Order lifecycle event (created / modified / filled / cancelled).
    ///
    /// Boxed to reduce the enum's overall stack footprint
    /// (`large_enum_variant` lint — `OrderEvent` is ~320 bytes).
    Order(Box<private::OrderEvent>),
    /// Untyped trade payload — schema is not in the public Nordnet
    /// docs (Decision §12). Future revisions may type this; the `Raw`
    /// suffix is the in-API signal that this is the only payload
    /// without a typed struct.
    TradeRaw(Value),
    /// Unknown wire `type`. Forward-compat: future event kinds — and
    /// malformed `err` frames missing the required `msg` field — land
    /// here without erroring out.
    Unknown { kind: String, data: Value },
    /// A known event `type` whose payload failed to deserialize into the
    /// typed struct. Carries the raw payload plus the rendered serde
    /// error so consumers can log and continue.
    ///
    /// Non-terminal (mirrors [`PublicEvent::DecodeFailed`]).
    DecodeFailed {
        kind: String,
        data: Value,
        error: String,
    },
}

impl PrivateEvent {
    /// Decode one wire envelope into a typed event. Mirrors
    /// [`PublicEvent::from_envelope`] but routes `"trade"` to
    /// [`PrivateEvent::TradeRaw`] (private feed = own-account fills,
    /// schema not in docs) and routes `"order"` to
    /// [`PrivateEvent::Order`].
    pub(crate) fn from_envelope(env: Envelope) -> Self {
        match env.kind.as_str() {
            "heartbeat" => PrivateEvent::Heartbeat,
            "err" => match parse_server_error(env.data) {
                Ok(se) => PrivateEvent::Error(se),
                Err(data) => PrivateEvent::Unknown {
                    kind: env.kind,
                    data,
                },
            },
            "order" => match serde_json::from_value::<private::OrderEvent>(env.data.clone()) {
                Ok(o) => PrivateEvent::Order(Box::new(o)),
                Err(e) => PrivateEvent::DecodeFailed {
                    kind: env.kind,
                    data: env.data,
                    error: e.to_string(),
                },
            },
            "trade" => PrivateEvent::TradeRaw(env.data),
            _ => PrivateEvent::Unknown {
                kind: env.kind,
                data: env.data,
            },
        }
    }
}
