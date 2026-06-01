// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod error;
pub mod i2c;
pub mod uart;

use crate::sensor::SensorConnection;
use std::time::Duration;

pub use self::error::*;

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Rtd,
}

impl TryFrom<&str> for DeviceType {
    type Error = DriverError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "RTD" => Ok(DeviceType::Rtd),
            other => Err(DriverError::UnknownDevice(other.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub firmware_version: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    PoweredOn,
    SoftwareReset,
    BrownOut,
    Watchdog,
    Unknown,
}

pub trait CommandTransport {
    fn read(&mut self) -> Result<String>;
    fn write(&mut self, buf: &[u8]) -> Result<()>;

    fn send_command(&mut self, command: &[u8]) -> Result<String> {
        self.write(command)?;
        self.read()
    }
}

/// Commands common to both UART and I2C drivers.
pub trait Driver: CommandTransport {
    fn connection_info(&self) -> SensorConnection;
    fn device_info(&mut self) -> Result<DeviceInfo>;
    fn device_name(&mut self) -> Result<Option<String>> {
        let response = self.send_command(b"Name,?")?;

        parse_name_response(&response)
    }

    fn set_device_name(&mut self, device_name: &str) -> Result<()> {
        let command = format!("Name,{device_name}");
        self.send_command(command.as_bytes())?;
        std::thread::sleep(Duration::from_millis(300));

        Ok(())
    }

    fn clear_device_name(&mut self) -> Result<()> {
        self.send_command(b"Name,")?;
        std::thread::sleep(Duration::from_millis(300));

        Ok(())
    }

    fn status(&mut self) -> Result<Status>;
}

fn parse_name_response(response: &str) -> Result<Option<String>> {
    let response = response.trim();

    let name = response
        .strip_prefix("?NAME,")
        .or_else(|| response.strip_prefix("?Name,"))
        .or_else(|| response.strip_prefix("?name,"))
        .unwrap_or(response)
        .trim();

    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_atlas_name_response() {
        assert_eq!(
            parse_name_response("?Name,RTD_ABC123").unwrap(),
            Some("RTD_ABC123".to_string())
        );
        assert_eq!(parse_name_response("?Name,").unwrap(), None);
    }
}
