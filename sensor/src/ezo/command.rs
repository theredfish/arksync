pub trait BaseCmd {
    fn baud();
    fn cal();
    fn i2c();
    fn information();
}

pub trait UartCmd: BaseCmd {}

pub trait I2cCmd: BaseCmd {}
