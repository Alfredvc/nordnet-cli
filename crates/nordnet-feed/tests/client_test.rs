//! End-to-end PublicFeedClient + PrivateFeedClient tests against
//! loopback TCP. No live Nordnet host. No TLS (encrypted=false).

use nordnet_feed::command::{MarketDataKind, SubscribeArgs};
use nordnet_feed::{FeedError, PrivateEvent, PrivateFeedClient, PublicEvent, PublicFeedClient};
use nordnet_model::models::login::Feed;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Bind a loopback TCP listener and return (listener, plain Feed).
async fn loopback() -> (TcpListener, Feed) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let feed = Feed {
        hostname: "127.0.0.1".into(),
        port: port as i64,
        encrypted: false,
    };
    (listener, feed)
}

#[tokio::test]
async fn subscribe_then_recv_price_tick() {
    let (listener, feed) = loopback().await;

    let server = tokio::spawn(async move {
        let (mut sock, _addr) = listener.accept().await.unwrap();
        // Read whatever the client writes (login + subscribe frames).
        let mut buf = [0u8; 4096];
        let n = sock.read(&mut buf).await.unwrap();
        let written = std::str::from_utf8(&buf[..n]).unwrap();
        // Both frames separated by '\n':
        assert!(written.contains(r#""cmd":"login""#));
        assert!(written.contains(r#""cmd":"subscribe""#));
        // Reply with one price tick:
        let tick = r#"{"type":"price","data":{"i":"101","m":11,"bid":132.5,"ask":132.55}}"#;
        sock.write_all(tick.as_bytes()).await.unwrap();
        sock.write_all(b"\n").await.unwrap();
        // Drop server socket to send EOF.
    });

    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    client.login("session-key").await.unwrap();
    client
        .subscribe(SubscribeArgs::MarketData {
            kind: MarketDataKind::Price,
            market: 11,
            identifier: "101".into(),
        })
        .await
        .unwrap();

    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .expect("recv timed out")
        .expect("recv errored")
        .expect("got None instead of an event");
    match event {
        PublicEvent::Price(p) => {
            assert_eq!(p.i, "101");
            assert_eq!(p.m, 11);
        }
        other => panic!("expected Price, got {:?}", other),
    }

    server.await.unwrap();
}

#[tokio::test]
async fn plain_tcp_path_works() {
    // Identical to the test above but more focused: just verify
    // encrypted=false connects + exchanges a frame without errors.
    let (listener, feed) = loopback().await;

    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        sock.write_all(br#"{"type":"heartbeat","data":{}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
    });

    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(event, PublicEvent::Heartbeat));
    server.await.unwrap();
}

#[tokio::test]
async fn heartbeat_with_extra_fields_stays_heartbeat() {
    // Forward-compat: server adds {"server_time":123} inside the
    // heartbeat data. Must NOT route to Unknown.
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        sock.write_all(br#"{"type":"heartbeat","data":{"server_time":1234567890}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(event, PublicEvent::Heartbeat));
    server.await.unwrap();
}

#[tokio::test]
async fn unknown_envelope_type_lands_in_unknown() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        sock.write_all(br#"{"type":"future_kind","data":{"x":1}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        PublicEvent::Unknown { kind, .. } => assert_eq!(kind, "future_kind"),
        other => panic!("expected Unknown, got {:?}", other),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn server_err_surfaces_as_event_not_result_err() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        // Discard whatever the client wrote.
        let mut buf = [0u8; 1024];
        let _ = sock.read(&mut buf).await.unwrap();
        let err_frame =
            r#"{"type":"err","data":{"msg":"Not authorized.","cmd":{"cmd":"subscribe"}}}"#;
        sock.write_all(err_frame.as_bytes()).await.unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    client.login("k").await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        PublicEvent::Error(e) => assert_eq!(e.msg, "Not authorized."),
        other => panic!("expected Error, got {:?}", other),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn login_error_then_close_returns_none_after_err() {
    // Mock sequence per spec §"Testing strategy" line 483: client sends
    // login + 3 subscribes; server replies with one err and closes.
    // err arrives as Event::Error; next recv returns Ok(None) (clean
    // EOF after the err frame).
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = sock.read(&mut buf).await.unwrap();
        sock.write_all(br#"{"type":"err","data":{"msg":"Login failed.","cmd":{"cmd":"login"}}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
        // Server drops socket -> client side reads EOF.
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    client.login("k").await.unwrap();
    for _ in 0..3 {
        client
            .subscribe(SubscribeArgs::MarketData {
                kind: MarketDataKind::Price,
                market: 11,
                identifier: "101".into(),
            })
            .await
            .unwrap();
    }
    let first = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(first, PublicEvent::Error(_)));
    let second = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(
        second.is_none(),
        "expected Ok(None) after server EOF, got {:?}",
        second
    );
    server.await.unwrap();
}

#[tokio::test]
async fn mid_frame_disconnect_returns_err() {
    // When the server writes a partial JSON frame and then calls
    // `sock.shutdown()`, the TCP half-close delivers a clean EOF to the
    // client. `LinesCodec` treats clean EOF as an implicit line
    // terminator and returns the buffered partial data as a "line".
    // `serde_json` then fails to parse the truncated JSON, surfacing
    // as `FeedError::Decode`. The important guarantee: the result is
    // `Err(...)`, never `Ok(Some(event))` (no silently malformed event).
    //
    // Note: `FeedError::Closed` (which maps `UnexpectedEof` IO error)
    // would instead surface if the OS delivers an abrupt RST rather than
    // a graceful FIN — that is a different OS-level condition. For a
    // graceful shutdown the decode error is the correct observable result
    // from `LinesCodec`.
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        // Write half a frame (no terminator) then half-close the send side.
        sock.write_all(br#"{"type":"price","data":{"i":"101","m":11"#)
            .await
            .unwrap();
        sock.shutdown().await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap();
    match result {
        // LinesCodec delivers the partial buffer on clean EOF; serde_json
        // then fails to parse the truncated JSON.
        Err(FeedError::Decode { .. }) => {} // expected
        // FeedError::Closed is also acceptable if the OS sends an abrupt RST.
        Err(FeedError::Closed) => {}
        Ok(None) => panic!("expected Err, got Ok(None) (mid-frame should not be clean EOF)"),
        Ok(Some(e)) => panic!("expected Err, got Ok(Some({:?}))", e),
        Err(other) => panic!("expected Decode or Closed, got {:?}", other),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn private_feed_order_event_round_trip() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = sock.read(&mut buf).await.unwrap();
        // Push the spec golden order payload.
        let order = r#"{"type":"order","data":{"volume":111.0,"price":{"value":132.55,"currency":"SEK"},"volume_condition":"NORMAL","order_id":202178767,"reference":"ABC132","tradable":{"market_id":11,"identifier":"101"},"validity":{"type":"DAY","valid_until":1613061300000},"accno":123123,"accid":1,"side":"BUY","modified":1612955053717,"activation_condition":{"type":"NONE"},"order_state":"LOCAL","action_state":"INS_PEND","order_type":"LIMIT"}}"#;
        sock.write_all(order.as_bytes()).await.unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PrivateFeedClient::connect(&feed).await.unwrap();
    client.login("k").await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        PrivateEvent::Order(o) => {
            assert_eq!(o.order_id, 202178767);
        }
        other => panic!("expected Order, got {:?}", other),
    }
    server.await.unwrap();
}
