#![allow(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

// use core::fmt::{self, Write};

use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::hprintln;
use hal::prelude::*;
use hal::time::Bps;
use hal::{delay, serial};
use nb;

use mpu9250::Mpu9250;

#[entry]
#[inline(never)]
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(32.mhz())
        .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    let mut delay = delay::Delay::new(core.SYST, clocks);

    // SPI1
    let ncs = gpiob.pb0.output().push_pull();
    let scl_sck = gpiob.pb3;
    let sda_sdi_mosi = gpiob.pb5;
    let ad0_sdo_miso = gpiob.pb4;
    let spi = device.SPI1.spi(
        (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
        mpu9250::MODE,
        1.mhz(),
        clocks,
    );
    hprintln!("spi ok").unwrap();

    hprintln!("delay ok").unwrap();
    // MPU
    let mut mpu = Mpu9250::imu_default(spi, ncs, &mut delay).unwrap();

    loop {
        match mpu.accel() {
            Ok(accel) => {
                hprintln!("{:?}; {:?}; {:?}", accel.x, accel.y, accel.z).unwrap();
            }
            Err(e) => {
                hprintln!("e");
            }
        }
    }
}

unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
    match opt {
        Some(ref mut x) => &mut *x,
        None => panic!("extract"),
    }
}
