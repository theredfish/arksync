use std::fmt;

#[allow(unused)]
#[derive(Debug)]
pub enum DriverError {
    Connection(String),
    UnknownDevice(String),
    Read(String),
    Write(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::Connection(msg) => write!(f, "Connection error: {}", msg),
            DriverError::UnknownDevice(msg) => write!(f, "Unknown device: {}", msg),
            DriverError::Read(msg) => write!(f, "Read error: {}", msg),
            DriverError::Write(msg) => write!(f, "Write error: {}", msg),
        }
    }
}

impl std::error::Error for DriverError {}

pub type Result<T> = std::result::Result<T, DriverError>;
