mod error;
pub mod i2c;
pub mod uart;

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

pub trait ReadWriteCmd {
    fn read(&mut self) -> Result<String>;
    fn write(&mut self, buf: &[u8]) -> Result<()>;
}

/// Commands common to both UART and I2C drivers.
pub trait Driver: ReadWriteCmd {
    fn device_info(&mut self) -> Result<DeviceInfo>;
    fn status(&mut self) -> Result<Status>;
}
