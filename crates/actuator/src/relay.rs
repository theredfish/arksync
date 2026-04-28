use serde::Serialize;

pub const MIST_RELAY: RelaySpec = RelaySpec {
    id: "mist_relay",
    model: "5V dual-channel relay module",
    channels: 2,
    coil_voltage_vdc: 5.0,
    contact_current_a: 10.0,
    contact_type: "2NO 2NC",
    gpio_bcm_pin: 17,
    active_low: true,
};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelaySpec {
    pub id: &'static str,
    pub model: &'static str,
    pub channels: u8,
    pub coil_voltage_vdc: f32,
    pub contact_current_a: f32,
    pub contact_type: &'static str,
    pub gpio_bcm_pin: u8,
    pub active_low: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayState {
    pub id: &'static str,
    pub gpio_bcm_pin: u8,
    pub active_low: bool,
    pub active: bool,
    pub level: &'static str,
}

impl RelayState {
    pub fn new(spec: RelaySpec, active: bool) -> Self {
        Self {
            id: spec.id,
            gpio_bcm_pin: spec.gpio_bcm_pin,
            active_low: spec.active_low,
            active,
            level: match (spec.active_low, active) {
                (true, true) | (false, false) => "low",
                (true, false) | (false, true) => "high",
            },
        }
    }
}

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
pub struct RelayDriver {
    output_pin: std::sync::Mutex<rppal::gpio::OutputPin>,
}

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
impl RelayDriver {
    pub fn new(spec: RelaySpec) -> Self {
        use rppal::gpio::Gpio;

        let gpio = Gpio::new().unwrap_or_else(|error| {
            panic!("Failed to access Raspberry Pi GPIO controller: {error}");
        });

        let output_pin = gpio
            .get(spec.gpio_bcm_pin)
            .unwrap_or_else(|error| panic!("Failed to access GPIO{}: {error}", spec.gpio_bcm_pin))
            .into_output_high();

        Self {
            output_pin: std::sync::Mutex::new(output_pin),
        }
    }

    pub fn apply(&self, state: RelayState) -> Result<(), String> {
        let mut output_pin = self
            .output_pin
            .lock()
            .map_err(|error| format!("GPIO mutex poisoned: {error}"))?;

        match state.level {
            "low" => output_pin.set_low(),
            "high" => output_pin.set_high(),
            level => return Err(format!("Unsupported relay level: {level}")),
        }

        Ok(())
    }
}

#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
pub struct RelayDriver {
    spec: RelaySpec,
}

#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
impl RelayDriver {
    pub fn new(spec: RelaySpec) -> Self {
        log::warn!(
            "GPIO debug loop is running without Raspberry Pi GPIO access; state changes will only be logged."
        );
        Self { spec }
    }

    pub fn apply(&self, state: RelayState) -> Result<(), String> {
        log::debug!(
            "Simulated relay '{}' on GPIO{} -> {} ({})",
            state.id,
            self.spec.gpio_bcm_pin,
            if state.active { "ON" } else { "OFF" },
            state.level
        );

        Ok(())
    }
}
