#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;

use hal::gpio;
use hal::prelude::*;
// use hal::delay;
// use ehal::blocking::delay::DelayMs;

use cortex_m_rt::{entry, exception, ExceptionFrame};

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    // let core = cortex_m::Peripherals::take().unwrap();
    // let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();
    // let clocks = rcc
    //     .cfgr
    //     .sysclk(64.mhz())
    //     .pclk1(32.mhz())
    //     .pclk2(32.mhz())
    //     .freeze(&mut flash.acr);
    // let mut delay = delay::Delay::new(core.SYST, clocks);

    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let mut beeper = gpiob.pb3
                          .pull_type(gpio::PullNone)
                          .output()
                          .output_type(gpio::PushPull);
    // let mut b = true;
    let _ = beeper.set_high();
    loop {
        cortex_m::asm::wfi();
    }

    // loop {
    //     if b {
    //         beeper.set_high();
    //     } else {
    //         beeper.set_low();
    //     }
    //     b = !b;
    //     for _ in 1..10 {
    //         delay.delay_ms(100u8);
    //     }

    // }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
