#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_semihosting;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::hprintln;
use hal::prelude::*;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(72.mhz())
                    .pclk1(32.mhz())
                    .pclk2(32.mhz())
                    .freeze(&mut flash.acr);
    hprintln!("main: sysclk: {:?}; hclck: {:?}",
              clocks.sysclk(),
              clocks.hclk()).unwrap();
    loop {}
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("hardfault").unwrap();

    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    hprintln!("unh interrult").unwrap();
    panic!("Unhandled exception (IRQn = {})", irqn);
}
