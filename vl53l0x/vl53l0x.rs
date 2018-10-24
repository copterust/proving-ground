//use ehal::blocking::i2c::WriteRead;

pub const ADDRESS: u8 = 0x29;

pub struct VL53L0x<I2C: ehal::blocking::i2c::WriteRead> {
    com: I2C,
    pub revision_id: u8,
    io_mode2v8: bool,
    stop_variable: u8,
}

/// MPU Error
#[derive(Debug, Copy, Clone)]
pub enum Error<E> {
    /// WHO_AM_I returned invalid value (returned value is argument).
    InvalidDevice(u8),
    /// Underlying bus error.
    BusError(E),
}

pub fn new<I2C, E>(i2c: I2C) -> Result<VL53L0x<I2C>, Error<E>>
where
    I2C: ehal::blocking::i2c::WriteRead<Error = E>,
{
    let mut chip = VL53L0x {
        com: i2c,
        revision_id: 0x00,
        io_mode2v8: true,
        stop_variable: 0,
    };

    let wai = chip.who_am_i();
    if wai == 0xEE {
        chip.init_hardware();
        // FIXME: return an error/optional
        /*
        chip.set_high_i2c_voltage(); // TODO: make configurable
        chip.revision_id = chip.read_revision_id();
        chip.reset();
        chip.set_high_i2c_voltage();
        chip.set_standard_i2c_mode(); // TODO: make configurable
         */
        Ok(chip)
    } else {
        Err(Error::InvalidDevice(wai))
    }
}

impl<I2C: ehal::blocking::i2c::WriteRead> VL53L0x<I2C> {
    fn power_on(&mut self) {
        // TODO use pin to poweron
    }

    fn write_byte(&mut self, reg: u8, byte: u8) {
        let mut buffer = [0];
        let _ = self
            .com
            .write_read(ADDRESS, &[reg, byte], &mut buffer);
    }

    fn write_register(&mut self, reg: Register, byte: u8) {
        let mut buffer = [0];
        let _ = self
            .com
            .write_read(ADDRESS, &[reg as u8, byte], &mut buffer);
    }

    fn read_register(&mut self, reg: Register) -> u8 {
        let mut data: [u8; 1] = [0];
        // FIXME:
        //  * device address is not a const
        //  * register address is u16
        let _ = self.com.write_read(ADDRESS, &[reg as u8], &mut data);
        data[0]
    }

    fn read_byte(&mut self, reg: u8) -> u8 {
        let mut data: [u8; 1] = [0];
        // FIXME:
        //  * device address is not a const
        //  * register address is u16
        let _ = self.com.write_read(ADDRESS, &[reg], &mut data);
        data[0]
    }

    fn write6(&mut self, reg: Register, bytes: [u8; 6]) {
        let mut buf: [u8; 6] = [0, 0, 0, 0, 0, 0];
        let _ = self
            .com
            .write_read(ADDRESS, &[reg as u8, bytes[0], bytes[1], bytes[2],
                bytes[3], bytes[4], bytes[5]], &mut buf);
    }

    fn write_register16(&mut self, reg: Register, word: u16) {
        let mut buffer = [0];
        let msb = (word >> 8) as u8;
        let lsb = (word & 0xFF) as u8;
        let _ = self
            .com
            .write_read(ADDRESS, &[reg as u8, msb, lsb], &mut buffer);
    }

    fn set_signal_rate_limit(&mut self, limit: f32) -> bool {
	    if limit < 0.0 || limit > 511.99 {
		    return false;
	    }

	    // Q9.7 fixed point format (9 integer bits, 7 fractional bits)
	    self.write_register16(Register::FINAL_RANGE_CONFIG_MIN_COUNT_RATE_RTN_LIMIT, (limit * ((1 << 7) as f32)) as u16);
	    return true;
    }

    fn get_spad_info(&mut self) -> (u8, u8) {
        self.write_byte(0x80, 0x01);
        self.write_byte(0xFF, 0x01);
        self.write_byte(0x00, 0x00);

        self.write_byte(0xFF, 0x06);
        let mut tmp83 = self.read_byte(0x83);
        self.write_byte(0x83, tmp83 | 0x04);
        self.write_byte(0xFF, 0x07);
        self.write_byte(0x81, 0x01);

        self.write_byte(0x80, 0x01);

        self.write_byte(0x94, 0x6b);
        self.write_byte(0x83, 0x00);

        while self.read_byte(0x83) == 0x00 {
            // TODO check timeout
            // usleep(1);
        }

        self.write_byte(0x83, 0x01);
        let tmp = self.read_byte(0x92);

        let count: u8 = tmp & 0x7f;
        let type_is_aperture: u8 = (tmp >> 7) & 0x01;

        self.write_byte(0x81, 0x00);
        self.write_byte(0xFF, 0x06);
        tmp83 = self.read_byte(0x83);
        self.write_byte(0x83, tmp83 & !0x04);
        self.write_byte(0xFF, 0x01);
        self.write_byte(0x00, 0x01);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x80, 0x00);

        return (count, type_is_aperture);
    }

    fn read6(&mut self, reg: Register) -> [u8; 6] {
        let mut data: [u8; 6] = [0, 0, 0, 0, 0, 0];
        let _ = self.com.write_read(ADDRESS, &[reg as u8], &mut data);
        data
    }

    fn init_hardware(&mut self) {
        // Enable the sensor
        
        self.power_on();
        // VL53L0X_DataInit() begin

        // Sensor uses 1V8 mode for I/O by default; switch to 2V8 mode if necessary
        if self.io_mode2v8 {
            // set bit 0
            let ext_sup_hv = self.read_register(Register::VHV_CONFIG_PAD_SCL_SDA__EXTSUP_HV);
            self.write_register(Register::VHV_CONFIG_PAD_SCL_SDA__EXTSUP_HV, ext_sup_hv | 0x01);
        }

        // "Set I2C standard mode"
        self.write_byte(0x88, 0x00);
        self.write_byte(0x80, 0x01);
        self.write_byte(0xFF, 0x01);
        self.write_byte(0x00, 0x00);
        self.stop_variable = self.read_byte(0x91);
        self.write_byte(0x00, 0x01);
        self.write_byte(0xFF, 0x00);
        self.write_byte(0x80, 0x00);

        // disable SIGNAL_RATE_MSRC (bit 1) and SIGNAL_RATE_PRE_RANGE (bit 4) limit checks
        let config = self.read_register(Register::MSRC_CONFIG_CONTROL);
        self.write_register(Register::MSRC_CONFIG_CONTROL, config | 0x12);

        // set final range signal rate limit to 0.25 MCPS (million counts per second)
        self.set_signal_rate_limit(0.25);

        self.write_register(Register::SYSTEM_SEQUENCE_CONFIG, 0xFF);

        // VL53L0X_DataInit() end

        // VL53L0X_StaticInit() begin

        // TODO fail to initialize on timeout of this
        let (spad_count, spad_type_is_aperture) = self.get_spad_info();

        // The SPAD map (RefGoodSpadMap) is read by VL53L0X_get_info_from_device() in the API,
        // but the same data seems to be more easily readable from GLOBAL_CONFIG_SPAD_ENABLES_REF_0 through _6, so read it from there
        let mut ref_spad_map: [u8; 6] = self.read6(Register::GLOBAL_CONFIG_SPAD_ENABLES_REF_0);

        // -- VL53L0X_set_reference_spads() begin (assume NVM values are valid)

        self.write_byte(0xFF, 0x01);
        self.write_register(Register::DYNAMIC_SPAD_REF_EN_START_OFFSET, 0x00);
        self.write_register(Register::DYNAMIC_SPAD_NUM_REQUESTED_REF_SPAD, 0x2C);
        self.write_byte(0xFF, 0x00);
        self.write_register(Register::GLOBAL_CONFIG_REF_EN_START_SELECT, 0xB4);

        // 12 is the first aperture spad
        let first_spad_to_enable = if spad_type_is_aperture > 0 { 12 } else { 0 };
        let mut spads_enabled: u8 = 0;

        for i in 0..48 {
            if i < first_spad_to_enable || spads_enabled == spad_count {
                // This bit is lower than the first one that should be enabled, or (reference_spad_count) bits have already been enabled, so zero this bit
                ref_spad_map[i / 8] &= !(1 << (i % 8));
            } else if (ref_spad_map[i / 8] >> (i % 8)) & 0x1 > 0 {
                spads_enabled = spads_enabled + 1;
            }
        }

        self.write6(Register::GLOBAL_CONFIG_SPAD_ENABLES_REF_0, ref_spad_map);

        // -- VL53L0X_set_reference_spads() end

        // -- VL53L0X_load_tuning_settings() begin
        // DefaultTuningSettings from vl53l0x_tuning.h

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x00, 0x00);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x09, 0x00);
        self.write_byte(0x10, 0x00);
        self.write_byte(0x11, 0x00);

        self.write_byte(0x24, 0x01);
        self.write_byte(0x25, 0xFF);
        self.write_byte(0x75, 0x00);

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x4E, 0x2C);
        self.write_byte(0x48, 0x00);
        self.write_byte(0x30, 0x20);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x30, 0x09);
        self.write_byte(0x54, 0x00);
        self.write_byte(0x31, 0x04);
        self.write_byte(0x32, 0x03);
        self.write_byte(0x40, 0x83);
        self.write_byte(0x46, 0x25);
        self.write_byte(0x60, 0x00);
        self.write_byte(0x27, 0x00);
        self.write_byte(0x50, 0x06);
        self.write_byte(0x51, 0x00);
        self.write_byte(0x52, 0x96);
        self.write_byte(0x56, 0x08);
        self.write_byte(0x57, 0x30);
        self.write_byte(0x61, 0x00);
        self.write_byte(0x62, 0x00);
        self.write_byte(0x64, 0x00);
        self.write_byte(0x65, 0x00);
        self.write_byte(0x66, 0xA0);

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x22, 0x32);
        self.write_byte(0x47, 0x14);
        self.write_byte(0x49, 0xFF);
        self.write_byte(0x4A, 0x00);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x7A, 0x0A);
        self.write_byte(0x7B, 0x00);
        self.write_byte(0x78, 0x21);

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x23, 0x34);
        self.write_byte(0x42, 0x00);
        self.write_byte(0x44, 0xFF);
        self.write_byte(0x45, 0x26);
        self.write_byte(0x46, 0x05);
        self.write_byte(0x40, 0x40);
        self.write_byte(0x0E, 0x06);
        self.write_byte(0x20, 0x1A);
        self.write_byte(0x43, 0x40);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x34, 0x03);
        self.write_byte(0x35, 0x44);

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x31, 0x04);
        self.write_byte(0x4B, 0x09);
        self.write_byte(0x4C, 0x05);
        self.write_byte(0x4D, 0x04);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x44, 0x00);
        self.write_byte(0x45, 0x20);
        self.write_byte(0x47, 0x08);
        self.write_byte(0x48, 0x28);
        self.write_byte(0x67, 0x00);
        self.write_byte(0x70, 0x04);
        self.write_byte(0x71, 0x01);
        self.write_byte(0x72, 0xFE);
        self.write_byte(0x76, 0x00);
        self.write_byte(0x77, 0x00);

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x0D, 0x01);

        self.write_byte(0xFF, 0x00);
        self.write_byte(0x80, 0x01);
        self.write_byte(0x01, 0xF8);

        self.write_byte(0xFF, 0x01);
        self.write_byte(0x8E, 0x01);
        self.write_byte(0x00, 0x01);
        self.write_byte(0xFF, 0x00);
        self.write_byte(0x80, 0x00);

        // -- VL53L0X_load_tuning_settings() end

        // "Set interrupt config to new sample ready"
        // -- VL53L0X_SetGpioConfig() begin

        self.write_register(SYSTEM_INTERRUPT_CONFIG_GPIO, 0x04);
        // active low
        self.write_register(GPIO_HV_MUX_ACTIVE_HIGH, self.read_register(GPIO_HV_MUX_ACTIVE_HIGH) & ~0x10);
        self.write_register(SYSTEM_INTERRUPT_CLEAR, 0x01);

        // -- VL53L0X_SetGpioConfig() end
/*

        self.measurement_timing_budget_microseconds = self.get_measurement_timing_budget();

        // "Disable MSRC and TCC by default"
        // MSRC = Minimum Signal Rate Check
        // TCC = Target CentreCheck
        // -- VL53L0X_SetSequenceStepEnable() begin

        self.write_register(SYSTEM_SEQUENCE_CONFIG, 0xE8);

        // -- VL53L0X_SetSequenceStepEnable() end

        // "Recalculate timing budget"
        self.set_measurement_timing_budget(self.measurement_timing_budget_microseconds);

        // VL53L0X_StaticInit() end

        // VL53L0X_PerformRefCalibration() begin (VL53L0X_perform_ref_calibration())

        // -- VL53L0X_perform_vhv_calibration() begin

        self.write_register(SYSTEM_SEQUENCE_CONFIG, 0x01);
        if (!self.perform_single_ref_calibration(0x40)) {
            throw(std::runtime_error("Failed performing ref/vhv calibration!"));
        }

        // -- VL53L0X_perform_vhv_calibration() end

        // -- VL53L0X_perform_phase_calibration() begin

        self.write_register(SYSTEM_SEQUENCE_CONFIG, 0x02);
        if (!self.perform_single_ref_calibration(0x00)) {
            throw(std::runtime_error("Failed performing ref/phase calibration!"));
        }

        // -- VL53L0X_perform_phase_calibration() end

        // "restore the previous Sequence Config"
        self.write_register(SYSTEM_SEQUENCE_CONFIG, 0xE8);

        // VL53L0X_PerformRefCalibration() end
    }
*/
    }

    pub fn who_am_i(&mut self) -> u8 {
        self.read_register(Register::WhoAmI)
    }

/*
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
*/
}

#[allow(non_camel_case_types)]
enum Register {
    WhoAmI = 0xC0,
	VHV_CONFIG_PAD_SCL_SDA__EXTSUP_HV = 0x89,
    MSRC_CONFIG_CONTROL = 0x60,
    SYSTEM_SEQUENCE_CONFIG = 0x01,
    FINAL_RANGE_CONFIG_MIN_COUNT_RATE_RTN_LIMIT = 0x44,
	GLOBAL_CONFIG_SPAD_ENABLES_REF_0 = 0xB0,
	DYNAMIC_SPAD_REF_EN_START_OFFSET = 0x4F,
	DYNAMIC_SPAD_NUM_REQUESTED_REF_SPAD = 0x4E,
	GLOBAL_CONFIG_REF_EN_START_SELECT = 0xB6,
}
