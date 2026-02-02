use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

use crate::commands::UartCommand;
use crate::serial_port::{SerialPort, SerialPortConnection};

#[derive(Debug, Clone, Copy)]
pub enum SensorKind {
    Rtd,
}

#[derive(Debug, Clone, Default)]
pub enum SensorName {
    #[default]
    Unnamed,
    Named(String),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum SensorState {
    Active,
    Degraded,
    #[default]
    Initializing,
    Unreachable,
}

#[derive(Debug, Clone)]
pub enum SensorType {
    I2c { addr: String },
    Uart { port: SerialPort },
}

#[derive(Debug, Clone)]
pub struct Sensor {
    pub kind: SensorKind,
    pub firmware: f64,
    pub name: SensorName,
    pub state: SensorState,
    pub sensor_type: SensorType,
    pub connection: Option<Arc<Mutex<UartCommand>>>,
    pub last_activity: DateTime<Utc>,
}

impl Sensor {
    /// Create a new sensor by connecting to a serial port and querying device info
    ///
    /// This method:
    /// 1. Opens a connection to the serial port
    /// 2. Queries device information (type, firmware)
    /// 3. Creates a Sensor with the retrieved information
    ///
    /// # Arguments
    /// * `port` - Serial port metadata (port name and serial number)
    ///
    /// # Returns
    /// * `Ok(Sensor)` - Successfully connected and queried sensor
    /// * `Err` - Failed to connect or query device
    pub fn from_device(port: SerialPort) -> Result<Self, Box<dyn std::error::Error>> {
        // Open connection
        let connection = SerialPortConnection::open(&port)?;
        let mut uart = UartCommand::connect_with_connection(connection)?;

        // Query device information
        let device_info = uart.device_info()?;

        // Parse device type to SensorKind
        let kind = match device_info.device_type.as_str() {
            "RTD" => SensorKind::Rtd,
            _ => SensorKind::Rtd, // Default to RTD for now
        };

        // Wrap connection for sharing
        let shared_connection = Arc::new(Mutex::new(uart));

        Ok(Sensor {
            kind,
            firmware: device_info.firmware_version,
            name: SensorName::Unnamed, // Default to unnamed, can be set later
            state: SensorState::Initializing,
            sensor_type: SensorType::Uart { port },
            connection: Some(shared_connection),
            last_activity: Utc::now(),
        })
    }

    /// Check if sensor has an active connection
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Get a reference to the UART command interface
    ///
    /// Returns None if no connection is established
    pub fn get_uart(&self) -> Option<Arc<Mutex<UartCommand>>> {
        self.connection.clone()
    }
}

// TODO: update the sensor module to organize the lib per different sensors.
// Start with RTD sensor, and then expand for each type of sensor
//
// See to have Read and Write types
// command => u8
// features: i2c, uart
// i2c => later see for embedded_hal compatibility
// Rename lib into arksync_ezo (since we capture from ezo circuit)
// See https://github.com/RougeEtoile/ezo_i2c_rs
