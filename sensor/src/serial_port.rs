use serialport::{SerialPortInfo, SerialPortType};

pub fn find_atlas_sc_port() -> Vec<SerialPortInfo> {
    serialport::available_ports()
        .unwrap_or(Vec::new())
        .into_iter()
        .filter(|port| is_atlas_sc_device(port))
        .collect::<Vec<SerialPortInfo>>()
}

/// Checks if a port is an Atlas Scientific device
pub fn is_atlas_sc_device(port: &SerialPortInfo) -> bool {
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
