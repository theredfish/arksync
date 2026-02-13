// To see...
pub enum DriverError {
    Connection(String),
    UnknownDevice(String),
    Read(String),
    Write(String),
}

pub type Result<T> = std::result::Result<T, DriverError>;
