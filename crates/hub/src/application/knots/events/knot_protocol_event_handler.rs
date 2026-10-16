// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_protocol::knot::{
    KnotAck, KnotControlMessage, KnotEnvelope, KnotMessage, KnotNack, KnotNackReason,
};
use arksync_protocol::ArkSyncActor;
use eyre::{Result, WrapErr};
use sqlx::PgPool;

use crate::application::register_knot_hello;
use crate::application::{HubSensorEventEnvelope, SensorRegistry};
use crate::domain::HubId;
use crate::infrastructure::store::{
    insert_knot_message_receipt, knot as knot_store, KnotMessageStoreError,
};

use super::knot_sensor_protocol::{handle_knot_sensor_protocol, InvalidSensorPayload};

pub struct KnotProtocolEventResult {
    pub response: Option<KnotEnvelope>,
    pub sensor_events: Vec<HubSensorEventEnvelope>,
}

pub async fn handle_knot_protocol_event(
    pool: &PgPool,
    hub_id: HubId,
    event: &KnotEnvelope,
    response_event_id: EventId,
    responded_at: Timestamp,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
) -> Result<KnotProtocolEventResult> {
    let ArkSyncActor::Knot { .. } = &event.source else {
        return Ok(KnotProtocolEventResult {
            response: Some(response(
                hub_id,
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

    // Observation ordering belongs to the established session, never this durable handler.
    if matches!(event.payload, KnotMessage::Observation(_)) {
        return Ok(KnotProtocolEventResult {
            response: None,
            sensor_events: Vec::new(),
        });
    }

    let KnotMessage::Sensor(_) = &event.payload else {
        return handle_control_message(pool, hub_id, event, response_event_id, responded_at).await;
    };

    let result =
        handle_knot_sensor_protocol(pool, hub_id, event, received_at, sensor_registry).await;

    let (sensor_events, reply) = match result {
        Ok(events) => (
            events,
            KnotControlMessage::Ack(KnotAck::Processed { event_id: event.id }),
        ),
        Err(error) if error.downcast_ref::<InvalidSensorPayload>().is_some() => (
            Vec::new(),
            KnotControlMessage::Nack(KnotNack {
                event_id: event.id,
                reason: KnotNackReason::InvalidPayload,
            }),
        ),
        Err(error) => return Err(error),
    };

    Ok(KnotProtocolEventResult {
        response: Some(response(hub_id, response_event_id, responded_at, reply)),
        sensor_events,
    })
}

async fn handle_control_message(
    pool: &PgPool,
    hub_id: HubId,
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
                let config = match register_knot_hello(&mut txn, hub_id, hello).await {
                    Ok(config) => config,
                    Err(crate::application::HubKnotError::KnotStore(
                        knot_store::KnotStoreError::OwnershipConflict,
                    )) => {
                        txn.rollback().await?;
                        return Ok(KnotProtocolEventResult {
                            response: Some(response(
                                hub_id,
                                response_event_id,
                                responded_at,
                                KnotControlMessage::Nack(KnotNack {
                                    event_id: event.id,
                                    reason: KnotNackReason::ConfigurationConflict,
                                }),
                            )),
                            sensor_events: Vec::new(),
                        });
                    }
                    Err(error) => return Err(error.into()),
                };
                insert_knot_message_receipt(&mut *txn, hub_id, event.id, hardware_uid, "hello")
                    .await
                    .map_err(message_store_error)?;
                if !knot_store::insert_knot_config_delivery(
                    &mut *txn,
                    hub_id,
                    response_event_id,
                    &config,
                )
                .await?
                {
                    return Err(eyre::eyre!(
                        "Hello configuration changed before it could be issued"
                    ));
                }
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
            record_config_result(
                pool,
                hub_id,
                event,
                applied.event_id,
                applied.config_version,
                None,
            )
            .await?
        }
        KnotControlMessage::ConfigRejected(rejected) => {
            record_config_result(
                pool,
                hub_id,
                event,
                rejected.event_id,
                rejected.config_version,
                Some(&rejected.reason),
            )
            .await?
        }
        KnotControlMessage::Heartbeat => {
            match knot_store::knot_by_hardware_uid(pool, hub_id, hardware_uid).await {
                Ok(_) => KnotControlMessage::Ack(KnotAck::Heartbeat { event_id: event.id }),
                Err(knot_store::KnotStoreError::NotFound) => KnotControlMessage::Nack(KnotNack {
                    event_id: event.id,
                    reason: KnotNackReason::InvalidPayload,
                }),
                Err(error) => {
                    return Err(eyre::eyre!(
                        "failed to identify heartbeat source: {error:?}"
                    ))
                }
            }
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
        response: Some(response(
            hub_id,
            response_event_id,
            responded_at,
            response_message,
        )),
        sensor_events: Vec::new(),
    })
}

async fn record_config_result(
    pool: &PgPool,
    hub_id: HubId,
    event: &KnotEnvelope,
    config_event_id: EventId,
    version: u64,
    rejection: Option<&str>,
) -> Result<KnotControlMessage> {
    let ArkSyncActor::Knot { hardware_uid } = &event.source else {
        unreachable!()
    };
    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin Knot config-result transaction")?;
    let valid = knot_store::settle_knot_config_delivery(
        &mut txn,
        hub_id,
        config_event_id,
        hardware_uid,
        version,
        rejection,
    )
    .await?;
    if !valid {
        txn.rollback().await?;
        return Ok(KnotControlMessage::Nack(KnotNack {
            event_id: event.id,
            reason: KnotNackReason::ConfigurationConflict,
        }));
    }
    let kind = if rejection.is_some() {
        "config_rejected"
    } else {
        "config_applied"
    };
    let is_new = insert_knot_message_receipt(&mut *txn, hub_id, event.id, hardware_uid, kind)
        .await
        .map_err(message_store_error)?;
    if is_new {
        txn.commit()
            .await
            .wrap_err("failed to commit Knot config result")?;
    } else {
        txn.rollback().await?;
    }
    Ok(KnotControlMessage::Ack(KnotAck::Processed {
        event_id: event.id,
    }))
}

fn response(
    hub_id: HubId,
    event_id: EventId,
    occurred_at: Timestamp,
    message: KnotControlMessage,
) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        ArkSyncActor::Hub {
            hub_id: *hub_id.as_bytes(),
        },
        occurred_at,
        KnotMessage::Control(message),
    )
}

fn message_store_error(error: KnotMessageStoreError) -> eyre::Report {
    eyre::eyre!("Knot message store error: {error:?}")
}
