#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::{self, Write};

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::prelude::*;
use hal::time::Bps;
use nb;

static mut NOW_MS: u32 = 0;
static mut LAST_SNAPSHOT_MS: u32 = 0;
static mut L: Option<Logger<hal::serial::Tx<hal::stm32f30x::USART1>>> = None;

#[entry]
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(32.mhz())
        .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let serial = device
        .USART1
        .serial((gpioa.pa9, gpioa.pa10), Bps(115200), clocks);
    let (mut tx, _rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    unsafe {
        L = Some(Logger { tx });
    }
    let l = unsafe { extract(&mut L) };
    write!(l, "logger ok\r\n");
    let ticks = clocks.sysclk().0 / 1000; // 1 ms?
    let mut syst = core.SYST;
    syt_tick_config(&mut syst, ticks);
    write!(l, "Ticks: {}\r\n", ticks);
    write!(l, "Waiting for interrupt; will print every ~2s\r\n");
    loop {
        cortex_m::asm::wfi();
    }
}

fn syt_tick_config(syst: &mut cortex_m::peripheral::SYST, ticks: u32) {
    syst.set_reload(ticks - 1);
    syst.clear_current();
    syst.set_clock_source(SystClkSource::Core);
    syst.enable_interrupt();
    syst.enable_counter();
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

unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
    match opt {
        Some(ref mut x) => &mut *x,
        None => panic!("extract"),
    }
}

#[exception]
unsafe fn SysTick() {
    NOW_MS = NOW_MS.wrapping_add(1);
    if (NOW_MS.wrapping_sub(LAST_SNAPSHOT_MS)) > 2000 {
        LAST_SNAPSHOT_MS = NOW_MS;
        let l = extract(&mut L);
        write!(
            l,
            "Tick: {:?}ms; last: {:?}ms\r\n",
            NOW_MS, LAST_SNAPSHOT_MS
        );
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
