#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use core::fmt::Write;
use core::intrinsics;
use core::panic::PanicInfo;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::pac::interrupt;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use nb;

static mut L: Option<hal::serial::Tx<hal::pac::USART2>> = None;
static mut RX_CONSOLE: Option<hal::serial::Rx<hal::pac::USART2>> = None;

static mut RX_GPS: Option<hal::serial::Rx<hal::pac::USART3>> = None;
static mut TX_GPS: Option<hal::serial::Tx<hal::pac::USART3>> = None;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
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
    let mut usart2 =
        device
            .USART2
            .serial((gpioa.pa14, gpioa.pa15), Bps(115200), clocks);

    let mut usart3 =
        device
            .USART3
            .serial((gpiob.pb10, gpiob.pb11), Bps(9600), clocks);

    usart3.listen(serial::Event::Rxne);
    usart2.listen(serial::Event::Rxne);
    let us2_int = usart2.get_interrupt();
    let us3_int = usart3.get_interrupt();

    let (mut tx2, rx2) = usart2.split();
    let (tx3, rx3) = usart3.split();
    // COBS frame
    tx2.write(0x00).unwrap();
    unsafe {
        L = Some(tx2);
        RX_CONSOLE = Some(rx2);
        RX_GPS = Some(rx3);
        TX_GPS = Some(tx3);
    };
    let l = unsafe { extract(&mut L) };
    write!(l, "logger ok...\r\n").unwrap();
    write!(l, "starting loop...\r\n").unwrap();
    unsafe { cortex_m::interrupt::enable() };
    unsafe { cortex_m::peripheral::NVIC::unmask(us2_int) };
    unsafe { cortex_m::peripheral::NVIC::unmask(us3_int) };

    loop {
        cortex_m::asm::wfi();
    }
}

unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
    match opt {
        Some(ref mut x) => &mut *x,
        None => panic!("extract"),
    }
}

#[interrupt]
fn USART2_EXTI26() {
    let rx = unsafe { extract(&mut RX_CONSOLE) };
    let l = unsafe { extract(&mut L) };
    match rx.read() {
        Ok(b) => {
            // echo byte as is to console
            write!(l, "{}", b as char).unwrap();
        }
        Err(nb::Error::WouldBlock) => {}
        Err(nb::Error::Other(e)) => match e {
            serial::Error::Overrun => {
                rx.clear_overrun_error();
            }
            serial::Error::Framing => {
                rx.clear_framing_error();
            }
            serial::Error::Noise => {
                rx.clear_noise_error();
            }
            _ => {
                write!(l, "console read error: {:?}", e).unwrap();
            }
        },
    };
}

#[interrupt]
fn USART3_EXTI28() {
    let rx = unsafe { extract(&mut RX_GPS) };
    let l = unsafe { extract(&mut L) };
    match rx.read() {
        Ok(b) => {
            // transfer byte to console
            l.write_char(b as char).unwrap();
        }
        Err(nb::Error::WouldBlock) => {}
        Err(nb::Error::Other(e)) => match e {
            serial::Error::Overrun => {
                rx.clear_overrun_error();
            }
            serial::Error::Framing => {
                rx.clear_framing_error();
            }
            serial::Error::Noise => {
                rx.clear_noise_error();
            }
            _ => {
                write!(l, "gps read error: {:?}", e).unwrap();
            }
        },
    };
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    match unsafe { &mut L } {
        Some(ref mut l) => {
            let payload = panic_info.payload().downcast_ref::<&str>();
            match (panic_info.location(), payload) {
                (Some(location), Some(msg)) => {
                    write!(
                        l,
                        "\r\npanic in file '{}' at line {}: {:?}\r\n",
                        location.file(),
                        location.line(),
                        msg
                    )
                    .unwrap();
                }
                (Some(location), None) => {
                    write!(
                        l,
                        "panic in file '{}' at line {}",
                        location.file(),
                        location.line()
                    )
                    .unwrap();
                }
                (None, Some(msg)) => {
                    write!(l, "panic: {:?}", msg).unwrap();
                }
                (None, None) => {
                    write!(l, "panic occured, no info available").unwrap();
                }
            }
        }
        None => {}
    }
    intrinsics::abort()
}
