// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{vec, vec::Vec};
use arksync_bus::{EventId, Timestamp};
use arksync_protocol::knot::{
    KnotLiveMeasurement, KnotMeasurementQuality, KnotObservationMessage, KnotSensorInventory,
    KnotSensorObservationState,
};

pub const DEFAULT_OBSERVATION_CAPACITY: usize = 64;
pub const DEFAULT_INVENTORY_INTERVAL_MS: u64 = 5_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotObservationError {
    WrongSession,
    CapacityExceeded,
    InvalidInventory,
    ConflictingInventory,
    StaleInventory,
    MissingInventory,
    WrongInventoryRevision,
    UnknownSensor,
    InvalidMeasurement,
    StaleMeasurement,
}

/// Portable ordering guard for one established Knot session. It holds no measurements.
/// The receiver's trusted Hello selects the session; observations cannot change it.
pub struct KnotObservationCursor {
    session_id: EventId,
    capacity: usize,
    inventory: Option<KnotSensorInventory>,
    sequences: Vec<u64>,
}

impl KnotObservationCursor {
    pub fn new(session_id: EventId, capacity: usize) -> Self {
        Self {
            session_id,
            capacity,
            inventory: None,
            sequences: Vec::new(),
        }
    }

    pub fn session_id(&self) -> EventId {
        self.session_id
    }

    pub fn accept(&mut self, message: &KnotObservationMessage) -> Result<(), KnotObservationError> {
        if message.session_id() != self.session_id {
            return Err(KnotObservationError::WrongSession);
        }
        match message {
            KnotObservationMessage::Inventory(inventory) => self.accept_inventory(inventory),
            KnotObservationMessage::Measurement(measurement) => {
                self.accept_measurement(measurement)
            }
        }
    }

    fn accept_inventory(
        &mut self,
        inventory: &KnotSensorInventory,
    ) -> Result<(), KnotObservationError> {
        if inventory.sensors.len() > self.capacity {
            return Err(KnotObservationError::CapacityExceeded);
        }
        if inventory.revision == 0
            || inventory.sensors.iter().enumerate().any(|(i, sensor)| {
                inventory.sensors[..i]
                    .iter()
                    .any(|other| other.reference == sensor.reference)
                    || sensor
                        .sensor
                        .firmware
                        .is_some_and(|value| !value.is_finite())
            })
        {
            return Err(KnotObservationError::InvalidInventory);
        }
        if let Some(current) = &self.inventory {
            if inventory.revision < current.revision {
                return Err(KnotObservationError::StaleInventory);
            }
            if inventory.revision == current.revision {
                return if inventory == current {
                    Ok(())
                } else {
                    Err(KnotObservationError::ConflictingInventory)
                };
            }
        }
        self.sequences = vec![0; inventory.sensors.len()];
        self.inventory = Some(inventory.clone());
        Ok(())
    }

    fn accept_measurement(
        &mut self,
        measurement: &KnotLiveMeasurement,
    ) -> Result<(), KnotObservationError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(KnotObservationError::MissingInventory)?;
        if measurement.inventory_revision != inventory.revision {
            return Err(KnotObservationError::WrongInventoryRevision);
        }
        let index = inventory
            .sensors
            .iter()
            .position(|sensor| sensor.reference == measurement.sensor)
            .ok_or(KnotObservationError::UnknownSensor)?;
        let valid_value = match measurement.quality {
            KnotMeasurementQuality::Valid => {
                measurement.value.is_some()
                    && inventory.sensors[index].state == KnotSensorObservationState::Present
            }
            KnotMeasurementQuality::Invalid => true,
            KnotMeasurementQuality::Unavailable => measurement.value.is_none(),
        };
        if !valid_value
            || measurement.value.is_some_and(|value| !value.is_finite())
            || measurement.sequence == 0
        {
            return Err(KnotObservationError::InvalidMeasurement);
        }
        if measurement.sequence <= self.sequences[index] {
            return Err(KnotObservationError::StaleMeasurement);
        }
        self.sequences[index] = measurement.sequence;
        Ok(())
    }
}

/// One current inventory and at most one pending value per sensor. No ACK/retry queue.
/// A runner peeks, attempts transport, then marks sent only on success.
pub struct KnotObservationPublisher {
    cursor: KnotObservationCursor,
    inventory: Option<(Timestamp, KnotSensorInventory)>,
    measurements: Vec<(Timestamp, KnotLiveMeasurement)>,
    inventory_pending: bool,
    last_inventory_sent_ms: Option<u64>,
}

impl KnotObservationPublisher {
    pub fn new(session_id: EventId, capacity: usize) -> Self {
        Self {
            cursor: KnotObservationCursor::new(session_id, capacity),
            inventory: None,
            measurements: Vec::new(),
            inventory_pending: false,
            last_inventory_sent_ms: None,
        }
    }

    pub fn publish_at(
        &mut self,
        message: &KnotObservationMessage,
        occurred_at: Timestamp,
    ) -> Result<(), KnotObservationError> {
        self.cursor.accept(message)?;
        match message {
            KnotObservationMessage::Inventory(inventory) => {
                if self
                    .inventory
                    .as_ref()
                    .is_none_or(|(_, current)| current.revision != inventory.revision)
                {
                    self.measurements.clear();
                }
                self.inventory = Some((occurred_at, inventory.clone()));
                self.inventory_pending = true;
            }
            KnotObservationMessage::Measurement(measurement) => {
                if let Some(pending) = self
                    .measurements
                    .iter_mut()
                    .find(|(_, current)| current.sensor == measurement.sensor)
                {
                    *pending = (occurred_at, measurement.clone());
                } else {
                    self.measurements.push((occurred_at, measurement.clone()));
                }
            }
        }
        Ok(())
    }

    pub fn resynchronize(&mut self) {
        self.inventory_pending = true;
    }

    pub fn pending_measurement_count(&self) -> usize {
        self.measurements.len()
    }

    pub fn next_message(&self, now_ms: u64) -> Option<(Timestamp, KnotObservationMessage)> {
        if self.inventory_pending
            || self
                .last_inventory_sent_ms
                .is_none_or(|last| now_ms.saturating_sub(last) >= DEFAULT_INVENTORY_INTERVAL_MS)
        {
            if let Some((time, inventory)) = &self.inventory {
                return Some((*time, KnotObservationMessage::Inventory(inventory.clone())));
            }
        }
        self.measurements
            .first()
            .map(|(time, value)| (*time, KnotObservationMessage::Measurement(value.clone())))
    }

    pub fn mark_sent(&mut self, message: &KnotObservationMessage, now_ms: u64) {
        match message {
            KnotObservationMessage::Inventory(inventory) => {
                if self
                    .inventory
                    .as_ref()
                    .is_some_and(|(_, current)| current == inventory)
                {
                    self.inventory_pending = false;
                    self.last_inventory_sent_ms = Some(now_ms);
                }
            }
            KnotObservationMessage::Measurement(measurement) => {
                self.measurements.retain(|(_, current)| {
                    current.sensor != measurement.sensor
                        || current.sequence != measurement.sequence
                        || current.inventory_revision != measurement.inventory_revision
                });
            }
        }
    }
}
