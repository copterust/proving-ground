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
use rt::{entry, exception, ExceptionFrame};

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

    // i2c
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let scl = gpiob.pb8.alternating(gpio::AF4);
    let sda = gpiob.pb9.alternating(gpio::AF4);
    let mut i2c = hal::i2c::I2c::i2c1(
        device.I2C1,
        (scl, sda),
        1.mhz(),
        clocks,
        &mut rcc.apb1);

    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let txpin = gpioa.pa9.alternating(gpio::AF7);
    let rxpin = gpioa.pa10.alternating(gpio::AF7);
    let mut serial = serial::Serial::usart1(
        device.USART1,
        (txpin, rxpin),
        Bps(9600),
        clocks,
        &mut rcc.apb2,
    );
    serial.listen(serial::Event::Rxne);
    let (mut tx, mut rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    let mut l = Logger { tx };
    write!(l, "\r\nReading i2c...\r\n");

    let mut data: [u8; 1] = [0];
    let addr: u8 = 0xC0; // Probably WHO_AM_I, should be 0xEE
    i2c.write_read(0x29, &[addr], &mut data).unwrap();
    write!(l, "Result: {}\r\n", data[0]);

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

exception!(HardFault, hard_fault);
fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);
fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
