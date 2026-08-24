// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_protocol::knot::{
    KnotAck, KnotConfigApplied, KnotConfigRejected, KnotControlMessage, KnotEnvelope, KnotMessage,
    KnotNack, KnotNackReason,
};
use arksync_protocol::ArkSyncActor;
use eyre::{Result, WrapErr};
use sqlx::PgPool;

use crate::application::register_knot_hello;
use crate::application::{HubSensorEventEnvelope, SensorRegistry};
use crate::config::CONFIG;
use crate::infrastructure::store::{
    insert_knot_message_receipt, knot as knot_store, KnotMessageStoreError,
};

use super::knot_sensor_protocol::handle_knot_sensor_protocol;

pub struct KnotProtocolEventResult {
    pub response: Option<KnotEnvelope>,
    pub sensor_events: Vec<HubSensorEventEnvelope>,
}

pub async fn handle_knot_protocol_event(
    pool: &PgPool,
    event: &KnotEnvelope,
    response_event_id: EventId,
    responded_at: Timestamp,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
) -> Result<KnotProtocolEventResult> {
    let ArkSyncActor::Knot { hardware_uid } = &event.source else {
        return Ok(KnotProtocolEventResult {
            response: Some(response(
                response_event_id,
                responded_at,
                KnotControlMessage::Nack(KnotNack {
                    event_id: event.id,
                    reason: KnotNackReason::InvalidPayload,
                }),
            )),
            sensor_events: Vec::new(),
        });
    };

    let KnotMessage::Sensor(message) = &event.payload else {
        return handle_control_message(pool, event, response_event_id, responded_at).await;
    };

    let sensor_events = handle_knot_sensor_protocol(
        pool,
        event.id,
        hardware_uid,
        message,
        event.occurred_at,
        received_at,
        sensor_registry,
    )
    .await?;

    Ok(KnotProtocolEventResult {
        response: Some(response(
            response_event_id,
            responded_at,
            KnotControlMessage::Ack(KnotAck::Processed { event_id: event.id }),
        )),
        sensor_events,
    })
}

async fn handle_control_message(
    pool: &PgPool,
    event: &KnotEnvelope,
    response_event_id: EventId,
    responded_at: Timestamp,
) -> Result<KnotProtocolEventResult> {
    let ArkSyncActor::Knot { hardware_uid } = &event.source else {
        return Err(eyre::eyre!("control message source is not a Knot"));
    };
    let KnotMessage::Control(message) = &event.payload else {
        return Err(eyre::eyre!("protocol payload is not a control message"));
    };

    let response_message = match message {
        KnotControlMessage::Hello(hello) => {
            if hello.hardware_uid != *hardware_uid {
                KnotControlMessage::Nack(KnotNack {
                    event_id: event.id,
                    reason: KnotNackReason::InvalidPayload,
                })
            } else {
                let mut txn = pool
                    .begin()
                    .await
                    .wrap_err("failed to begin Knot Hello transaction")?;
                let is_new =
                    insert_knot_message_receipt(&mut *txn, event.id, hardware_uid, "hello")
                        .await
                        .map_err(message_store_error)?;
                let config = if is_new {
                    register_knot_hello(&mut txn, hello)
                        .await
                        .wrap_err("failed to register Knot Hello")?
                } else {
                    crate::application::knot_protocol_config_for_hardware_uid(
                        &mut txn,
                        hardware_uid,
                    )
                    .await
                    .wrap_err("failed to reload config for duplicate Knot Hello")?
                };
                txn.commit()
                    .await
                    .wrap_err("failed to commit Knot Hello transaction")?;

                KnotControlMessage::Ack(KnotAck::Hello {
                    event_id: event.id,
                    config,
                })
            }
        }
        KnotControlMessage::ConfigApplied(applied) => {
            record_config_applied(pool, event.id, hardware_uid, *applied).await?;
            KnotControlMessage::Ack(KnotAck::Processed { event_id: event.id })
        }
        KnotControlMessage::ConfigRejected(rejected) => {
            record_config_rejected(pool, event.id, hardware_uid, rejected).await?;
            KnotControlMessage::Ack(KnotAck::Processed { event_id: event.id })
        }
        KnotControlMessage::Configure(_) => KnotControlMessage::Nack(KnotNack {
            event_id: event.id,
            reason: KnotNackReason::UnsupportedMessage,
        }),
        KnotControlMessage::Ack(_) | KnotControlMessage::Nack(_) => {
            return Ok(KnotProtocolEventResult {
                response: None,
                sensor_events: Vec::new(),
            });
        }
    };

    Ok(KnotProtocolEventResult {
        response: Some(response(response_event_id, responded_at, response_message)),
        sensor_events: Vec::new(),
    })
}

async fn record_config_applied(
    pool: &PgPool,
    event_id: EventId,
    hardware_uid: &str,
    applied: KnotConfigApplied,
) -> Result<()> {
    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin Knot config-applied transaction")?;
    let is_new = insert_knot_message_receipt(&mut *txn, event_id, hardware_uid, "config_applied")
        .await
        .map_err(message_store_error)?;

    if is_new {
        knot_store::update_station_knot_config_status(
            &mut *txn,
            hardware_uid,
            Some(applied.config_version as i64),
            "applied",
            None,
        )
        .await
        .map_err(|error| eyre::eyre!("failed to persist applied Knot config: {error:?}"))?;
    }

    txn.commit()
        .await
        .wrap_err("failed to commit Knot config-applied transaction")?;
    Ok(())
}

async fn record_config_rejected(
    pool: &PgPool,
    event_id: EventId,
    hardware_uid: &str,
    rejected: &KnotConfigRejected,
) -> Result<()> {
    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin Knot config-rejected transaction")?;
    let is_new = insert_knot_message_receipt(&mut *txn, event_id, hardware_uid, "config_rejected")
        .await
        .map_err(message_store_error)?;

    if is_new {
        knot_store::update_station_knot_config_status(
            &mut *txn,
            hardware_uid,
            None,
            "rejected",
            Some(&rejected.reason),
        )
        .await
        .map_err(|error| eyre::eyre!("failed to persist rejected Knot config: {error:?}"))?;
    }

    txn.commit()
        .await
        .wrap_err("failed to commit Knot config-rejected transaction")?;
    Ok(())
}

fn response(
    event_id: EventId,
    occurred_at: Timestamp,
    message: KnotControlMessage,
) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        ArkSyncActor::Hub {
            hub_id: *CONFIG.local_hub_id.as_bytes(),
        },
        occurred_at,
        KnotMessage::Control(message),
    )
}

fn message_store_error(error: KnotMessageStoreError) -> eyre::Report {
    eyre::eyre!("Knot message store error: {error:?}")
}
