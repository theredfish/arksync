pub struct I2cDriver {
    address: u8,
    // Later replaced by I2c bus or similar from hal
    bus: String,
}
