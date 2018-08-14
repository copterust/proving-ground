#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::{self, Write};

use hal::prelude::*;
use hal::time::Bps;
use hal::{delay, gpio, serial, spi};
use nb;
use rt::{entry, exception, ExceptionFrame};

use mpu9250::Mpu9250;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(36.mhz())
        .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    let txpin = gpioa.pa9.alternating(gpio::AF7);
    let rxpin = gpioa.pa10.alternating(gpio::AF7);
    let mut serial = serial::Serial::usart1(
        device.USART1,
        (txpin, rxpin),
        Bps(115200),
        clocks,
        &mut rcc.apb2,
    );
    serial.listen(serial::Event::Rxne);
    let (mut tx, _rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    let mut l = Logger { tx };
    write!(l, "logger ok\r\n");
    let mut delay = delay::Delay::new(core.SYST, clocks);
    // SPI1
    let ncs = gpiob.pb9.output().push_pull();
    let scl_sck = gpiob.pb3.alternating(gpio::AF5);
    let sda_sdi_mosi = gpiob.pb5.alternating(gpio::AF5);
    let ad0_sdo_miso = gpiob.pb4.alternating(gpio::AF5);
    let spi = spi::Spi::spi1(
        device.SPI1,
        (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
        mpu9250::MODE,
        1.mhz(),
        clocks,
        &mut rcc.apb2,
    );
    let mut mpu = Mpu9250::imu_default(spi, ncs, &mut delay).unwrap();

    write!(l, "starting loop...\r\n");

    let mut flag = true;
    loop {
        if flag {
            match mpu.temp() {
                Ok(t) => {
                    write!(l, "Temp: {:?}\r\n", t);
                }
                Err(e) => {
                    write!(l, "Error: {:?}\r\n", e);
                }
            }
        } else {
            match mpu.raw_temp() {
                Ok(t) => {
                    write!(l, "Raw Temp: {:?}\r\n", t);
                }
                Err(e) => {
                    write!(l, "Error: {:?}\r\n", e);
                }
            }
        }
        flag = !flag;
        for _ in 0..10 {
            delay.delay_ms(250u32);
        }
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
