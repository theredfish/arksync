//! UART commands for Atlas Scientific sensors
//!
//! # Example Usage
//!
//! ```no_run
//! use arksync_sensor::commands::UartCommand;
//! use arksync_sensor::serial_port::SerialPort;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create port metadata
//! let port = SerialPort {
//!     port_name: "/dev/ttyUSB0".to_string(),
//!     serial_number: "DP065KS3".to_string(),
//! };
//!
//! // Connect to the sensor
//! let mut uart = UartCommand::connect(port)?;
//!
//! // Get device information
//! let info = uart.device_info()?;
//! println!("Device: {} v{}", info.device_type, info.firmware_version);
//!
//! // Read temperature
//! let temp = uart.read_temperature()?;
//! println!("Temperature: {:.2}°C", temp);
//!
//! // Check status
//! let status = uart.check_status()?;
//! println!("Status: {:?}", status);
//!
//! // Get calibration info
//! let cal = uart.get_calibration()?;
//! println!("Calibration points: {}", cal.calibration_points);
//! # Ok(())
//! # }
//! ```

use crate::serial_port::{SerialPort, SerialPortConnection};
use std::io;

#[derive(Debug)]
pub struct UartCommand {
    connection: SerialPortConnection,
}

impl UartCommand {
    /// Connect to a sensor via UART
    pub fn connect(port: SerialPort) -> Result<Self, serialport::Error> {
        let connection = SerialPortConnection::open(&port)?;
        Ok(Self { connection })
    }

    /// Create a UartCommand from an already-open connection
    pub fn connect_with_connection(
        connection: SerialPortConnection,
    ) -> Result<Self, serialport::Error> {
        Ok(Self { connection })
    }

    /// Get device information (firmware version, device type)
    ///
    /// Retries up to 3 times if we get unexpected data (like temperature readings)
    pub fn device_info(&mut self) -> io::Result<DeviceInfo> {
        const MAX_RETRIES: usize = 3;

        for attempt in 1..=MAX_RETRIES {
            // Send "i" command to get device information
            let response = self.connection.send_command("i")?;

            // Atlas Scientific response format: ?I,RTD,1.0
            // Format: ?I,<device_type>,<firmware_version>
            if response.starts_with("?I,") {
                let parts: Vec<&str> = response.trim_start_matches("?I,").split(',').collect();

                if parts.len() >= 2 {
                    return Ok(DeviceInfo {
                        device_type: parts[0].to_string(),
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

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to get valid device info after multiple attempts",
        ))
    }

    /// Get current temperature reading
    ///
    /// Sends "R" command and waits for response (terminated by \r)
    /// Sensor responds when measurement is complete (~1 second per reading max)
    pub fn read_temperature(&mut self) -> io::Result<f64> {
        let response = self.connection.send_command("R")?;

        response
            .trim()
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Check if sensor is responsive (return status code)
    pub fn check_status(&mut self) -> io::Result<StatusCode> {
        let response = self.connection.send_command("Status")?;

        // Atlas Scientific status response format: ?STATUS,<code>
        // Codes: P (powered off and restarted), S (software reset), B (brown out), W (watchdog), U (unknown)
        if response.starts_with("?STATUS,") {
            let code = response.chars().nth(8).unwrap_or('U');
            Ok(match code {
                'P' => StatusCode::PoweredOn,
                'S' => StatusCode::SoftwareReset,
                'B' => StatusCode::BrownOut,
                'W' => StatusCode::Watchdog,
                _ => StatusCode::Unknown,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid status response: {}", response),
            ))
        }
    }

    /// Set sensor to sleep mode (low power)
    pub fn sleep(&mut self) -> io::Result<()> {
        self.connection.send_command("Sleep")?;
        Ok(())
    }

    /// Get sensor calibration status
    pub fn get_calibration(&mut self) -> io::Result<CalibrationStatus> {
        let response = self.connection.send_command("Cal,?")?;

        // Response format: ?Cal,<number of calibration points>
        if response.starts_with("?Cal,") {
            let points: u8 = response
                .trim_start_matches("?Cal,")
                .trim()
                .parse()
                .unwrap_or(0);

            Ok(CalibrationStatus {
                calibration_points: points,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid calibration response: {}", response),
            ))
        }
    }

    /// Send raw command (for testing/debugging)
    pub fn send_raw(&mut self, command: &str) -> io::Result<String> {
        self.connection.send_command(command)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: String,
    pub firmware_version: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    PoweredOn,
    SoftwareReset,
    BrownOut,
    Watchdog,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CalibrationStatus {
    pub calibration_points: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_parsing() {
        // This would require a mock connection for proper testing
        // For now, this is a placeholder for future integration tests
    }
}
