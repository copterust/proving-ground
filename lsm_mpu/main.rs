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
use hal::time::Bps;
use hal::{delay, serial};
use nb;

use lsm303c::Lsm303c;
use mpu9250::Mpu9250;

static mut L: Option<hal::serial::Tx<hal::pac::USART1>> = None;
static mut RX: Option<hal::serial::Rx<hal::pac::USART1>> = None;
static mut QUIET: bool = true;
const TURN_QUIET: u8 = 'q' as u8;

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
        L = Some(tx);
        RX = Some(rx);
    };
    let l = unsafe { extract(&mut L) };
    write!(l, "logger ok\r\n").unwrap();
    let mut delay = delay::Delay::new(core.SYST, clocks);
    // I2C
    let i2c = device.I2C1.i2c((gpiob.pb6, gpiob.pb7), 400.khz(), clocks);
    write!(l, "i2c ok\r\n").unwrap();
    // lsm
    let mut lsm303 = Lsm303c::default(i2c).expect("lsm error");
    write!(l, "lsm ok\r\n").unwrap();
    // SPI1
    let ncs = gpiob.pb9.output().push_pull();
    let spi = device.SPI1.spi(
        // scl_sck, ad0_sd0_miso, sda_sdi_mosi,
        (gpiob.pb3, gpiob.pb4, gpiob.pb5),
        mpu9250::MODE,
        1.mhz(),
        clocks,
    );
    write!(l, "spi ok\r\n").unwrap();
    let mut mpu =
        Mpu9250::marg_default(spi, ncs, &mut delay).expect("mpu error");
    // done
    unsafe { cortex_m::interrupt::enable() };
    unsafe { cortex_m::peripheral::NVIC::unmask(ser_int) };
    write!(l, "All ok; Press 'q' to toggle verbosity!\r\n").unwrap();
    loop {
        let mlsm_meas = lsm303.all::<[f32; 3]>();
        let mmpu_meas = mpu.all::<[f32; 3]>();
        match (mlsm_meas, mmpu_meas) {
            (Ok(lsm_meas), Ok(mpu_meas)) => {
                if unsafe { !QUIET } {
                    write!(
                        l,
                        "lsm: mag({},{},{}); a({},{},{}); t({});\r\n",
                        lsm_meas.mag[0],
                        lsm_meas.mag[1],
                        lsm_meas.mag[2],
                        lsm_meas.accel[0],
                        lsm_meas.accel[1],
                        lsm_meas.accel[2],
                        lsm_meas.temp
                    )
                    .unwrap();
                    write!(
                        l,
                        "mpu: mag({},{},{}); a({},{},{}); t({});\r\n",
                        mpu_meas.mag[0],
                        mpu_meas.mag[1],
                        mpu_meas.mag[2],
                        mpu_meas.accel[0],
                        mpu_meas.accel[1],
                        mpu_meas.accel[2],
                        mpu_meas.temp
                    )
                    .unwrap();
                }
            }
            (Err(e), _) => {
                write!(l, "Err lsm meas: {:?}", e).unwrap();
            }
            (_, Err(e)) => {
                write!(l, "Err mpu meas: {:?}", e).unwrap();
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
    let l = unsafe { extract(&mut L) };
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
