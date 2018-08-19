#![no_std]

//use ehal::blocking::i2c::WriteRead;
use core::fmt;

pub const ADDRESS: u8 = 0x76;

pub struct BMP280<I2C: ehal::blocking::i2c::WriteRead>
{
    com: I2C,
    // Temperature compensation
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    t_fine: i32
}

pub fn new<I2C, E>(i2c: I2C) -> Result<BMP280<I2C>, E>
where I2C: ehal::blocking::i2c::WriteRead<Error = E> {
    let mut chip = BMP280 {
        com: i2c,
        dig_t1: 0,
        dig_t2: 0,
        dig_t3: 0,
        t_fine: 0
    };

    if chip.id() == 0x58 {
        chip.read_calibration();
    }

    Ok(chip)
}

impl<I2C: ehal::blocking::i2c::WriteRead> BMP280<I2C>
{
    fn read_calibration(&mut self) {
        let mut data: [u8; 24] = [0; 24];
        let _ = self.com.write_read(ADDRESS, &[Register::calib00 as u8], &mut data);

        self.dig_t1 = ((data[1] as u16) << 8) | (data[0] as u16);
        self.dig_t2 = ((data[3] as i16) << 8) | (data[2] as i16);
        self.dig_t3 = ((data[5] as i16) << 8) | (data[4] as i16);
    }

    pub fn read_temp(&mut self) -> f64 {
        let mut data: [u8; 6] = [0, 0, 0, 0, 0, 0];
        let _ = self.com.write_read(ADDRESS, &[Register::press as u8], &mut data);
        let _pres = (data[0] as u32) << 12 | (data[1] as u32) << 4
            | (data[2] as u32) >> 4;
        let temp = (data[3] as u32) << 12 | (data[4] as u32) << 4
            | (data[5] as u32) >> 4;

        let v1 = ((temp as f64) / 16384.0 - (self.dig_t1 as f64) / 1024.0)
                * (self.dig_t2 as f64);
        let v2 = (((temp as f64) / 131072.0 - (self.dig_t1 as f64) / 8192.0)
                * ((temp as f64) / 131072.0 - (self.dig_t1 as f64) / 8192.0))
                * (self.dig_t3 as f64);
        self.t_fine = (v1 + v2) as i32;

        ((v1 + v2) / 5120.0)
    }

    pub fn config(&mut self) -> Config {
        let config = self.read_byte(Register::config);
        let t_sb = match (config & (0b111 << 5)) >> 5 {
            x if x == Standby::ms0_5 as u8 => Standby::ms0_5,
            x if x == Standby::ms62_5 as u8 => Standby::ms62_5,
            x if x == Standby::ms125 as u8 => Standby::ms125,
            x if x == Standby::ms250 as u8 => Standby::ms250,
            x if x == Standby::ms500 as u8 => Standby::ms500,
            x if x == Standby::ms1000 as u8 => Standby::ms1000,
            x if x == Standby::ms2000 as u8 => Standby::ms2000,
            x if x == Standby::ms4000 as u8 => Standby::ms4000,
            _ => Standby::unknown
        };
        let filter = match (config & (0b111 << 2)) >> 2 {
            x if x == Filter::off as u8 => Filter::off,
            x if x == Filter::c2 as u8 => Filter::c2,
            x if x == Filter::c4 as u8 => Filter::c4,
            x if x == Filter::c8 as u8 => Filter::c8,
            x if x == Filter::c16 as u8 => Filter::c16,
            _ => Filter::unknown
        };
        Config { t_sb: t_sb, filter: filter }
    }

    pub fn set_config(&mut self, new: Config) {
        let config: u8 = 0x00;
        let t_sb = (new.t_sb as u8) << 5;
        let filter = (new.filter as u8) << 2;
        self.write_byte(Register::config, config | t_sb | filter);
    }

    pub fn set_control(&mut self, new: Control) {
        let osrs_t: u8 = (new.osrs_t as u8) << 5;
        let osrs_p: u8 = (new.osrs_p as u8) << 2;
        let control: u8 = osrs_t | osrs_p | (new.mode as u8);
        self.write_byte(Register::ctrl_meas, control);
    }

    pub fn control(&mut self) -> Control {
        let config = self.read_byte(Register::ctrl_meas);
        let osrs_t = match (config & (0b111 << 5)) >> 5 {
            x if x == Oversampling::skipped as u8 => Oversampling::skipped,
            x if x == Oversampling::x1 as u8 => Oversampling::x1,
            x if x == Oversampling::x2 as u8 => Oversampling::x2,
            x if x == Oversampling::x4 as u8 => Oversampling::x4,
            x if x == Oversampling::x8 as u8 => Oversampling::x8,
            _ => Oversampling::x16
        };
        let osrs_p = match (config & (0b111 << 2)) >> 2 {
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

/// Standby time in ms
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum Standby {
    ms0_5 = 0b000,
    ms62_5 = 0b001,
    ms125 = 0b010,
    ms250 = 0b011,
    ms500 = 0b100,
    ms1000 = 0b101,
    ms2000 = 0b110,
    ms4000 = 0b111,
    unknown
}

/// The time constant of IIR filter
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum Filter {
    off = 0x00,
    c2 = 0x01,
    c4 = 0x02,
    c8 = 0x03,
    c16 = 0x04,
    unknown
}

/// Configuration register, sets the rate, filter and interface options
/// of the device. Note that writing to this register while device in normal
/// mode may be ignored. Writes in sleep mode are not ignored.
///
/// spi3w_en is intentionally left out of this implementation.
#[derive(Debug)]
pub struct Config {
    /// Controls inactive duration in normal mode
    pub t_sb: Standby,
    /// Controls the time constant of IIR filter
    pub filter: Filter
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
#[allow(dead_code)]
pub enum Oversampling {
    skipped = 0b000,
    x1 = 0b001,
    x2 = 0b010,
    x4 = 0b011,
    x8 = 0b100,
    x16 = 0b101
}

#[derive(Debug)]
#[allow(dead_code)]
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
    config = 0xF5,
    press = 0xF7,
    calib00 = 0x88
}
