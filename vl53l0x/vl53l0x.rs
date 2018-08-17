//use ehal::blocking::i2c::WriteRead;

pub const ADDRESS: u8 = 0x29;

pub struct VL53L0x<I2C: ehal::blocking::i2c::WriteRead> {
    com: I2C,
    pub revision_id: u8,
}

pub fn new<I2C, E>(i2c: I2C) -> Result<VL53L0x<I2C>, E>
where
    I2C: ehal::blocking::i2c::WriteRead<Error = E>,
{
    let mut chip = VL53L0x {
        com: i2c,
        revision_id: 0x00,
    };

    if chip.who_am_i() == 0xEE {
        // FIXME: return an error/optional
        chip.set_high_i2c_voltage(); // TODO: make configurable
        chip.revision_id = chip.read_revision_id();
        chip.reset();
        chip.set_high_i2c_voltage();
        chip.set_standard_i2c_mode(); // TODO: make configurable
    }

    Ok(chip)
}

impl<I2C: ehal::blocking::i2c::WriteRead> VL53L0x<I2C> {
    pub fn set_standard_i2c_mode(&mut self) {
        self.write_byte_raw(0x80, 0x01);
        self.write_byte_raw(0xFF, 0x01);
        self.write_byte_raw(0x00, 0x00);
        let _ = self.read_byte_raw(0x91); // Stop byte? What's that?
        self.write_byte_raw(0x00, 0x01);
        self.write_byte_raw(0xFF, 0x00);
        self.write_byte_raw(0x80, 0x00);
    }

    pub fn reset(&mut self) {
        self.write_byte(Register::SoftResetGo2SoftResetN, 0x01);
    }

    pub fn read_revision_id(&mut self) -> u8 {
        self.read_byte(Register::IdentificationRevisionID)
    }

    pub fn who_am_i(&mut self) -> u8 {
        self.read_byte(Register::WhoAmI)
    }

    pub fn set_high_i2c_voltage(&mut self) {
        // Set i2c to 2.8 V
        let cfg = self.read_byte(Register::VHVConfigPadSCLSDAExtSupHV);
        self.write_byte(Register::VHVConfigPadSCLSDAExtSupHV, cfg | 0x01);
    }

    fn write_byte_raw(&mut self, reg: u8, byte: u8) {
        // FIXME:
        //  * remove this function
        //  * device address is not a const
        //  * register address is u16
        let mut buffer = [0];
        let _ = self.com.write_read(ADDRESS, &[reg, byte], &mut buffer);
    }

    fn read_byte_raw(&mut self, reg: u8) -> u8 {
        // FIXME:
        //  * remove this function
        //  * device address is not a const
        //  * register address is u16
        let mut data: [u8; 1] = [0];
        let _ = self.com.write_read(ADDRESS, &[reg], &mut data);
        data[0]
    }

    fn write_byte(&mut self, reg: Register, byte: u8) {
        let mut buffer = [0];
        let _ = self
            .com
            .write_read(ADDRESS, &[reg as u8, byte], &mut buffer);
    }

    fn read_byte(&mut self, reg: Register) -> u8 {
        let mut data: [u8; 1] = [0];
        // FIXME:
        //  * device address is not a const
        //  * register address is u16
        let _ = self.com.write_read(ADDRESS, &[reg as u8], &mut data);
        data[0]
    }
}

enum Register {
    WhoAmI = 0xC0,
    VHVConfigPadSCLSDAExtSupHV = 0x89,
    IdentificationRevisionID = 0xC2,
    SoftResetGo2SoftResetN = 0xBF,
}
