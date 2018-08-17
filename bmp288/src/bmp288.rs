#![no_std]

//use ehal::blocking::i2c::WriteRead;

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
    pub fn id(&mut self) -> u8 {
        self.read_byte(Register::ID)
    }

/*
    fn write_byte(&mut self, reg: Register, byte: u8) {
        let mut buffer = [0];
        let _ = self.com.write_read(ADDRESS, &[reg as u8, byte], &mut buffer);
    }
*/
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
    ID = 0xD0,
}
