// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

use crate::ArkSyncEnvelope;

/// Prefix identifying an ArkSync frame before decoding its payload.
pub const ARKSYNC_FRAME_MAGIC: [u8; 4] = *b"ARSK";
/// Wire representation version encoded in every ArkSync frame.
pub const ARKSYNC_PROTOCOL_VERSION: u8 = 1;
/// Number of bytes preceding the Postcard payload.
pub const ARKSYNC_FRAME_HEADER_LEN: usize = ARKSYNC_FRAME_MAGIC.len() + 1;

/// Failure while encoding or decoding an ArkSync protocol frame.
#[derive(Debug, PartialEq)]
pub enum ProtocolFrameError {
    BufferTooSmall,
    InvalidMagic,
    UnsupportedVersion(u8),
    Postcard(postcard::Error),
}

impl core::fmt::Display for ProtocolFrameError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => formatter.write_str("ArkSync frame buffer is too small"),
            Self::InvalidMagic => formatter.write_str("invalid ArkSync frame magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported ArkSync protocol version {version}")
            }
            Self::Postcard(error) => write!(formatter, "invalid ArkSync Postcard payload: {error}"),
        }
    }
}

impl From<postcard::Error> for ProtocolFrameError {
    fn from(value: postcard::Error) -> Self {
        Self::Postcard(value)
    }
}

/// Encodes one complete ArkSync envelope into a caller-provided bounded buffer.
pub fn encode_frame<'buffer, Message>(
    envelope: &ArkSyncEnvelope<Message>,
    buffer: &'buffer mut [u8],
) -> Result<&'buffer [u8], ProtocolFrameError>
where
    Message: Serialize,
{
    if buffer.len() < ARKSYNC_FRAME_HEADER_LEN {
        return Err(ProtocolFrameError::BufferTooSmall);
    }

    buffer[..ARKSYNC_FRAME_MAGIC.len()].copy_from_slice(&ARKSYNC_FRAME_MAGIC);
    buffer[ARKSYNC_FRAME_MAGIC.len()] = ARKSYNC_PROTOCOL_VERSION;
    let payload = postcard::to_slice(envelope, &mut buffer[ARKSYNC_FRAME_HEADER_LEN..])?;
    let frame_len = ARKSYNC_FRAME_HEADER_LEN + payload.len();

    Ok(&buffer[..frame_len])
}

/// Validates and decodes one complete ArkSync protocol frame.
///
/// The transport or routing context selects the actor-specific `Message` type.
pub fn decode_frame<'frame, Message>(
    frame: &'frame [u8],
) -> Result<ArkSyncEnvelope<Message>, ProtocolFrameError>
where
    Message: Deserialize<'frame>,
{
    if frame.len() < ARKSYNC_FRAME_HEADER_LEN {
        return Err(ProtocolFrameError::BufferTooSmall);
    }
    if frame[..ARKSYNC_FRAME_MAGIC.len()] != ARKSYNC_FRAME_MAGIC {
        return Err(ProtocolFrameError::InvalidMagic);
    }

    let version = frame[ARKSYNC_FRAME_MAGIC.len()];
    if version != ARKSYNC_PROTOCOL_VERSION {
        return Err(ProtocolFrameError::UnsupportedVersion(version));
    }

    postcard::from_bytes(&frame[ARKSYNC_FRAME_HEADER_LEN..]).map_err(Into::into)
}
