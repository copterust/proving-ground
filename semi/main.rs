#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_semihosting;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::hprintln;

#[entry]
fn main() -> ! {
    hprintln!("main").unwrap();
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
