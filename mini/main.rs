#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;
use rt::{entry, exception};

entry!(main);
fn main() -> ! {
    panic!("opa");
}

exception!(HardFault, |ef| {
    panic!("HardFault at {:#?}", ef);
});

exception!(*, |irqn| {
    panic!("Unhandled exception (IRQn = {})", irqn);
});
