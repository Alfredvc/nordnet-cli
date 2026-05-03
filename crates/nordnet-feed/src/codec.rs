//! Newline-JSON frame codec for Nordnet feed sockets.
//!
//! Wraps [`tokio_util::codec::LinesCodec`] with a 1 MiB max frame size.
//! Larger frames return [`FeedError::FrameTooLarge`].

use tokio_util::codec::LinesCodec;

/// Maximum frame size in bytes. Designer choice — Nordnet docs do not
/// specify. Sized to fit any plausible event while preventing
/// memory-DoS from malformed input.
pub const MAX_FRAME_BYTES: usize = 1 << 20; // 1 MiB

/// Construct a fresh `LinesCodec` configured with the feed's max frame
/// size. Use with `tokio_util::codec::Framed` over an `AsyncRead +
/// AsyncWrite` socket.
pub fn new_lines_codec() -> LinesCodec {
    LinesCodec::new_with_max_length(MAX_FRAME_BYTES)
}
