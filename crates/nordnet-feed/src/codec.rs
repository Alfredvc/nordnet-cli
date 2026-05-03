//! Newline-JSON frame codec for Nordnet feed sockets.
//!
//! Wraps [`tokio_util::codec::LinesCodec`] with a 1 MiB max frame size.
//! Larger frames return [`crate::FeedError::FrameTooLarge`].
//!
//! # Frame terminator quirks
//!
//! `LinesCodec` accepts both LF (`\n`) and CRLF (`\r\n`) line endings —
//! a trailing `\r` immediately before the `\n` terminator is silently
//! stripped from the decoded line. Nordnet uses LF-only per protocol,
//! so this is a no-op in practice; documented here so the behavior is
//! discoverable to anyone changing the framing later.

use tokio_util::codec::LinesCodec;

/// Maximum frame size in bytes. Designer choice — Nordnet docs do not
/// specify. Sized to fit any plausible event while preventing
/// memory-DoS from malformed input.
pub const MAX_FRAME_BYTES: usize = 1 << 20; // 1 MiB

/// Construct a fresh `LinesCodec` configured with the feed's max frame
/// size. Use with `tokio_util::codec::Framed` over an `AsyncRead +
/// AsyncWrite` socket.
pub(crate) fn new_lines_codec() -> LinesCodec {
    LinesCodec::new_with_max_length(MAX_FRAME_BYTES)
}

#[cfg(test)]
mod tests {
    //! Codec framing round-trip + 1 MiB cap behavior.
    //!
    //! Uses `tokio::io::duplex` to exercise newline framing without a
    //! real socket. TLS path is NOT covered here — TLS testing would
    //! require a live Nordnet host, which is prohibited by the
    //! pipeline's no-API rule.
    //!
    //! `LinesCodec::new_with_max_length(n)` errors when `buf.len() > n`
    //! with no newline found:
    //! - A frame of *exactly* `n` bytes is accepted.
    //! - A frame of `n + 1` bytes or more (before a `\n`) errors.
    //!
    //! Source verified in tokio-util 0.7 `lines_codec.rs` line 151:
    //! `(false, None) if buf.len() > self.max_length`.

    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio::io::AsyncWriteExt;
    use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

    #[tokio::test]
    async fn round_trip_three_frames() {
        let (server, client) = tokio::io::duplex(8192);
        let mut framed = Framed::new(client, new_lines_codec());

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
        let payload = vec![b'a'; MAX_FRAME_BYTES];
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
        let oversize = MAX_FRAME_BYTES + 1;
        let payload = vec![b'a'; oversize];
        let (server, client) = tokio::io::duplex(oversize + 8192);
        let mut framed = Framed::new(client, new_lines_codec());

        tokio::spawn(async move {
            let mut s = server;
            s.write_all(&payload).await.unwrap();
            s.write_all(b"\n").await.unwrap();
        });

        let err = framed.next().await.unwrap().unwrap_err();
        assert!(
            matches!(err, LinesCodecError::MaxLineLengthExceeded),
            "expected MaxLineLengthExceeded, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn write_emits_newline_terminator() {
        let (mut server, client) = tokio::io::duplex(1024);
        let mut framed: Framed<_, LinesCodec> = Framed::new(client, new_lines_codec());

        framed.send("hello".to_string()).await.unwrap();
        SinkExt::<String>::flush(&mut framed).await.unwrap();
        drop(framed);

        let mut buf = vec![0u8; 64];
        let n = tokio::io::AsyncReadExt::read(&mut server, &mut buf)
            .await
            .unwrap();
        assert_eq!(&buf[..n], b"hello\n");
    }
}
