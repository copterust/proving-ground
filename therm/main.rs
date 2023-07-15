#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::prelude::*;
use hal::time::Bps;
use hal::{delay, serial};

use mpu9250::Mpu9250;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
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
    let mut serial =
        device
            .USART1
            .serial((gpioa.pa9, gpioa.pa10), Bps(115200), clocks);
    serial.listen(serial::Event::Rxne);
    let (mut tx, _rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    let mut l = tx;
    write!(l, "logger ok\r\n").unwrap();
    let mut delay = delay::Delay::new(core.SYST, clocks);
    // SPI1
    let ncs = gpiob.pb9.output().push_pull();
    let scl_sck = gpiob.pb3;
    let sda_sdi_mosi = gpiob.pb5;
    let ad0_sdo_miso = gpiob.pb4;
    let spi = device.SPI1.spi(
        (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
        mpu9250::MODE,
        1.mhz(),
        clocks,
    );
    let mut mpu = Mpu9250::imu_default(spi, ncs, &mut delay).unwrap();

    write!(l, "starting loop...\r\n").unwrap();

    let mut flag = true;
    loop {
        if flag {
            match mpu.temp() {
                Ok(t) => {
                    write!(l, "Temp: {:?}\r\n", t).unwrap();
                }
                Err(e) => {
                    write!(l, "Error: {:?}\r\n", e).unwrap();
                }
            }
        } else {
            match mpu.raw_temp() {
                Ok(t) => {
                    write!(l, "Raw Temp: {:?}\r\n", t).unwrap();
                }
                Err(e) => {
                    write!(l, "Error: {:?}\r\n", e).unwrap();
                }
            }
        }
        flag = !flag;
        for _ in 0..10 {
            delay.delay_ms(250u32);
        }
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
