#![allow(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

// use core::fmt::{self, Write};

use asm_delay::AsmDelay;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::hprintln;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use nb;

use mpu9250::{Mpu9250, MpuConfig};

#[entry]
#[inline(never)]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(72.mhz())
                    .pclk1(32.mhz())
                    .pclk2(32.mhz())
                    .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let mut pa1 = gpioa.pa1
                       .output()
                       .output_speed(hal::gpio::HighSpeed)
                       .pull_type(hal::gpio::PullDown);

    // SPI1
    let ncs = gpiob.pb0.output().push_pull();
    let scl_sck = gpiob.pb3;
    let sda_sdi_mosi = gpiob.pb5;
    let ad0_sdo_miso = gpiob.pb4;
    let spi = device.SPI1.spi((scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                              mpu9250::MODE,
                              1.mhz(),
                              clocks);
    hprintln!("spi ok").unwrap();
    let mut delay = AsmDelay::new(clocks.sysclk());
    hprintln!("delay ok").unwrap();
    // MPU
    let mmpu = Mpu9250::marg_with_reinit(spi,
                                         ncs,
                                         &mut delay,
                                         &mut MpuConfig::marg(),
                                         |spi, ncs| {
                                             let (dev_spi, (scl, miso, mosi)) =
                                                 spi.free();
                                             let new_spi =
                                                 dev_spi.spi((scl, miso, mosi),
                                                             mpu9250::MODE,
                                                             20.mhz(),
                                                             clocks);
                                             Some((new_spi, ncs))
                                         });
    // .unwrap();
    let mut mpu = match mmpu {
        Ok(m) => m,
        Err(e) => {
            hprintln!("err: {:?}", e);
            panic!("oops")
        }
    };
    hprintln!("mpu ok").unwrap();

    pa1.set_low();
    for _ in 1..10 {
        pa1.toggle();
        match mpu.all() {
            Ok(a) => {
                hprintln!(
                    "[a:({:?},{:?},{:?}),g:({:?},{:?},{:?}),m:({:?},{:?},{:?}),]",
                    a.accel.x,
                    a.accel.y,
                    a.accel.z,
                    a.gyro.x,
                    a.gyro.y,
                    a.gyro.z,
                    a.mag.x,
                    a.mag.y,
                    a.mag.z,
                )
                .unwrap();
            }
            Err(e) => {
                hprintln!("e");
            }
        }
    }

    hprintln!("running calibration...").unwrap();
    let accel_biases = mpu.calibrate_at_rest(&mut delay).unwrap();
    hprintln!("calibration ok: {:?}", accel_biases).unwrap();

    loop {
        pa1.toggle();
        match mpu.all() {
            Ok(a) => {
                hprintln!(
                    "[a:({:?},{:?},{:?}),g:({:?},{:?},{:?}),m:({:?},{:?},{:?}),]",
                    a.accel.x,
                    a.accel.y,
                    a.accel.z,
                    a.gyro.x,
                    a.gyro.y,
                    a.gyro.z,
                    a.mag.x,
                    a.mag.y,
                    a.mag.z,
                )
                .unwrap();
            }
            Err(e) => {
                hprintln!("e");
            }
        };
    }
}

unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
    match opt {
        Some(ref mut x) => &mut *x,
        None => panic!("extract"),
    }
}
