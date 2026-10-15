// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

use crate::{ArkSyncEnvelope, CONFIG};

/// Failure while encoding or decoding an ArkSync protocol frame.
#[derive(Debug, PartialEq)]
pub enum ProtocolFrameError {
    BufferTooSmall,
    InvalidFrameMagic,
    UnsupportedVersion(u8),
    Postcard(postcard::Error),
}

impl core::fmt::Display for ProtocolFrameError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => formatter.write_str("ArkSync frame buffer is too small"),
            Self::InvalidFrameMagic => formatter.write_str("invalid ArkSync frame magic"),
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
pub fn encode_frame<'b, Message>(
    envelope: &ArkSyncEnvelope<Message>,
    buffer: &'b mut [u8],
) -> Result<&'b [u8], ProtocolFrameError>
where
    Message: Serialize,
{
    let config = CONFIG.load();
    let header_len = config.frame_header_len();

    if buffer.len() < header_len {
        return Err(ProtocolFrameError::BufferTooSmall);
    }

    buffer[..config.frame_magic.len()].copy_from_slice(&config.frame_magic);
    buffer[config.frame_magic.len()] = config.protocol_version;
    let payload = postcard::to_slice(envelope, &mut buffer[header_len..])?;
    let frame_len = header_len + payload.len();

    Ok(&buffer[..frame_len])
}

/// Validates and decodes one complete ArkSync protocol frame.
///
/// The transport or routing context selects the actor-specific `Message` type.
pub fn decode_frame<'f, Message>(
    frame: &'f [u8],
) -> Result<ArkSyncEnvelope<Message>, ProtocolFrameError>
where
    Message: Deserialize<'f>,
{
    let config = CONFIG.load();
    let header_len = config.frame_header_len();

    if frame.len() < header_len {
        return Err(ProtocolFrameError::BufferTooSmall);
    }
    if frame[..config.frame_magic.len()] != config.frame_magic {
        return Err(ProtocolFrameError::InvalidFrameMagic);
    }

    let version = frame[config.frame_magic.len()];
    if !config.supports_version(version) {
        return Err(ProtocolFrameError::UnsupportedVersion(version));
    }

    postcard::from_bytes(&frame[header_len..]).map_err(Into::into)
}
