use arksync_actuator::relay::{RelayDriver, RelayState, MIST_RELAY};
use tauri::{AppHandle, Emitter};
use tokio::time::{interval, Duration};

const RELAY_EVENT: &str = "relay_state_changed";
const RELAY_TICK_SECONDS: u64 = 5;

pub fn spawn_debug_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        log::info!(
            "Starting relay debug loop on GPIO{} (active-low={}), toggling every {}s.",
            MIST_RELAY.gpio_bcm_pin,
            MIST_RELAY.active_low,
            RELAY_TICK_SECONDS
        );

        let driver = RelayDriver::new(MIST_RELAY);
        let mut relay_is_active = false;
        let mut ticker = interval(Duration::from_secs(RELAY_TICK_SECONDS));

        loop {
            ticker.tick().await;

            relay_is_active = !relay_is_active;
            let state = RelayState::new(MIST_RELAY, relay_is_active);

            if let Err(error) = driver.apply(state) {
                log::error!(
                    "Failed to switch relay '{}' on GPIO{}: {error}",
                    MIST_RELAY.id,
                    MIST_RELAY.gpio_bcm_pin
                );
                continue;
            }

            log::info!(
                "Relay '{}' switched {} with {} level.",
                MIST_RELAY.id,
                if state.active { "ON" } else { "OFF" },
                state.level
            );

            if let Err(error) = app.emit(RELAY_EVENT, state) {
                log::error!("Failed to emit relay state event: {error}");
            }
        }
    });
}
