#![no_std]

//use ehal::blocking::i2c::WriteRead;
use core::fmt;

pub const ADDRESS: u8 = 0x76;

pub struct BMP280<I2C: ehal::blocking::i2c::WriteRead>
{
    com: I2C,
}

pub fn new<I2C, E>(i2c: I2C) -> Result<BMP280<I2C>, E>
where I2C: ehal::blocking::i2c::WriteRead<Error = E> {
    let mut chip = BMP280 {
        com: i2c,
    };

    Ok(chip)
}

impl<I2C: ehal::blocking::i2c::WriteRead> BMP280<I2C>
{
    pub fn set_control(&mut self, new: Control) {
        let osrs_t = (new.osrs_t as u8) << 5;
        let osrs_p = (new.osrs_p as u8) << 2;
        let control = osrs_t | osrs_p | (new.mode as u8);
        self.write_byte(Register::ctrl_meas, control);
    }

    pub fn control(&mut self) -> Control {
        let config = self.read_byte(Register::ctrl_meas);
        let osrs_t = match config & (0b111 << 5) >> 5 {
            x if x == Oversampling::skipped as u8 => Oversampling::skipped,
            x if x == Oversampling::x1 as u8 => Oversampling::x1,
            x if x == Oversampling::x2 as u8 => Oversampling::x2,
            x if x == Oversampling::x4 as u8 => Oversampling::x4,
            x if x == Oversampling::x8 as u8 => Oversampling::x8,
            _ => Oversampling::x16
        };
        let osrs_p = match config & (0b111 << 2) >> 2 {
            x if x == Oversampling::skipped as u8 => Oversampling::skipped,
            x if x == Oversampling::x1 as u8 => Oversampling::x1,
            x if x == Oversampling::x2 as u8 => Oversampling::x2,
            x if x == Oversampling::x4 as u8 => Oversampling::x4,
            x if x == Oversampling::x8 as u8 => Oversampling::x8,
            _ => Oversampling::x16
        };
        let mode = match config & 0b11 {
            x if x == PowerMode::Sleep as u8 => PowerMode::Sleep,
            x if x == PowerMode::Forced as u8 => PowerMode::Forced,
            x if x == PowerMode::Normal as u8 => PowerMode::Normal,
            _ => PowerMode::Forced
        };

        Control { osrs_t: osrs_t, osrs_p: osrs_p, mode: mode }
    }

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

#[derive(Debug)]
pub struct Control {
    pub osrs_t: Oversampling,
    pub osrs_p: Oversampling,
    pub mode: PowerMode
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

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum Oversampling {
    skipped = 0b000,
    x1 = 0b001,
    x2 = 0b010,
    x4 = 0b011,
    x8 = 0b100,
    x16 = 0b101
}

#[derive(Debug)]
pub enum PowerMode {
    Sleep = 0b00,
    Forced = 0b01,
    Normal = 0b11
}

#[allow(non_camel_case_types)]
enum Register {
    id = 0xD0,
    reset = 0xE0,
    status = 0xF3,
    ctrl_meas = 0xF4,
}
