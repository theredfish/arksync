use serialport::{SerialPortInfo, SerialPortType};
use std::io::{self, Read, Write};
use std::time::Duration;

// Atlas Scientific RTD Sensor Configuration
// Based on datasheet specifications
pub const RTD_BAUD_RATE: u32 = 9600;
pub const RTD_TIMEOUT_MS: u64 = 1000; // Timeout acts as safety net for response-based reading

/// Metadata about a serial port (no active connection)
#[derive(Debug, Clone)]
pub struct SerialPort {
    pub port_name: String,
    pub serial_number: String,
}

/// Active serial port connection for communication
pub struct SerialPortConnection {
    port: Box<dyn serialport::SerialPort>,
}

impl SerialPortConnection {
    /// Open a serial port connection with Atlas Scientific RTD defaults
    /// - Baud rate: 9600
    /// - Timeout: 1000ms (safety net for unresponsive sensors)
    /// - Encoding: ASCII
    /// - Terminator: Carriage return (\r)
    /// - Decimal places: 3
    /// - Temperature unit: Celsius (default)
    pub fn open(port_info: &SerialPort) -> Result<Self, serialport::Error> {
        let port = serialport::new(&port_info.port_name, RTD_BAUD_RATE)
            .timeout(Duration::from_millis(RTD_TIMEOUT_MS))
            .open()?;

        let mut port = port;

        // Flush any stale data in the input buffer
        // Atlas Scientific sensors might have leftover readings or responses
        let _ = port.clear(serialport::ClearBuffer::Input);

        Ok(Self { port })
    }

    /// Write a command to the sensor
    pub fn write_command(&mut self, command: &str) -> std::io::Result<()> {
        self.port.write_all(command.as_bytes())?;
        self.port.write_all(b"\r")?; // Atlas Scientific expects carriage return
        self.port.flush()?;
        Ok(())
    }

    /// Read response from the sensor until carriage return terminator
    ///
    /// Atlas Scientific sensors terminate responses with \r
    /// This method blocks until \r is received or timeout occurs
    pub fn read_response(&mut self) -> std::io::Result<String> {
        let mut buffer = Vec::new();
        let mut single_byte = [0u8; 1];

        // Read byte-by-byte until carriage return
        loop {
            match self.port.read_exact(&mut single_byte) {
                Ok(_) => {
                    if single_byte[0] == b'\r' {
                        break;
                    }
                    buffer.push(single_byte[0]);
                }
                Err(e) => return Err(e),
            }
        }

        // Convert to string and trim whitespace
        let response = String::from_utf8_lossy(&buffer).trim().to_string();

        Ok(response)
    }

    /// Flush the input buffer to clear any stale data
    pub fn flush_input(&mut self) -> std::io::Result<()> {
        self.port
            .clear(serialport::ClearBuffer::Input)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Send a command and read the response
    ///
    /// Flushes input buffer first to clear stale data, then sends command and waits for response.
    /// The serialport timeout (1000ms) acts as a safety net if the sensor doesn't respond.
    ///
    /// # Arguments
    /// * `command` - The command string to send
    pub fn send_command(&mut self, command: &str) -> io::Result<String> {
        // Flush any stale data before sending command
        self.flush_input()?;
        self.write_command(command)?;
        self.read_response()
    }
}

impl std::fmt::Debug for SerialPortConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialPortConnection")
            .field("port", &"<open connection>")
            .finish()
    }
}

pub fn find_asc_port() -> Vec<SerialPort> {
    serialport::available_ports()
        .unwrap_or(Vec::new())
        .into_iter()
        .filter(filter_asc_device)
        .filter_map(filter_map_usb_serial)
        .collect::<Vec<SerialPort>>()
}

/// Checks if a port is an Atlas Scientific device
fn filter_asc_device(port: &SerialPortInfo) -> bool {
    match &port.port_type {
        SerialPortType::UsbPort(usb_info) => {
            // Atlas Scientific USB devices typically use FTDI chips
            // FTDI Vendor ID: 0x0403
            // Common Product IDs: 0x6001 (FT232), 0x6015 (FT231X)
            usb_info.vid == 0x0403 && (usb_info.pid == 0x6001 || usb_info.pid == 0x6015)
        }
        _ => false,
    }
}

fn filter_map_usb_serial(port: SerialPortInfo) -> Option<SerialPort> {
    let SerialPortInfo {
        port_name,
        port_type,
    } = port;

    let SerialPortType::UsbPort(usb_port) = port_type else {
        return None;
    };

    usb_port.serial_number.map(|serial_number| SerialPort {
        port_name,
        serial_number,
    })
}

// We need to maintain a list of opened serial ports
//
// DP065KS3 + serial port
// DK0HFBFB + serial port
// pub fn collect_atlas_sc_sensors(serial_ports: Vec<SerialPortInfo>) -> Vec<Sensor> {
//     serial_ports.iter().map(|serial_port| {
//         serial_port.
//     }).collect()
// }
