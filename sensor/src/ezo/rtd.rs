use crate::ezo::{driver::Driver, sensor::SensorData};

pub struct Rtd<D: Driver> {
    data: SensorData,
    driver: D,
}

// I'm trying to find a way to represent two different implementations but to organize
// nicely each sensor with a strongly typed representation.
//
// For example EZO RTD has common functions such as in BaseCmd. But also specific
// commands for uart (C: enable/disable continuous reading, *OK),
// and others for i2c.
//
// So we have some kind of same names for two modes, but some mode has something
// (uart) that the other (i2c) doesn't have.
//
// The same way is for pH EZO, but this sensors also had functions in uart
// that RTD doesn't have such has Slope (ph probe slope), pHext (extended ph scale) ...
//
// I was thinking of a system of trait to implement. But then how to implement both
// I2c and Uart: we can't. Or we would provide the behavior of one for the other. At
// first I thought about it because I could, by trait bound, inherit common commands.
//
// Then I was thinking that maybe I could use an enum Mode, and based on the mode
// passed to initialize; such as Rtd::uart() -> RtdUart or Rtd::i2c() -> RtdI2c
// that would be a factory pattern. Seems the best so far, I can find better.
//
// Then impl BaseUart for RtdUart; impl BaseI2c for RtdI2c
// Then impl RtdUart { fn continuous_reading(bool)... } for specific functions.
// But I find it a bit less great than one Rtd struct.
//
// Maybe another way, and since I want to make a library, would be to feature gate
// each module. Like one uart module and i2c module. For each sensor type? Then if you enable
// both features you could do uart::Rtd or i2c::Rtd. But could complexify name clashing issues.
//
// What's the best when it comes to polymorphism in Rust but with specifics based on the
// narrowed down type.

// TODO:
// - Define Rtd uart/i2c based on current implemented commands
// - Try driver style first then quickly switch if too complex
// - Just create todo impl for i2c to prepare
// - Manage sensor connection variant Uart/I2c
// - Must have a working state at then of the iteration
//
// Later:
//
// - Define impl Write/Read for Uart
// - Implement uart driver
// - Implement i2c driver https://docs.rs/embedded-hal/latest/embedded_hal/i2c/index.html
// - See to avoid Mutex on sensor connection?
