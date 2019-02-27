#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_abort; // requires nightly

use cortex_m::asm;

use cortex_m_rt::entry;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;

#[entry]
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
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    // let mut afio = device.AFIO.constrain(&mut rcc.apb2);
    // rcc.apb2.enr().modify(|_, w| w.afioen().enabled());
    // rcc.apb2.rstr().modify(|_, w| w.afiorst().set_bit());
    // rcc.apb2.rstr().modify(|_, w| w.afiorst().clear_bit());

    let channels = device.DMA1.split(&mut rcc.ahb);

    // USART2
    let mut serial = device
        .USART1
        .serial((gpiob.pb6, gpiob.pb7), Bps(9600), clocks);
    serial.listen(serial::Event::Rxne);
    serial.listen(serial::Event::Txe);
    let (mut tx, _) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();

    let (_, c, tx) = tx.write_all(channels.4, b"The quick brown fox").wait();

    asm::bkpt();

    let (_, c, tx) = tx.write_all(c, b" jumps").wait();

    asm::bkpt();

    let (_, c, tx) = tx.write_all(c, b" over the lazy dog.").wait();

    asm::bkpt();

    let (_, c, tx) = tx.write_all(c, b"287012370 91287012.").wait();
    let (_, _c, _tx) = tx.write_all(c, b"wyfdwfyu  91287012.").wait();

    loop {
        asm::wfi();
    }
}
