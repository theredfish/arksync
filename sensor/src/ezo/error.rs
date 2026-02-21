use crate::ezo::driver::DriverError;
use std::fmt;

#[derive(Debug)]
pub enum SensorError {
    Driver(DriverError),
}

impl fmt::Display for SensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SensorError::Driver(err) => write!(f, "Driver error: {}", err),
        }
    }
}

impl std::error::Error for SensorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SensorError::Driver(err) => Some(err),
        }
    }
}

impl From<DriverError> for SensorError {
    fn from(err: DriverError) -> Self {
        SensorError::Driver(err)
    }
}

pub type Result<T> = std::result::Result<T, SensorError>;
