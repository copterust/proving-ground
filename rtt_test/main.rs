#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use cortex_m_rt::entry;
use hal::prelude::*;
use hal::gpio;

//use rtt_target::{rtt_init_print, rprintln};
use panic_abort as _;

#[entry]
fn main() -> ! {
//    rtt_init_print!();
    let device = hal::pac::Peripherals::take().unwrap();
    // let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    // let mut flash = device.FLASH.constrain();
    // let clocks = rcc
    //     .cfgr
    //     .sysclk(64.mhz())
    //     .pclk1(32.mhz())
    //     .pclk2(32.mhz())
    //     .freeze(&mut flash.acr);
    //rprintln!("device ok!");
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    // rprintln!("gpiob ok!");

    let mut beeper = gpiob
        .pb3
        .pull_type(gpio::PullNone)
        .output()
        .output_type(gpio::PushPull);
    // rprintln!("beeper ok!");
    let _ = beeper.set_high();
    //rprintln!("set high!");
    loop {
        //rprintln!("in the loop!");
        cortex_m::asm::wfi();
        //rprintln!("should not !");
    }
}
