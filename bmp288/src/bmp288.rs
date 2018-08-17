#![no_std]

//use ehal::blocking::i2c::WriteRead;
use core::fmt;

pub const ADDRESS: u8 = 0x76;

pub struct BMP288<I2C: ehal::blocking::i2c::WriteRead>
{
    com: I2C,
}

pub fn new<I2C, E>(i2c: I2C) -> Result<BMP288<I2C>, E>
where I2C: ehal::blocking::i2c::WriteRead<Error = E> {
    let mut chip = BMP288 {
        com: i2c,
    };

    Ok(chip)
}

impl<I2C: ehal::blocking::i2c::WriteRead> BMP288<I2C>
{
    pub fn status(&mut self) -> Status {
        let status = self.read_byte(Register::status);
        Status {
            measuring: (1 == status & 0b00001000),
            im_update: (1 == status & 0b00000001)
        }
    }

    pub fn id(&mut self) -> u8 {
        self.read_byte(Register::id)
    }

    /// Software reset, emulates POR
    pub fn reset(&mut self) {
        self.write_byte(Register::reset, 0xB6); // Magic from documentation
    }

    fn write_byte(&mut self, reg: Register, byte: u8) {
        let mut buffer = [0];
        let _ = self.com.write_read(ADDRESS, &[reg as u8, byte], &mut buffer);
    }

    fn read_byte(&mut self, reg: Register) -> u8 {
        let mut data: [u8; 1] = [0];
        let _ = self.com.write_read(ADDRESS, &[reg as u8], &mut data);
        data[0]
    }
}

pub struct Status {
    measuring: bool,
    im_update: bool
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "conversion is running: {}, NVM data being copied: {}",
            self.measuring, self.im_update)
    }
}

#[allow(non_camel_case_types)]
enum Register {
    id = 0xD0,
    reset = 0xE0,
    status = 0xF3,
}
