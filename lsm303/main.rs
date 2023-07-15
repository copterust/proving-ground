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

use lsm303c::Lsm303c;

static mut TX: Option<hal::serial::Tx<hal::pac::USART1>> = None;
static mut RX: Option<hal::serial::Rx<hal::pac::USART1>> = None;
static mut QUIET: bool = true;
const TURN_QUIET: u8 = 'q' as u8;

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
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let mut serial =
        device
            .USART1
            .serial((gpioa.pa9, gpioa.pa10), Bps(115200), clocks);
    let ser_int = serial.get_interrupt();
    serial.listen(serial::Event::Rxne);
    let (mut tx, rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    unsafe {
        TX = Some(tx);
        RX = Some(rx);
    };
    let l = unsafe { extract(&mut TX) };
    write!(l, "logger ok\r\n").unwrap();
    // I2C
    let i2c = device.I2C1.i2c((gpiob.pb6, gpiob.pb7), 400.khz(), clocks);
    write!(l, "i2c ok\r\n").unwrap();
    // lsm
    let mut lsm303 = Lsm303c::default(i2c).expect("lsm error");
    write!(l, "lsm ok\r\n").unwrap();
    // done
    unsafe { cortex_m::interrupt::enable() };
    unsafe { cortex_m::peripheral::NVIC::unmask(ser_int) };

    write!(l, "All ok; Press 'q' to toggle verbosity!\r\n").unwrap();
    loop {
        match lsm303.mag::<[f32; 3]>() {
            Ok(meas) => {
                if unsafe { !QUIET } {
                    write!(
                        l,
                        "lsm: mag({},{},{})\r\n",
                        meas[0], meas[1], meas[2]
                    )
                    .unwrap();
                }
            }
            Err(e) => {
                write!(l, "Err meas: {:?}", e).unwrap();
            }
        };
        match lsm303.unscaled_mag::<[i16; 3]>() {
            Ok(meas) => {
                if unsafe { !QUIET } {
                    write!(
                        l,
                        "lsm: unscmag({},{},{})\r\n",
                        meas[0], meas[1], meas[2],
                    )
                    .unwrap();
                }
            }
            Err(e) => {
                write!(l, "Err meas: {:?}", e).unwrap();
            }
        }
    }
}

unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
    match opt {
        Some(ref mut x) => &mut *x,
        None => panic!("extract"),
    }
}

#[interrupt]
fn USART1_EXTI25() {
    let rx = unsafe { extract(&mut RX) };
    let l = unsafe { extract(&mut TX) };
    match rx.read() {
        Ok(b) => {
            if b == TURN_QUIET {
                unsafe {
                    QUIET = !QUIET;
                }
            } else {
                // echo byte as is
                write!(l, "{}", b as char).unwrap();
            }
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
                write!(l, "read error: {:?}", e).unwrap();
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
    match unsafe { &mut TX } {
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
