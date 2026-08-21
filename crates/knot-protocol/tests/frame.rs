// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot_protocol::{
    decode_knot_frame, encode_knot_frame, KnotCapabilities, KnotControlMessage, KnotEnvelope,
    KnotFrameError, KnotHello, KnotMessage, KnotMessageSource, KNOT_FRAME_HEADER_LEN,
    KNOT_FRAME_MAGIC, KNOT_PROTOCOL_VERSION,
};

fn hello_envelope() -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([1; 16]),
        KnotMessageSource::Knot {
            hardware_uid: "knot-rpi-1".into(),
        },
        Timestamp::from_unix_millis(1_779_840_000_000),
        KnotMessage::Control(KnotControlMessage::Hello(KnotHello {
            hardware_uid: "knot-rpi-1".into(),
            capabilities: KnotCapabilities {
                gpio: true,
                uart: true,
                i2c: false,
                atlas_scientific_ezo: true,
            },
            last_applied_config_version: Some(7),
        })),
    )
}

#[test]
fn frame_round_trips_a_hello_message() {
    let envelope = hello_envelope();
    let mut buffer = [0; 256];

    let frame = encode_knot_frame(&envelope, &mut buffer).unwrap();
    let decoded = decode_knot_frame(frame).unwrap();

    assert_eq!(&frame[..KNOT_FRAME_MAGIC.len()], KNOT_FRAME_MAGIC);
    assert_eq!(frame[KNOT_FRAME_MAGIC.len()], KNOT_PROTOCOL_VERSION);
    assert_eq!(decoded, envelope);
}

#[test]
fn hello_frame_has_a_stable_v1_representation() {
    let envelope = hello_envelope();
    let mut buffer = [0; 256];

    let frame = encode_knot_frame(&envelope, &mut buffer).unwrap();

    assert_eq!(
        frame,
        &[
            65, 82, 83, 75, 1, 1, 1, 1, 1, 1, 1, 65, 1, 129, 1, 1, 1, 1, 1, 1, 1, 1, 10, 107, 110,
            111, 116, 45, 114, 112, 105, 45, 49, 128, 128, 217, 235, 204, 103, 0, 0, 10, 107, 110,
            111, 116, 45, 114, 112, 105, 45, 49, 1, 1, 0, 1, 1, 7,
        ]
    );
}

#[test]
fn frame_rejects_invalid_magic_and_version() {
    let envelope = hello_envelope();
    let mut buffer = [0; 256];
    let frame = encode_knot_frame(&envelope, &mut buffer).unwrap();
    let mut invalid_magic = frame.to_vec();
    invalid_magic[0] = b'X';
    let mut invalid_version = frame.to_vec();
    invalid_version[KNOT_FRAME_MAGIC.len()] = KNOT_PROTOCOL_VERSION + 1;

    assert_eq!(
        decode_knot_frame(&invalid_magic),
        Err(KnotFrameError::InvalidMagic)
    );
    assert_eq!(
        decode_knot_frame(&invalid_version),
        Err(KnotFrameError::UnsupportedVersion(
            KNOT_PROTOCOL_VERSION + 1
        ))
    );
}

#[test]
fn frame_rejects_a_short_buffer() {
    let envelope = hello_envelope();
    let mut buffer = [0; KNOT_FRAME_HEADER_LEN];

    let result = encode_knot_frame(&envelope, &mut buffer);

    assert!(matches!(result, Err(KnotFrameError::Postcard(_))));
    assert_eq!(
        decode_knot_frame(&buffer[..KNOT_FRAME_HEADER_LEN - 1]),
        Err(KnotFrameError::BufferTooSmall)
    );
}
