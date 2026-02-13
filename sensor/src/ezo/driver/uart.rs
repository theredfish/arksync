use super::{DeviceInfo, DeviceType, Driver, DriverError, ReadWriteCmd, Result};
use crate::serial_port::SerialPort;
use crate::{ezo::driver::Status, serial_port::SerialPortConnection};

pub struct Uart {
    connection: SerialPortConnection,
}

impl Uart {
    pub fn new(serial_port: SerialPort) -> Result<Self> {
        let connection = SerialPortConnection::open(&serial_port)
            .map_err(|err| DriverError::Connection(err.to_string()))?;

        Ok(Uart { connection })
    }
}

impl ReadWriteCmd for Uart {
    fn read(&mut self) -> Result<String> {
        self.connection
            .read_until_carrier()
            .map_err(|err| DriverError::Read(err.to_string()))
    }

    fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.connection
            .write_command(buf)
            .map_err(|err| DriverError::Write(err.to_string()))
    }
}

impl Driver for Uart {
    /// Get device information (firmware version, device type)
    ///
    /// Retries up to 3 times if we get unexpected data (like temperature readings)
    fn device_info(&mut self) -> Result<DeviceInfo> {
        const MAX_RETRIES: usize = 3;

        for attempt in 1..=MAX_RETRIES {
            // Send "i" command to get device information
            self.write(b"i")?;
            let response = self.read()?;

            // Atlas Scientific response format: ?I,RTD,1.0
            // Format: ?I,<device_type>,<firmware_version>
            if response.starts_with("?I,") {
                let parts: Vec<&str> = response.trim_start_matches("?I,").split(',').collect();

                if parts.len() >= 2 {
                    return Ok(DeviceInfo {
                        device_type: DeviceType::try_from(parts[0])?,
                        firmware_version: parts[1].parse().unwrap_or(0.0),
                    });
                }
            }

            // Got unexpected response (possibly temperature reading or stale data)
            eprintln!(
                "Attempt {}/{}: Unexpected response to 'i' command: '{}' - retrying...",
                attempt, MAX_RETRIES, response
            );

            // Small delay before retry
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Err(DriverError::Read(
            "Failed to get valid device info after multiple attempts".to_string(),
        ))
    }

    fn status(&mut self) -> Result<Status> {
        self.write(b"Status")?;
        let response = self.read()?;

        // Atlas Scientific status response format: ?STATUS,<code>
        // Codes: P (powered off and restarted), S (software reset), B (brown out), W (watchdog), U (unknown)
        if response.starts_with("?STATUS,") {
            let code = response.chars().nth(8).unwrap_or('U');
            Ok(match code {
                'P' => Status::PoweredOn,
                'S' => Status::SoftwareReset,
                'B' => Status::BrownOut,
                'W' => Status::Watchdog,
                _ => Status::Unknown,
            })
        } else {
            Err(DriverError::Read(format!(
                "Unexpected response to 'Status' command: '{}'",
                response
            )))
        }
    }
}
