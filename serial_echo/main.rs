#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use nb;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(32.mhz())
        .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    // USART2
    let mut serial =
        device
            .USART2
            .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
    serial.listen(serial::Event::Rxne);
    let (mut tx, mut rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    write!(tx, "starting loop...\r\n").unwrap();
    loop {
        match rx.read() {
            Ok(b) => {
                tx.write(b).unwrap();
            }
            Err(nb::Error::Other(e)) => match e {
                serial::Error::Overrun => {
                    rx.clear_overrun_error();
                }
                _ => {
                    write!(tx, "read error: {:?}", e).unwrap();
                }
            },
            Err(nb::Error::WouldBlock) => {}
        };
    }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
