use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::time::{interval, Duration};

const RELAY_NAME: &str = "mist_relay";
const RELAY_EVENT: &str = "relay_state_changed";
const RELAY_GPIO_BCM_PIN: u8 = 17;
const RELAY_TICK_SECONDS: u64 = 5;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayState {
    pub name: &'static str,
    pub gpio_bcm_pin: u8,
    pub active_low: bool,
    pub active: bool,
    pub level: &'static str,
}

impl RelayState {
    fn new(active: bool) -> Self {
        Self {
            name: RELAY_NAME,
            gpio_bcm_pin: RELAY_GPIO_BCM_PIN,
            active_low: true,
            active,
            level: if active { "low" } else { "high" },
        }
    }
}

pub fn spawn_debug_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        log::info!(
            "Starting relay debug loop on GPIO{} (active-low), toggling every {}s.",
            RELAY_GPIO_BCM_PIN,
            RELAY_TICK_SECONDS
        );

        let driver = RelayDriver::new(RELAY_GPIO_BCM_PIN);
        let mut relay_is_active = false;
        let mut ticker = interval(Duration::from_secs(RELAY_TICK_SECONDS));

        loop {
            ticker.tick().await;

            relay_is_active = !relay_is_active;
            let state = RelayState::new(relay_is_active);

            if let Err(error) = driver.apply(state) {
                log::error!(
                    "Failed to switch relay '{}' on GPIO{}: {error}",
                    RELAY_NAME,
                    RELAY_GPIO_BCM_PIN
                );
                continue;
            }

            log::info!(
                "Relay '{}' switched {} with {} level.",
                RELAY_NAME,
                if state.active { "ON" } else { "OFF" },
                state.level
            );

            if let Err(error) = app.emit(RELAY_EVENT, state) {
                log::error!("Failed to emit relay state event: {error}");
            }
        }
    });
}

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
struct RelayDriver {
    output_pin: std::sync::Mutex<rppal::gpio::OutputPin>,
}

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
impl RelayDriver {
    fn new(pin: u8) -> Self {
        use rppal::gpio::Gpio;

        let gpio = Gpio::new().unwrap_or_else(|error| {
            panic!("Failed to access Raspberry Pi GPIO controller: {error}");
        });

        let output_pin = gpio
            .get(pin)
            .unwrap_or_else(|error| panic!("Failed to access GPIO{pin}: {error}"))
            .into_output_high();

        Self {
            output_pin: std::sync::Mutex::new(output_pin),
        }
    }

    fn apply(&self, state: RelayState) -> Result<(), String> {
        let mut output_pin = self
            .output_pin
            .lock()
            .map_err(|error| format!("GPIO mutex poisoned: {error}"))?;

        if state.active {
            output_pin.set_low();
        } else {
            output_pin.set_high();
        }

        Ok(())
    }
}

#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
struct RelayDriver {
    pin: u8,
}

#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
impl RelayDriver {
    fn new(pin: u8) -> Self {
        log::warn!(
            "GPIO debug loop is running without Raspberry Pi GPIO access; state changes will only be logged."
        );
        Self { pin }
    }

    fn apply(&self, state: RelayState) -> Result<(), String> {
        log::debug!(
            "Simulated relay '{}' on GPIO{} -> {} ({})",
            state.name,
            self.pin,
            if state.active { "ON" } else { "OFF" },
            state.level
        );

        Ok(())
    }
}
