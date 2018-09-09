#![deny(warnings)]
#![no_std]
#![no_main]
use cortex_m_rt::entry;
#[allow(unused)]
use panic_abort;

#[entry]
fn main() -> ! {
    panic!("opa");
}
