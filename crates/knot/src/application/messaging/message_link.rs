// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Point-to-point asynchronous port used by runtime-specific message links.
///
/// The protocol state machine remains synchronous. A Tokio, Embassy, MQTT,
/// LoRa, or Zenoh adapter implements this port to move its envelopes.
#[allow(async_fn_in_trait)]
pub trait MessageLink<Message> {
    type Error;

    async fn send(&mut self, message: Message) -> Result<(), Self::Error>;
    async fn receive(&mut self) -> Result<Option<Message>, Self::Error>;
}
