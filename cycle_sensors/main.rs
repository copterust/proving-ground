#![allow(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use cortex_m::asm;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::prelude::*;
use hal::time::Bps;
use hal::{delay, serial};
use nb;

use mpu9250::{Mpu9250, MpuConfig};

#[entry]
#[inline(never)]
fn main() -> ! {
    let freq = 72.mhz();
    let device = hal::pac::Peripherals::take().unwrap();
    let mut core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(freq)
                    .pclk1(32.mhz())
                    .pclk2(32.mhz())
                    .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);

    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let mut pa1 = gpioa.pa1
                       .output()
                       .output_speed(hal::gpio::HighSpeed)
                       .pull_type(hal::gpio::PullDown);

    let mut delay = asm_delay::AsmDelay::new(freq);

    // SPI1
    let ncs = gpiob.pb0.output().push_pull();
    let scl_sck = gpiob.pb3;
    let sda_sdi_mosi = gpiob.pb5;
    let ad0_sdo_miso = gpiob.pb4;
    let spi = device.SPI1.spi((scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                              mpu9250::MODE,
                              1.mhz(),
                              clocks);

    // MPU
    let mut mpu = Mpu9250::marg_default(spi, ncs, &mut delay).unwrap();

    pa1.set_low();
    let mut s = 0;
    let mut i = 0;

    core.DCB.enable_trace();
    core.DWT.enable_cycle_counter();

    cortex_m_semihosting::hprintln!("start").unwrap();
    loop {
        pa1.toggle();
        let n1 = cortex_m::peripheral::DWT::get_cycle_count() as i32;
        match mpu.all() {
            Ok(a) => {}
            Err(e) => {}
        }
        let n2 = cortex_m::peripheral::DWT::get_cycle_count() as i32;
        let diff = n2.wrapping_sub(n1);
        s += diff;
        i += 1;
        if i == 100 {
            cortex_m_semihosting::hprintln!("100: {} // {}",
                                            s,
                                            (s as f32) / 100f32);
            i = 0;
            s = 0;
        }
    }
}
