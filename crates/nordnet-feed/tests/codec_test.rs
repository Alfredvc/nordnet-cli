//! Codec framing round-trip + 1 MiB cap behavior.
//!
//! Uses `tokio::io::duplex` to exercise newline framing without a real
//! socket. TLS path is NOT covered here — TLS testing would require a
//! live Nordnet host, which is prohibited by the pipeline's no-API rule.
//!
//! # Cap semantics (LinesCodec behavior)
//!
//! `LinesCodec::new_with_max_length(n)` errors when `buf.len() > n` with
//! no newline found. This means:
//!
//! - A frame of *exactly* `n` bytes is accepted (the `>` check is strict).
//! - A frame of `n + 1` bytes or more (before a `\n`) triggers the error.
//!
//! Source verified in tokio-util 0.7.18 `lines_codec.rs` line 151:
//! `(false, None) if buf.len() > self.max_length`.

use futures_util::{SinkExt, StreamExt};
use nordnet_feed::codec::{new_lines_codec, MAX_FRAME_BYTES};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

#[tokio::test]
async fn round_trip_three_frames() {
    let (server, client) = tokio::io::duplex(8192);
    let mut framed = Framed::new(client, new_lines_codec());

    // Write three newline-terminated frames on the server side, then close.
    tokio::spawn(async move {
        let mut s = server;
        s.write_all(b"frame-one\n").await.unwrap();
        s.write_all(b"frame-two\n").await.unwrap();
        s.write_all(b"frame-three\n").await.unwrap();
        s.shutdown().await.unwrap();
    });

    let one = framed.next().await.unwrap().unwrap();
    let two = framed.next().await.unwrap().unwrap();
    let three = framed.next().await.unwrap().unwrap();
    assert_eq!(one, "frame-one");
    assert_eq!(two, "frame-two");
    assert_eq!(three, "frame-three");

    let eof = framed.next().await;
    assert!(eof.is_none(), "stream should terminate cleanly after EOF");
}

#[tokio::test]
async fn frame_at_one_mib_passes() {
    // A frame of exactly MAX_FRAME_BYTES is accepted — the cap is strict (>),
    // so equality is NOT an error. See module-level doc comment for details.
    let payload = vec![b'a'; MAX_FRAME_BYTES];
    // The duplex must be large enough to buffer the entire payload + newline.
    let (server, client) = tokio::io::duplex(MAX_FRAME_BYTES + 8192);
    let mut framed = Framed::new(client, new_lines_codec());

    tokio::spawn(async move {
        let mut s = server;
        s.write_all(&payload).await.unwrap();
        s.write_all(b"\n").await.unwrap();
        s.shutdown().await.unwrap();
    });

    let line = framed.next().await.unwrap().unwrap();
    assert_eq!(line.len(), MAX_FRAME_BYTES);
}

#[tokio::test]
async fn frame_one_byte_over_one_mib_errors() {
    // A frame of MAX_FRAME_BYTES + 1 bytes (before the newline) is over
    // the limit and must yield LinesCodecError::MaxLineLengthExceeded.
    let oversize = MAX_FRAME_BYTES + 1;
    let payload = vec![b'a'; oversize];
    // Duplex must be large enough to buffer oversize payload + newline.
    let (server, client) = tokio::io::duplex(oversize + 8192);
    let mut framed = Framed::new(client, new_lines_codec());

    tokio::spawn(async move {
        let mut s = server;
        s.write_all(&payload).await.unwrap();
        s.write_all(b"\n").await.unwrap();
        // No shutdown — let the framed error fire first.
    });

    let err = framed.next().await.unwrap().unwrap_err();
    assert!(
        matches!(err, LinesCodecError::MaxLineLengthExceeded),
        "expected MaxLineLengthExceeded, got: {err:?}"
    );
}

#[tokio::test]
async fn write_emits_newline_terminator() {
    // The codec encoder appends a `\n` when sending. Verify the raw bytes
    // received on the peer side include the terminator.
    let (mut server, client) = tokio::io::duplex(1024);
    let mut framed: Framed<_, LinesCodec> = Framed::new(client, new_lines_codec());

    framed.send("hello".to_string()).await.unwrap();
    // `send` writes the frame + newline; `flush` ensures the bytes are
    // pushed through the duplex before we read the server side. Annotating
    // `framed` above removes the ambiguity Rust sees on `SinkExt::flush`.
    SinkExt::<String>::flush(&mut framed).await.unwrap();
    drop(framed);

    let mut buf = vec![0u8; 64];
    let n = tokio::io::AsyncReadExt::read(&mut server, &mut buf)
        .await
        .unwrap();
    assert_eq!(&buf[..n], b"hello\n");
}
