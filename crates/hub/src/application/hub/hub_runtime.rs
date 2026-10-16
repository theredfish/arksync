// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::{SystemTime, UNIX_EPOCH};

use arksync_bus::{EventEnvelope, EventHandlerError, EventId, Timestamp};
use arksync_knot::application::{
    local_tokio_message_link, KeepAliveConfig, RetryPolicy, TokioKnotRuntime,
    TokioKnotRuntimeConfig,
};
use arksync_protocol::knot::{KnotCapabilities, KnotControlMessage, KnotEnvelope, KnotMessage};
use arksync_protocol::ArkSyncActor;
use arksync_sensor::infrastructure::events::SensorEvent;
use eyre::{eyre, Result, WrapErr};
use tokio_util::sync::CancellationToken;

use crate::application::{
    ensure_local_demo_temperature_relay_rule, handle_actuator_runtime_event,
    knot_protocol_config_for_hardware_uid, HubSensorEventEnvelope, HubService, HubSession,
    HubSessionHandle,
};
use crate::config::CONFIG;
use crate::domain::HubId;
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
        let pool = arksync_db::pool();
        let hub_id = HubId::from(CONFIG.local_hub_id);
        let shutdown = CancellationToken::new();
        let _shutdown_on_drop = shutdown.clone().drop_guard();
        let (hub_link, knot_link) = local_tokio_message_link::<KnotEnvelope>(100);
        let mut session =
            HubSession::new(pool.clone(), hub_id, hub_link, KeepAliveConfig::default()).await?;
        let session_handle = session.handle();
        let (sensor_event_tx, mut sensor_event_rx) = tokio::sync::mpsc::channel(100);
        session.subscribe_sensors(move |event: &HubSensorEventEnvelope| {
            sensor_event_tx
                .try_send(event.clone())
                .map_err(|_| EventHandlerError::Rejected)
        });
        let mut session_task = tokio::spawn(session.run(shutdown.child_token()));
        let mut session_running = true;
        let (legacy_actuator_event_tx, mut legacy_actuator_event_rx) =
            tokio::sync::mpsc::channel(100);
        let knot_shutdown = shutdown.child_token();
        let knot_runtime = tokio::spawn(async move {
            TokioKnotRuntime::run_until(
                TokioKnotRuntimeConfig {
                    hardware_uid: CONFIG.local_knot_hardware_uid.clone(),
                    capabilities: local_knot_capabilities(),
                    retry_policy: RetryPolicy::default(),
                    keep_alive_config: KeepAliveConfig::default(),
                },
                knot_link,
                legacy_actuator_event_tx,
                knot_shutdown,
            )
            .await;
        });

        let mut hub = HubService::new();
        let mut local_demo_configured = false;

        loop {
            tokio::select! {
                Some(event) = sensor_event_rx.recv() => {
                    let received_at = timestamp_now();
                    maybe_configure_local_demo_actuator(pool, hub_id, &event, &session_handle, &mut local_demo_configured).await?;
                    hub.handle_sensor_event(event, received_at)
                        .map_err(|error| eyre!("Hub rejected projected sensor event: {error:?}"))?;
                }
                result = &mut session_task, if session_running => {
                    log::error!("Local Knot session stopped: {result:?}");
                    session_running = false;
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

        shutdown.cancel();
        if session_running {
            session_task
                .await
                .wrap_err("local Hub session task join failed")?;
        }
        knot_runtime
            .await
            .wrap_err("local Knot runtime task join failed")?;
        Ok(())
    }
}

async fn maybe_configure_local_demo_actuator(
    pool: &sqlx::PgPool,
    hub_id: HubId,
    event: &crate::application::HubSensorEventEnvelope,
    session: &HubSessionHandle,
    configured: &mut bool,
) -> Result<()> {
    if *configured {
        return Ok(());
    }
    let SensorEvent::SensorMeasurementRecorded(measurement) = &event.payload else {
        return Ok(());
    };
    let arksync_knot::domain::KnotEventSource::Knot {
        knot_id,
        hub_id: source_hub_id,
    } = &event.source;
    if source_hub_id.uuid_v4() != hub_id.uuid_v4() || knot_id.uuid_v4() != CONFIG.local_knot_id {
        return Ok(());
    }
    let sensor = crate::infrastructure::store::sensors::sensor_by_station_knot_id_and_device_uid(
        pool,
        knot_id.uuid_v4(),
        measurement.device_uid.as_ref(),
    )
    .await
    .map_err(|error| eyre!("failed to find local demo sensor: {error:?}"))?;
    let sensor_id = sensor.id.into();

    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin local demo actuator config transaction")?;
    ensure_local_demo_temperature_relay_rule(&mut txn, *knot_id, sensor_id)
        .await
        .wrap_err("failed to ensure local demo relay rule")?;
    knot_store::increment_station_knot_config_version(&mut *txn, hub_id, knot_id.uuid_v4())
        .await
        .map_err(|error| eyre!("failed to increment Knot config version: {error:?}"))?;
    let config =
        knot_protocol_config_for_hardware_uid(&mut txn, hub_id, &CONFIG.local_knot_hardware_uid)
            .await
            .wrap_err("failed to load refreshed local Knot config")?;
    txn.commit()
        .await
        .wrap_err("failed to commit local demo actuator config transaction")?;

    let configure = EventEnvelope::new_with_id(
        EventId::new(),
        ArkSyncActor::Hub {
            hub_id: *hub_id.as_bytes(),
        },
        timestamp_now(),
        KnotMessage::Control(KnotControlMessage::Configure(config)),
    );
    session
        .send(configure)
        .await
        .wrap_err("failed to queue local Knot configuration refresh")?;
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
