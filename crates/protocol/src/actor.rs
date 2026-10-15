// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use arksync_bus::EventEnvelope;
use serde::{Deserialize, Serialize};

/// Actor that claims authorship of an ArkSync protocol message.
///
/// Remote transports authenticate their peers independently. This identity
/// remains useful for routing, auditing, and in-process message links.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArkSyncActor {
    Hub { hub_id: [u8; 16] },
    Knot { hardware_uid: String },
}

/// Common envelope used by actor-specific ArkSync message contracts.
pub type ArkSyncEnvelope<Message> = EventEnvelope<Message, ArkSyncActor>;
