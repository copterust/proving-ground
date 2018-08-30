#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::{self, Write};

use hal::gpio;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use nb;
use rt::{entry, exception};

mod vl53l0x;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(36.mhz())
        .freeze(&mut flash.acr);

    // serial
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let mut serial = device
        .USART1
        .serial((gpioa.pa9, gpioa.pa10), Bps(9600), clocks);
    serial.listen(serial::Event::Rxne);
    let (tx, mut rx) = serial.split();

    let mut l = Logger { tx };
    write!(l, "\r\nVL53L0x demo\r\n");

    // i2c
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let scl = gpiob.pb8.alternating(gpio::AF4);
    let sda = gpiob.pb9.alternating(gpio::AF4);
    let i2c = hal::i2c::I2c::i2c1(device.I2C1, (scl, sda), 1.mhz(), clocks, &mut rcc.apb1);

    let mut tof = vl53l0x::new(i2c).unwrap();
    write!(l, "WHO_AM_I: {}\r\n", tof.who_am_i());
    write!(l, "RevisionID: {}\r\n", tof.revision_id);

    loop {
        match rx.read() {
            Ok(b) => {
                l.tx.write(b).unwrap();
            }
            Err(nb::Error::Other(e)) => match e {
                serial::Error::Overrun => {
                    rx.clear_overrun_error();
                }
                _ => {
                    write!(l, "read error: {:?}", e);
                }
            },
            Err(nb::Error::WouldBlock) => {}
        };
    }
}

struct Logger<W: ehal::serial::Write<u8>> {
    tx: W,
}
impl<W: ehal::serial::Write<u8>> fmt::Write for Logger<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match self.write_char(c) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        match self.tx.flush() {
            Ok(_) => {}
            Err(_) => {}
        };

        Ok(())
    }

    fn write_char(&mut self, s: char) -> fmt::Result {
        match nb::block!(self.tx.write(s as u8)) {
            Ok(_) => {}
            Err(_) => {}
        }
        Ok(())
    }
}

exception!(HardFault, |ef| {
    panic!("HardFault at {:#?}", ef);
});

exception!(*, |irqn| {
    panic!("Unhandled exception (IRQn = {})", irqn);
});
