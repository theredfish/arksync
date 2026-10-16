// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{string::String, vec::Vec};
use arksync_bus::{EventId, Timestamp};
use serde::{Deserialize, Serialize};

use super::{KnotMeasurementUnit, KnotSensorDescriptor};

/// Best-effort observations, independent of sensor registration and durable receipts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotObservationMessage {
    Inventory(KnotSensorInventory),
    Measurement(KnotLiveMeasurement),
}

impl KnotObservationMessage {
    /// The EventId of the Hello that established this endpoint's session.
    pub fn session_id(&self) -> EventId {
        match self {
            Self::Inventory(inventory) => inventory.session_id,
            Self::Measurement(measurement) => measurement.session_id,
        }
    }
}

/// A Knot-local reference; interpreted only within a session and inventory revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotSensorRef(pub u32);

/// Complete inventory. Absence from a newer revision removes an observation.
/// Repeating the same revision must preserve its content and measurement ordering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotSensorInventory {
    pub session_id: EventId,
    pub revision: u64,
    pub sensors: Vec<KnotObservedSensor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotObservedSensor {
    pub reference: KnotSensorRef,
    /// None when no usable device UID is available; observation still works.
    pub device_uid: Option<String>,
    pub sensor: KnotSensorDescriptor,
    pub state: KnotSensorObservationState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotSensorObservationState {
    Present,
    Disconnected,
    Unavailable,
}

/// Sequence numbers increase per sensor within an inventory revision, starting at 1.
/// Acquisition time describes freshness, not ordering between messages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotLiveMeasurement {
    pub session_id: EventId,
    pub inventory_revision: u64,
    pub sensor: KnotSensorRef,
    pub sequence: u64,
    pub acquired_at: Timestamp,
    pub value: Option<f64>,
    pub unit: KnotMeasurementUnit,
    pub quality: KnotMeasurementQuality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotMeasurementQuality {
    Valid,
    /// A finite raw value may be retained for diagnostics, but is not a valid reading.
    Invalid,
    Unavailable,
}
