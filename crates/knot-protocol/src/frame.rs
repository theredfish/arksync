// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::KnotEnvelope;

pub const KNOT_FRAME_MAGIC: [u8; 4] = *b"ARSK";
pub const KNOT_PROTOCOL_VERSION: u8 = 1;
pub const KNOT_FRAME_HEADER_LEN: usize = KNOT_FRAME_MAGIC.len() + 1;

#[derive(Debug, PartialEq)]
pub enum KnotFrameError {
    BufferTooSmall,
    InvalidMagic,
    UnsupportedVersion(u8),
    Postcard(postcard::Error),
}

impl core::fmt::Display for KnotFrameError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => formatter.write_str("Knot frame buffer is too small"),
            Self::InvalidMagic => formatter.write_str("invalid Knot frame magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported Knot protocol version {version}")
            }
            Self::Postcard(error) => write!(formatter, "invalid Knot Postcard payload: {error}"),
        }
    }
}

impl From<postcard::Error> for KnotFrameError {
    fn from(value: postcard::Error) -> Self {
        Self::Postcard(value)
    }
}

pub fn encode_knot_frame<'buffer>(
    envelope: &KnotEnvelope,
    buffer: &'buffer mut [u8],
) -> Result<&'buffer [u8], KnotFrameError> {
    if buffer.len() < KNOT_FRAME_HEADER_LEN {
        return Err(KnotFrameError::BufferTooSmall);
    }

    buffer[..KNOT_FRAME_MAGIC.len()].copy_from_slice(&KNOT_FRAME_MAGIC);
    buffer[KNOT_FRAME_MAGIC.len()] = KNOT_PROTOCOL_VERSION;
    let payload = postcard::to_slice(envelope, &mut buffer[KNOT_FRAME_HEADER_LEN..])?;
    let frame_len = KNOT_FRAME_HEADER_LEN + payload.len();

    Ok(&buffer[..frame_len])
}

pub fn decode_knot_frame(frame: &[u8]) -> Result<KnotEnvelope, KnotFrameError> {
    if frame.len() < KNOT_FRAME_HEADER_LEN {
        return Err(KnotFrameError::BufferTooSmall);
    }
    if frame[..KNOT_FRAME_MAGIC.len()] != KNOT_FRAME_MAGIC {
        return Err(KnotFrameError::InvalidMagic);
    }

    let version = frame[KNOT_FRAME_MAGIC.len()];
    if version != KNOT_PROTOCOL_VERSION {
        return Err(KnotFrameError::UnsupportedVersion(version));
    }

    postcard::from_bytes(&frame[KNOT_FRAME_HEADER_LEN..]).map_err(Into::into)
}
