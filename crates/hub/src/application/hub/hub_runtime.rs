// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::{SystemTime, UNIX_EPOCH};

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    local_tokio_message_link, MessageLink, RetryPolicy, TokioKnotRuntime, TokioKnotRuntimeConfig,
};
use arksync_protocol::knot::{KnotCapabilities, KnotControlMessage, KnotEnvelope, KnotMessage};
use arksync_protocol::ArkSyncActor;
use arksync_sensor::infrastructure::events::SensorEvent;
use eyre::{eyre, Result, WrapErr};

use crate::application::{
    ensure_local_demo_temperature_relay_rule, handle_actuator_runtime_event,
    handle_knot_protocol_event, knot_protocol_config_for_hardware_uid, HubService, SensorRegistry,
};
use crate::config::CONFIG;
use crate::infrastructure::store::knot as knot_store;

/// Host runtime for the local Hub and its in-process Knot.
///
/// The Hub endpoint and Knot endpoint communicate through the same versioned
/// protocol used by future remote transports. This runtime owns Tokio, the
/// database pool, wall-clock time, and local task lifetimes. The portable Knot
/// state machine and protocol remain independent from those host concerns.
pub struct HubRuntime;

impl HubRuntime {
    /// Starts the Hub and logs any fatal orchestration error with its cause chain.
    pub async fn run() {
        if let Err(error) = Self::try_run().await {
            log::error!("Hub runtime failed: {error:?}");
        }
    }

    async fn try_run() -> Result<()> {
        let (mut hub_link, knot_link) = local_tokio_message_link::<KnotEnvelope>(100);
        let (legacy_actuator_event_tx, mut legacy_actuator_event_rx) =
            tokio::sync::mpsc::channel(100);
        let knot_runtime = tokio::spawn(async move {
            TokioKnotRuntime::run(
                TokioKnotRuntimeConfig {
                    hardware_uid: CONFIG.local_knot_hardware_uid.clone(),
                    capabilities: local_knot_capabilities(),
                    retry_policy: RetryPolicy::default(),
                },
                knot_link,
                legacy_actuator_event_tx,
            )
            .await;
        });

        let pool = arksync_db::pool();
        let mut hub = HubService::new();
        let mut sensor_registry = SensorRegistry::load(pool)
            .await
            .wrap_err("failed to load Hub sensor registry")?;
        let mut local_demo_configured = false;

        loop {
            tokio::select! {
                message = hub_link.receive() => {
                    let message = match message {
                        Ok(Some(message)) => message,
                        Ok(None) => {
                            log::error!("Local Knot protocol link closed");
                            break;
                        }
                        Err(error) => {
                            log::error!("Local Knot protocol link failed: {error:?}");
                            continue;
                        }
                    };
                    let received_at = timestamp_now();
                    match handle_knot_protocol_event(
                        pool,
                        &message,
                        EventId::new(),
                        timestamp_now(),
                        received_at,
                        &mut sensor_registry,
                    )
                    .await
                    {
                        Ok(result) => {
                            if let Some(response) = result.response {
                                if hub_link.send(response).await.is_err() {
                                    log::error!("Local Knot protocol link closed before Hub response");
                                    break;
                                }
                            }

                            for event in result.sensor_events {
                                maybe_configure_local_demo_actuator(
                                    pool,
                                    &event,
                                    &sensor_registry,
                                    &mut hub_link,
                                    &mut local_demo_configured,
                                )
                                .await?;
                                hub.handle_sensor_event(event, received_at)
                                    .map_err(|error| eyre!("Hub rejected projected sensor event: {error:?}"))?;
                            }
                        }
                        Err(error) => {
                            log::error!("Hub failed to process Knot protocol event: {error:?}");
                        }
                    }
                }
                Some(event) = legacy_actuator_event_rx.recv() => {
                    let arksync_knot::application::LegacyKnotActuatorMessage::Actuator(event) = event.payload else {
                        log::debug!("Hub ignored non-actuator message on legacy actuator path");
                        continue;
                    };

                    if let Err(error) = handle_actuator_runtime_event(pool, event).await {
                        log::error!("Hub failed to process legacy actuator event: {error:?}");
                    }
                }
                else => break,
            }
        }

        knot_runtime
            .await
            .wrap_err("local Knot runtime task join failed")?;
        Ok(())
    }
}

async fn maybe_configure_local_demo_actuator(
    pool: &sqlx::PgPool,
    event: &crate::application::HubSensorEventEnvelope,
    sensor_registry: &SensorRegistry,
    hub_link: &mut arksync_knot::application::TokioMessageLink<KnotEnvelope>,
    configured: &mut bool,
) -> Result<()> {
    if *configured {
        return Ok(());
    }
    let SensorEvent::SensorMeasurementRecorded(measurement) = &event.payload else {
        return Ok(());
    };
    let arksync_knot::domain::KnotEventSource::Knot { knot_id, .. } = &event.source;
    if knot_id.uuid_v4() != CONFIG.local_knot_id {
        return Ok(());
    }
    let Some(sensor_id) =
        sensor_registry.sensor_id(knot_id.uuid_v4(), measurement.device_uid.as_ref())
    else {
        return Ok(());
    };

    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin local demo actuator config transaction")?;
    ensure_local_demo_temperature_relay_rule(&mut txn, *knot_id, sensor_id)
        .await
        .wrap_err("failed to ensure local demo relay rule")?;
    knot_store::increment_station_knot_config_version(&mut *txn, knot_id.uuid_v4())
        .await
        .map_err(|error| eyre!("failed to increment Knot config version: {error:?}"))?;
    let config = knot_protocol_config_for_hardware_uid(&mut txn, &CONFIG.local_knot_hardware_uid)
        .await
        .wrap_err("failed to load refreshed local Knot config")?;
    txn.commit()
        .await
        .wrap_err("failed to commit local demo actuator config transaction")?;

    let configure = EventEnvelope::new_with_id(
        EventId::new(),
        ArkSyncActor::Hub {
            hub_id: *CONFIG.local_hub_id.as_bytes(),
        },
        timestamp_now(),
        KnotMessage::Control(KnotControlMessage::Configure(config)),
    );
    hub_link
        .send(configure)
        .await
        .map_err(|_| eyre!("local Knot protocol link closed before config refresh"))?;
    *configured = true;

    Ok(())
}

fn local_knot_capabilities() -> KnotCapabilities {
    KnotCapabilities {
        gpio: true,
        uart: true,
        i2c: false,
        atlas_scientific_ezo: true,
    }
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}
