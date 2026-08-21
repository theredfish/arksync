// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg(feature = "knot-tokio-runtime")]

use std::io;

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::MessageLink;
use arksync_knot_protocol::{
    decode_knot_frame, encode_knot_frame, KnotAck, KnotCapabilities, KnotConfig,
    KnotControlMessage, KnotEnvelope, KnotFrameError, KnotHello, KnotMessage, KnotMessageSource,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_FRAME_LEN: usize = 4_096;

#[derive(Debug)]
enum TcpPostcardLinkError {
    Io(io::Error),
    Frame(KnotFrameError),
    FrameTooLarge(usize),
}

impl From<io::Error> for TcpPostcardLinkError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<KnotFrameError> for TcpPostcardLinkError {
    fn from(value: KnotFrameError) -> Self {
        Self::Frame(value)
    }
}

struct TcpPostcardLink {
    stream: TcpStream,
}

impl TcpPostcardLink {
    async fn connect(address: std::net::SocketAddr) -> Result<Self, TcpPostcardLinkError> {
        Ok(Self {
            stream: TcpStream::connect(address).await?,
        })
    }

    fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}

impl MessageLink<KnotEnvelope> for TcpPostcardLink {
    type Error = TcpPostcardLinkError;

    async fn send(&mut self, message: KnotEnvelope) -> Result<(), Self::Error> {
        let mut buffer = [0; MAX_FRAME_LEN];
        let frame = encode_knot_frame(&message, &mut buffer)?;
        let frame_len = u32::try_from(frame.len())
            .map_err(|_| TcpPostcardLinkError::FrameTooLarge(frame.len()))?;

        self.stream.write_u32(frame_len).await?;
        self.stream.write_all(frame).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<KnotEnvelope>, Self::Error> {
        let frame_len = self.stream.read_u32().await? as usize;
        if frame_len > MAX_FRAME_LEN {
            return Err(TcpPostcardLinkError::FrameTooLarge(frame_len));
        }

        let mut frame = vec![0; frame_len];
        self.stream.read_exact(&mut frame).await?;
        Ok(Some(decode_knot_frame(&frame)?))
    }
}

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn hello(event_id: EventId) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        KnotMessageSource::Knot {
            hardware_uid: "knot-rpi-1".into(),
        },
        timestamp(1_780_000_000_000),
        KnotMessage::Control(KnotControlMessage::Hello(KnotHello {
            hardware_uid: "knot-rpi-1".into(),
            capabilities: KnotCapabilities {
                gpio: true,
                uart: true,
                i2c: false,
                atlas_scientific_ezo: true,
            },
            last_applied_config_version: None,
        })),
    )
}

fn hello_ack(event_id: EventId) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([2; 16]),
        KnotMessageSource::Hub { hub_id: [3; 16] },
        timestamp(1_780_000_000_100),
        KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello {
            event_id,
            config: KnotConfig {
                version: 1,
                knot_id: [4; 16],
                sensor_bindings: vec![],
                actuator_configs: vec![],
            },
        })),
    )
}

#[tokio::test]
async fn tcp_reconnect_retries_the_same_postcard_message() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let hello_id = EventId::from_bytes([1; 16]);
    let envelope = hello(hello_id);
    let expected = envelope.clone();

    let server = tokio::spawn(async move {
        let (first_stream, _) = listener.accept().await.unwrap();
        let mut first_link = TcpPostcardLink::new(first_stream);
        let first = first_link.receive().await.unwrap().unwrap();
        assert_eq!(first, expected);
        drop(first_link);

        let (second_stream, _) = listener.accept().await.unwrap();
        let mut second_link = TcpPostcardLink::new(second_stream);
        let retry = second_link.receive().await.unwrap().unwrap();
        assert_eq!(retry, expected);
        second_link.send(hello_ack(retry.id)).await.unwrap();
    });

    let mut first_link = TcpPostcardLink::connect(address).await.unwrap();
    first_link.send(envelope.clone()).await.unwrap();
    assert!(matches!(
        first_link.receive().await,
        Err(TcpPostcardLinkError::Io(_))
    ));

    let mut second_link = TcpPostcardLink::connect(address).await.unwrap();
    second_link.send(envelope).await.unwrap();
    let ack = second_link.receive().await.unwrap().unwrap();

    assert!(matches!(
        ack.payload,
        KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello { event_id, .. }))
            if event_id == hello_id
    ));
    server.await.unwrap();
}
