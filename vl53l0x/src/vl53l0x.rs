#![no_std]

//use ehal::blocking::i2c::WriteRead;

pub const ADDRESS: u8 = 0x29;

pub struct VL53L0x<I2C: ehal::blocking::i2c::WriteRead>
{
    com: I2C
}

pub fn new<I2C, E>(i2c: I2C) -> Result<VL53L0x<I2C>, E>
where I2C: ehal::blocking::i2c::WriteRead<Error = E> {
    let vl53l0x = VL53L0x {
        com: i2c
    };

    Ok(vl53l0x)
}

impl<I2C: ehal::blocking::i2c::WriteRead> VL53L0x<I2C>
{
    pub fn who_am_i(&mut self) -> u8 {
        let mut data: [u8; 1] = [0];
        let _some = self.com.write_read(ADDRESS, &[Register::WhoAmI as u8], &mut data);
        data[0]
    }
}

enum Register {
    WhoAmI = 0xC0
}
