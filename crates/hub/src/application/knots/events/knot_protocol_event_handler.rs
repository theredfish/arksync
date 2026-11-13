// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_event_router::{EventEnvelope, EventId, Timestamp};
use arksync_protocol::knot::{
    KnotAck, KnotControlMessage, KnotEnvelope, KnotMessage, KnotPayload, KnotRejectionReason,
    KnotSensorMessage,
};
use arksync_protocol::ArkSyncActor;
use eyre::{Result, WrapErr};
use sqlx::PgPool;

use crate::application::register_knot_hello;
use crate::application::{HubSensorEventEnvelope, SensorRegistry};
use crate::config::CONFIG;
use crate::infrastructure::store::{
    claim_knot_message, knot as knot_store, save_knot_message_reply, KnotMessageClaim,
    KnotMessageStoreError,
};

use super::knot_sensor_protocol::{
    handle_knot_sensor_protocol, sensor_message_kind, InvalidSensorPayload,
};

pub struct KnotProtocolEventResult {
    pub response: Option<KnotEnvelope>,
    pub sensor_events: Vec<HubSensorEventEnvelope>,
    /// Accepted only after all durable effects have committed.
    pub accepted: bool,
    /// A received ACK whose result was committed; its request may now leave the outbox.
    pub acknowledged: Option<KnotAck>,
}

pub async fn handle_knot_protocol_event(
    pool: &PgPool,
    event: &KnotEnvelope,
    response_event_id: EventId,
    responded_at: Timestamp,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
) -> Result<KnotProtocolEventResult> {
    let reject = |reason| rejected(event, response_event_id, responded_at, reason);
    let ArkSyncActor::Knot { hardware_uid } = &event.source else {
        return Ok(reject(KnotRejectionReason::InvalidPayload));
    };
    let payload = event.payload.payload();
    if let Some(KnotPayload::Control(KnotControlMessage::Heartbeat)) = payload {
        return match knot_store::station_knot_by_hardware_uid(pool, hardware_uid).await {
            Ok(_) => Ok(accepted(event, response_event_id, responded_at)),
            Err(knot_store::KnotStoreError::NotFound) => {
                Ok(reject(KnotRejectionReason::InvalidPayload))
            }
            Err(error) => Err(eyre::eyre!(
                "failed to identify heartbeat source: {error:?}"
            )),
        };
    }
    if let Some(KnotPayload::Control(KnotControlMessage::Hello(hello))) = payload {
        if hello.hardware_uid != *hardware_uid {
            return Ok(reject(KnotRejectionReason::InvalidPayload));
        }
    }
    // Hello needs a configuration response; config results need durable confirmation.
    if matches!(event.payload, KnotMessage::Non(KnotPayload::Control(_))) {
        return Ok(reject(KnotRejectionReason::UnsupportedMessage));
    }
    if let Some(KnotPayload::Sensor(message)) = payload {
        let finite = match message {
            KnotSensorMessage::Provisioned(p) => p.sensor.firmware.is_none_or(f64::is_finite),
            KnotSensorMessage::ProvisioningConflict(p) => {
                p.sensor.firmware.is_none_or(f64::is_finite)
            }
            KnotSensorMessage::Measurements(batch) => batch
                .measurements
                .iter()
                .all(|m| m.value.is_finite() && m.sensor.firmware.is_none_or(f64::is_finite)),
            _ => true,
        };
        if !finite {
            return Ok(reject(KnotRejectionReason::InvalidPayload));
        }
    }
    let kind = match payload {
        Some(KnotPayload::Sensor(message)) => sensor_message_kind(message),
        Some(KnotPayload::Control(_)) => "control",
        None => "ack",
    };
    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin Knot protocol transaction")?;
    match claim_knot_message(&mut txn, event, kind)
        .await
        .map_err(message_store_error)?
    {
        KnotMessageClaim::Replay(reply) => {
            txn.rollback().await?;
            return Ok(KnotProtocolEventResult {
                response: reply,
                sensor_events: Vec::new(),
                accepted: true,
                acknowledged: match &event.payload {
                    KnotMessage::Ack(ack) => Some(ack.clone()),
                    _ => None,
                },
            });
        }
        KnotMessageClaim::Conflict => {
            txn.rollback().await?;
            return Ok(reject(KnotRejectionReason::ConfigurationConflict));
        }
        KnotMessageClaim::New => {}
    }
    let mut result = accepted(event, response_event_id, responded_at);
    let mut sensor_effects = None;
    match &event.payload {
        KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Hello(hello))) => {
            let config = register_knot_hello(&mut txn, hello)
                .await
                .wrap_err("failed to register Knot Hello")?;
            if !knot_store::insert_knot_config_delivery(&mut *txn, response_event_id, &config)
                .await?
            {
                return Err(eyre::eyre!(
                    "Hello configuration changed before it could be issued"
                ));
            }
            result.response = Some(response(
                response_event_id,
                responded_at,
                KnotAck::Hello {
                    event_id: event.id,
                    config,
                },
            ));
        }
        KnotMessage::Con(KnotPayload::Control(KnotControlMessage::ConfigApplied(applied))) => {
            if !knot_store::settle_knot_config_delivery(
                &mut txn,
                applied.event_id,
                hardware_uid,
                applied.config_version,
                None,
            )
            .await?
            {
                txn.rollback().await?;
                return Ok(reject(KnotRejectionReason::ConfigurationConflict));
            }
        }
        KnotMessage::Con(KnotPayload::Control(KnotControlMessage::ConfigRejected(rejected))) => {
            if !knot_store::settle_knot_config_delivery(
                &mut txn,
                rejected.event_id,
                hardware_uid,
                rejected.config_version,
                Some(&rejected.reason),
            )
            .await?
            {
                txn.rollback().await?;
                return Ok(reject(KnotRejectionReason::ConfigurationConflict));
            }
        }
        KnotMessage::Con(KnotPayload::Sensor(message))
        | KnotMessage::Non(KnotPayload::Sensor(message)) => {
            match handle_knot_sensor_protocol(
                &mut txn,
                event.id,
                hardware_uid,
                message,
                event.occurred_at,
                received_at,
                sensor_registry,
            )
            .await
            {
                Ok(effects) => sensor_effects = Some(effects),
                Err(error) if error.downcast_ref::<InvalidSensorPayload>().is_some() => {
                    txn.rollback().await?;
                    return Ok(reject(KnotRejectionReason::InvalidPayload));
                }
                Err(error) => return Err(error),
            }
        }
        KnotMessage::Ack(ack) => {
            let (version, rejection) = match ack {
                KnotAck::ConfigApplied { config_version, .. } => (*config_version, None),
                KnotAck::ConfigRejected {
                    config_version,
                    reason,
                    ..
                } => (*config_version, Some(reason.clone())),
                KnotAck::Rejected { reason, .. } => {
                    let version = knot_store::knot_config_delivery_version(
                        &mut *txn,
                        ack.event_id(),
                        hardware_uid,
                    )
                    .await?;
                    let Some(version) = version.and_then(|v| u64::try_from(v).ok()) else {
                        txn.rollback().await?;
                        return Ok(reject(KnotRejectionReason::ConfigurationConflict));
                    };
                    if reason.is_retryable() {
                        // No terminal receipt: this request is still pending and may succeed later.
                        txn.rollback().await?;
                        result.acknowledged = Some(ack.clone());
                        return Ok(result);
                    }
                    (version, Some(format!("{reason:?}")))
                }
                _ => {
                    txn.rollback().await?;
                    return Ok(reject(KnotRejectionReason::UnsupportedMessage));
                }
            };
            if !knot_store::settle_knot_config_delivery(
                &mut txn,
                ack.event_id(),
                hardware_uid,
                version,
                rejection.as_deref(),
            )
            .await?
            {
                txn.rollback().await?;
                return Ok(reject(KnotRejectionReason::ConfigurationConflict));
            }
            result.acknowledged = Some(ack.clone());
        }
        _ => {
            txn.rollback().await?;
            return Ok(reject(KnotRejectionReason::UnsupportedMessage));
        }
    }
    save_knot_message_reply(&mut *txn, event.id, result.response.as_ref()).await?;
    txn.commit()
        .await
        .wrap_err("failed to commit Knot protocol result")?;
    if let Some(effects) = sensor_effects {
        for (device_uid, sensor_id) in effects.remembered_sensors {
            sensor_registry.remember_sensor(effects.knot_id, device_uid, sensor_id);
        }
        result.sensor_events = effects.events;
    }
    Ok(result)
}

fn accepted(event: &KnotEnvelope, response_id: EventId, at: Timestamp) -> KnotProtocolEventResult {
    KnotProtocolEventResult {
        response: event
            .payload
            .requires_ack()
            .then(|| response(response_id, at, KnotAck::Processed { event_id: event.id })),
        sensor_events: Vec::new(),
        accepted: true,
        acknowledged: None,
    }
}

fn rejected(
    event: &KnotEnvelope,
    response_id: EventId,
    at: Timestamp,
    reason: KnotRejectionReason,
) -> KnotProtocolEventResult {
    KnotProtocolEventResult {
        response: event.payload.requires_ack().then(|| {
            response(
                response_id,
                at,
                KnotAck::Rejected {
                    event_id: event.id,
                    reason,
                },
            )
        }),
        sensor_events: Vec::new(),
        accepted: false,
        acknowledged: None,
    }
}

fn response(event_id: EventId, occurred_at: Timestamp, ack: KnotAck) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        ArkSyncActor::Hub {
            hub_id: *CONFIG.local_hub_id.as_bytes(),
        },
        occurred_at,
        KnotMessage::Ack(ack),
    )
}

fn message_store_error(error: KnotMessageStoreError) -> eyre::Report {
    eyre::eyre!("Knot message store error: {error:?}")
}
