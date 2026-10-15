// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot::application::{ConnectionState, KeepAliveConfig, PeerPresence};

#[test]
fn presence_uses_session_time_and_closure_is_terminal() {
    let keep_alive_config = KeepAliveConfig::new(100, 300).unwrap();
    let mut presence = PeerPresence::new(10_000);
    presence.expire(10_299, keep_alive_config);
    assert_eq!(presence.state(), ConnectionState::Connecting);
    presence.expire(10_300, keep_alive_config);
    assert_eq!(presence.state(), ConnectionState::Unavailable);
    presence.observe(10_400);
    presence.expire(10_699, keep_alive_config);
    assert_eq!(presence.state(), ConnectionState::Connected);
    presence.expire(10_700, keep_alive_config);
    assert_eq!(presence.state(), ConnectionState::Unavailable);
    presence.observe(10_800);
    assert_eq!(presence.state(), ConnectionState::Connected);
    presence.close();
    presence.observe(10_900);
    presence.expire(11_200, keep_alive_config);
    assert_eq!(presence.state(), ConnectionState::Closed);
}

#[test]
fn keep_alive_requires_a_positive_interval_shorter_than_the_timeout() {
    assert_eq!(KeepAliveConfig::new(0, 300), None);
    assert_eq!(KeepAliveConfig::new(300, 300), None);
    assert_eq!(KeepAliveConfig::new(300, 100), None);
    assert!(KeepAliveConfig::new(100, 300).is_some());
}
