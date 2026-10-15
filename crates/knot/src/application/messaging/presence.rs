// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Connectivity of one session, independent from its persisted identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionState {
    #[default]
    Connecting,
    Connected,
    Unavailable,
    Closed,
}

/// Application keep-alive timing supplied by the host or embedded runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeepAliveConfig {
    interval_ms: u64,
    timeout_ms: u64,
}

impl KeepAliveConfig {
    pub fn new(interval_ms: u64, timeout_ms: u64) -> Option<Self> {
        (interval_ms > 0 && timeout_ms > interval_ms).then_some(Self {
            interval_ms,
            timeout_ms,
        })
    }

    pub fn interval_ms(self) -> u64 {
        self.interval_ms
    }

    pub fn timeout_ms(self) -> u64 {
        self.timeout_ms
    }
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5_000,
            timeout_ms: 15_000,
        }
    }
}

/// Pure presence tracking using elapsed monotonic milliseconds, never wall time.
#[derive(Clone, Copy, Debug, Default)]
pub struct PeerPresence {
    state: ConnectionState,
    last_seen_ms: Option<u64>,
}

impl PeerPresence {
    pub fn new(now_ms: u64) -> Self {
        Self {
            state: ConnectionState::Connecting,
            last_seen_ms: Some(now_ms),
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn observe(&mut self, now_ms: u64) {
        if self.state == ConnectionState::Closed {
            return;
        }
        self.last_seen_ms = Some(now_ms);
        self.state = ConnectionState::Connected;
    }

    pub fn expire(&mut self, now_ms: u64, keep_alive_config: KeepAliveConfig) {
        if self.state != ConnectionState::Closed
            && now_ms.saturating_sub(self.last_seen_ms.unwrap_or(0))
                >= keep_alive_config.timeout_ms()
        {
            self.state = ConnectionState::Unavailable;
        }
    }

    pub fn close(&mut self) {
        self.state = ConnectionState::Closed;
    }
}
