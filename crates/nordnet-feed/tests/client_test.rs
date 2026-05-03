//! End-to-end PublicFeedClient + PrivateFeedClient tests against
//! loopback TCP. No live Nordnet host. No TLS (encrypted=false).

use nordnet_feed::{
    FeedConfig, FeedError, MarketDataKind, PrivateEvent, PrivateFeedClient, PublicEvent,
    PublicFeedClient, SubscribeArgs, MAX_FRAME_BYTES,
};
use nordnet_model::auth::Session;
use nordnet_model::ids::{MarketId, OrderId, TradableId};
use nordnet_model::models::login::Feed;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn fake_session() -> Session {
    Session {
        session_key: "session-key".into(),
        expires_in: 60,
    }
}

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
        let mut buf = [0u8; 4096];
        let n = sock.read(&mut buf).await.unwrap();
        let written = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(written.contains(r#""cmd":"login""#));
        assert!(written.contains(r#""cmd":"subscribe""#));
        let tick = r#"{"type":"price","data":{"i":"101","m":11,"bid":132.5,"ask":132.55}}"#;
        sock.write_all(tick.as_bytes()).await.unwrap();
        sock.write_all(b"\n").await.unwrap();
    });

    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    client.login(&fake_session()).await.unwrap();
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
            assert_eq!(p.identifier, TradableId::from("101"));
            assert_eq!(p.market_id, MarketId(11));
        }
        other => panic!("expected Price, got {:?}", other),
    }

    server.await.unwrap();
}

#[tokio::test]
async fn plain_tcp_path_works() {
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
        let mut buf = [0u8; 1024];
        let _ = sock.read(&mut buf).await.unwrap();
        let err_frame =
            r#"{"type":"err","data":{"msg":"Not authorized.","cmd":{"cmd":"subscribe"}}}"#;
        sock.write_all(err_frame.as_bytes()).await.unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    client.login(&fake_session()).await.unwrap();
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
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = sock.read(&mut buf).await.unwrap();
        sock.write_all(br#"{"type":"err","data":{"msg":"Login failed.","cmd":{"cmd":"login"}}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    client.login(&fake_session()).await.unwrap();
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
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
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
        Err(FeedError::Decode { .. }) => {}
        Err(FeedError::Closed) => {}
        Ok(None) => panic!("expected Err, got Ok(None) (mid-frame should not be clean EOF)"),
        Ok(Some(e)) => panic!("expected Err, got Ok(Some({:?}))", e),
        Err(other) => panic!("expected Decode or Closed, got {:?}", other),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn empty_lines_are_skipped() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        sock.write_all(b"\n\n").await.unwrap();
        sock.write_all(br#"{"type":"heartbeat","data":{}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n\n").await.unwrap();
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
async fn malformed_err_frame_routes_to_unknown() {
    // Server sends an `err` frame whose `data` is a string (not the
    // documented `{msg, cmd}` object). Client must NOT silently produce
    // ServerError { msg: "", cmd: Null } — instead route to
    // PublicEvent::Unknown so callers get a clear signal.
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        sock.write_all(br#"{"type":"err","data":"oops"}"#)
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
        PublicEvent::Unknown { kind, data } => {
            assert_eq!(kind, "err");
            assert_eq!(data, nordnet_feed::Value::String("oops".into()));
        }
        other => panic!("expected Unknown {{ kind: \"err\" }}, got {:?}", other),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn payload_type_mismatch_surfaces_as_decode_failed_event() {
    // A `price` envelope whose payload has the wrong type for `bid`
    // (object instead of number) MUST NOT terminate the connection —
    // it surfaces as PublicEvent::DecodeFailed and the next frame can
    // still be read.
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        // First: a malformed price payload.
        sock.write_all(br#"{"type":"price","data":{"i":"101","m":11,"bid":{"oops":true}}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
        // Then: a valid heartbeat — verifies the connection is still alive.
        sock.write_all(br#"{"type":"heartbeat","data":{}}"#)
            .await
            .unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let bad = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match bad {
        PublicEvent::DecodeFailed { kind, error, .. } => {
            assert_eq!(kind, "price");
            assert!(!error.is_empty());
        }
        other => panic!("expected DecodeFailed, got {:?}", other),
    }
    let good = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(good, PublicEvent::Heartbeat));
    server.await.unwrap();
}

#[tokio::test]
async fn frame_too_large_surfaces_via_recv() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let payload = vec![b'a'; MAX_FRAME_BYTES + 1];
        let _ = sock.write_all(&payload).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap();
    assert!(
        matches!(result, Err(FeedError::FrameTooLarge)),
        "expected FrameTooLarge, got {:?}",
        result
    );
    server.await.unwrap();
}

#[tokio::test]
async fn heartbeat_watchdog_fires_when_server_silent() {
    // Server accepts but never writes — recv must return HeartbeatTimeout
    // within the configured budget instead of waiting forever.
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (_sock, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    });
    let config = FeedConfig {
        connect_timeout: Duration::from_secs(2),
        heartbeat_timeout: Some(Duration::from_millis(150)),
    };
    let mut client = PublicFeedClient::connect_with(&feed, &config)
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), client.recv())
        .await
        .expect("watchdog should fire before outer timeout");
    match result {
        Err(FeedError::HeartbeatTimeout(d)) => {
            assert_eq!(d, Duration::from_millis(150));
        }
        other => panic!("expected HeartbeatTimeout, got {:?}", other),
    }
    // After watchdog fires the client is terminal.
    assert!(matches!(client.recv().await, Err(FeedError::Closed)));
    server.abort();
}

#[tokio::test]
async fn heartbeat_watchdog_does_not_fire_while_frames_arrive() {
    // Frames arriving faster than the watchdog interval reset it.
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        for _ in 0..3 {
            sock.write_all(br#"{"type":"heartbeat","data":{}}"#)
                .await
                .unwrap();
            sock.write_all(b"\n").await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    let config = FeedConfig {
        connect_timeout: Duration::from_secs(2),
        heartbeat_timeout: Some(Duration::from_millis(500)),
    };
    let mut client = PublicFeedClient::connect_with(&feed, &config)
        .await
        .unwrap();
    for _ in 0..3 {
        let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(event, PublicEvent::Heartbeat));
    }
    server.await.unwrap();
}

#[tokio::test]
async fn connect_timeout_surfaces() {
    // Reserved-for-documentation IP 192.0.2.0/24 (TEST-NET-1, RFC 5737)
    // is unroutable — TCP connect must time out within the budget.
    let feed = Feed {
        hostname: "192.0.2.1".into(),
        port: 9,
        encrypted: false,
    };
    let config = FeedConfig {
        connect_timeout: Duration::from_millis(100),
        heartbeat_timeout: None,
    };
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        PublicFeedClient::connect_with(&feed, &config),
    )
    .await
    .expect("connect should fail before outer timeout");
    match result {
        Err(FeedError::ConnectTimeout(d)) => assert_eq!(d, Duration::from_millis(100)),
        other => panic!("expected ConnectTimeout, got {:?}", other.err()),
    }
}

#[tokio::test]
async fn calls_after_terminal_error_return_closed() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        sock.write_all(b"not-valid-json\n").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap();
    assert!(
        matches!(first, Err(FeedError::Decode { .. })),
        "expected Decode error, got {:?}",
        first
    );
    let second = client.recv().await;
    assert!(
        matches!(second, Err(FeedError::Closed)),
        "expected Closed after first error, got {:?}",
        second
    );
    let send = client.login(&fake_session()).await;
    assert!(
        matches!(send, Err(FeedError::Closed)),
        "expected Closed for send-after-terminal, got {:?}",
        send
    );
    let sub = client
        .subscribe(SubscribeArgs::MarketData {
            kind: MarketDataKind::Price,
            market: 11,
            identifier: "101".into(),
        })
        .await;
    assert!(
        matches!(sub, Err(FeedError::Closed)),
        "expected Closed for subscribe-after-terminal, got {:?}",
        sub
    );
    server.await.unwrap();
}

#[tokio::test]
async fn calls_after_clean_eof_return_closed() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (sock, _) = listener.accept().await.unwrap();
        drop(sock);
    });
    let mut client = PublicFeedClient::connect(&feed).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(first.is_none(), "expected Ok(None), got {:?}", first);
    let second = client.recv().await;
    assert!(
        matches!(second, Err(FeedError::Closed)),
        "expected Closed after EOF, got {:?}",
        second
    );
    server.await.unwrap();
}

#[tokio::test]
async fn private_feed_order_event_round_trip() {
    let (listener, feed) = loopback().await;
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = sock.read(&mut buf).await.unwrap();
        let order = r#"{"type":"order","data":{"volume":111.0,"price":{"value":132.55,"currency":"SEK"},"volume_condition":"NORMAL","order_id":202178767,"reference":"ABC132","tradable":{"market_id":11,"identifier":"101"},"validity":{"type":"DAY","valid_until":1613061300000},"accno":123123,"accid":1,"side":"BUY","modified":1612955053717,"activation_condition":{"type":"NONE"},"order_state":"LOCAL","action_state":"INS_PEND","order_type":"LIMIT"}}"#;
        sock.write_all(order.as_bytes()).await.unwrap();
        sock.write_all(b"\n").await.unwrap();
    });
    let mut client = PrivateFeedClient::connect(&feed).await.unwrap();
    client.login(&fake_session()).await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), client.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        PrivateEvent::Order(o) => {
            assert_eq!(o.order_id, OrderId(202178767));
        }
        other => panic!("expected Order, got {:?}", other),
    }
    server.await.unwrap();
}
