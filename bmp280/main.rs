#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::{self, Write};

use bmp280;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use nb;

#[entry]
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(32.mhz())
        .freeze(&mut flash.acr);

    // serial
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let mut serial = device
        .USART1
        .serial((gpioa.pa9, gpioa.pa10), Bps(115200), clocks);
    serial.listen(serial::Event::Rxne);
    let (mut tx, mut rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    let mut l = Logger { tx };
    write!(l, "\r\nBMP280 demo\r\n").unwrap();

    // i2c
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let scl = gpiob.pb6;
    let sda = gpiob.pb7;
    let i2c = device.I2C1.i2c((scl, sda), 400.khz(), clocks);
    write!(l, "i2c ok\r\n").unwrap();

    let mut ps = bmp280::BMP280::new(i2c).unwrap();
    write!(l, "ID: {}\r\n", ps.id()).unwrap();
    ps.reset();
    write!(l, "ID after reset: {}\r\n", ps.id()).unwrap();
    write!(l, "Status: {}\r\n", ps.status()).unwrap();
    write!(l, "{:?}\r\n", ps.control()).unwrap();
    ps.set_control(bmp280::Control {
        osrs_t: bmp280::Oversampling::x1,
        osrs_p: bmp280::Oversampling::x4,
        mode: bmp280::PowerMode::Normal,
    });
    write!(l, "After write {:?}\r\n", ps.control()).unwrap();
    ps.reset();
    write!(l, "After reset {:?}\r\n", ps.control()).unwrap();
    write!(l, "{:?}\r\n", ps.config()).unwrap();
    ps.set_config(bmp280::Config {
        t_sb: bmp280::Standby::ms250,
        filter: bmp280::Filter::c8,
    });
    write!(l, "After write {:?}\r\n", ps.config()).unwrap();
    ps.set_control(bmp280::Control {
        osrs_t: bmp280::Oversampling::x1,
        osrs_p: bmp280::Oversampling::x1,
        mode: bmp280::PowerMode::Forced,
    });
    write!(l, "Press any key to meausure\r\n").unwrap();
    loop {
        match rx.read() {
            Ok(_b) => {
                write!(l, "Temperature: {}\r\n", ps.temp()).unwrap();
                write!(l, "Pressure: {}\r\n", ps.pressure()).unwrap();
                ps.set_control(bmp280::Control {
                    osrs_t: bmp280::Oversampling::x1,
                    osrs_p: bmp280::Oversampling::x1,
                    mode: bmp280::PowerMode::Forced,
                });
            }
            Err(nb::Error::Other(e)) => match e {
                serial::Error::Overrun => {
                    rx.clear_overrun_error();
                }
                _ => {
                    write!(l, "read error: {:?}", e).unwrap();
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

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
