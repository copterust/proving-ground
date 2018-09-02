#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(panic_handler)]

use core::fmt::{self, Write};
use core::intrinsics;
use core::panic::PanicInfo;

use hal::prelude::*;
use hal::time::Bps;
use hal::{delay, serial};
use nb;
use rt::{entry, exception};
use stm32f30x::interrupt;
use cortex_m::peripheral::syst::SystClkSource;

use mpu9250::Mpu9250;
use dcmimu::DCMIMU;

static mut L: Option<Logger<hal::serial::Tx<hal::stm32f30x::USART1>>> = None;
static mut RX: Option<hal::serial::Rx<hal::stm32f30x::USART1>> = None;
static mut QUIET: bool = false;
const TURN_QUIET: u8 = 'q' as u8;
static mut NOW_MS: u32 = 0;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    let mut flash = device.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .pclk2(32.mhz())
        .freeze(&mut flash.acr);

    let mut serial = device
        .USART1
        .serial((gpioa.pa9, gpioa.pa10), Bps(115200), clocks);
    let ser_int = serial.get_interrupt();
    serial.listen(serial::Event::Rxne);
    let (mut tx, rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    unsafe {
        L = Some(Logger { tx });
        RX = Some(rx);
    };
    let l = unsafe { extract(&mut L) };
    write!(l, "logger ok\r\n");
    let mut delay = delay::Delay::new(core.SYST, clocks);
    // SPI1
    let ncs = gpiob.pb9.output().push_pull();
    let spi = device.SPI1.spi(
        // scl_sck, ad0_sd0_miso, sda_sdi_mosi,
        (gpiob.pb3, gpiob.pb4, gpiob.pb5),
        mpu9250::MODE,
        1.mhz(),
        clocks,
    );
    write!(l, "spi ok\r\n");
    let mut mpu = Mpu9250::imu_default(spi, ncs, &mut delay).expect("mpu error");
    write!(l, "mpu ok\r\n");
    let mut accel_biases = mpu.calibrate_at_rest(&mut delay).expect("calib error");
    // Correct axis for gravity;
    accel_biases.z -= mpu9250::G;
    write!(l, "calibration ok: {:?}\r\n", accel_biases);

    let mut dcmimu = DCMIMU::new();
    let mut syst = delay.free();
    unsafe { cortex_m::interrupt::enable() };
    syt_tick_config(&mut syst, clocks.sysclk().0/1000);
    let mut nvic = core.NVIC;
    nvic.enable(ser_int);

    let mut prev_t_ms = now_ms();
    write!(l, "All ok, now: {:?}; Press 'q' to toggle logging!\r\n", prev_t_ms);
    loop {
        match mpu.all() {
            Ok(meas) => {
                let gyro = meas.gyro;
                let accel = meas.accel - accel_biases;
                let t_ms = now_ms();
                let dt_ms = t_ms.wrapping_sub(prev_t_ms);
                prev_t_ms = t_ms;
                let dt_s = (dt_ms as f32) / 1000.;
                let dcm = dcmimu.update((gyro.x, gyro.y, gyro.z),
                                        (accel.x, accel.y, accel.z),
                                        dt_s);
                if unsafe { !QUIET } {
                    write!(l, "IMU: dt={}s; roll={}; yaw={}; pitch={}\r\n",
                           dt_s,
                           rad_to_degrees(dcm.roll),
                           rad_to_degrees(dcm.yaw),
                           rad_to_degrees(dcm.pitch));
                }
            }
            Err(e) => {
                write!(l, "Err: {:?}\r\n", e);
            }
        }
    }
}

fn rad_to_degrees(r: f32) -> f32 {
    (r * 180.) / 3.14159265359
}

unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
    match opt {
        Some(ref mut x) => &mut *x,
        None => panic!("extract"),
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

interrupt!(USART1_EXTI25, usart_exti25);
fn usart_exti25() {
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
                write!(l, "{}", b as char);
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
                write!(l, "read error: {:?}", e);
            }
        },
    };
}

fn syt_tick_config(syst: &mut cortex_m::peripheral::SYST,
                   ticks: u32) {
    syst.set_reload(ticks - 1);
    syst.clear_current();
    syst.set_clock_source(SystClkSource::Core);
    syst.enable_interrupt();
    syst.enable_counter();
}


fn now_ms() -> u32 {
    unsafe {
        NOW_MS
    }
}

exception!(SysTick, || {
    NOW_MS = NOW_MS.wrapping_add(1);
});

exception!(HardFault, |ef| {
    panic!("HardFault at {:#?}", ef);
});

exception!(*, |irqn| {
    panic!("Unhandled exception (IRQn = {})", irqn);
});


#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    match unsafe { &mut L } {
        Some(ref mut l) => {
            let payload = panic_info.payload().downcast_ref::<&str>();
            match (panic_info.location(), payload) {
                (Some(location), Some(msg)) => {
                    write!(l,
                           "\r\npanic in file '{}' at line {}: {:?}\r\n",
                           location.file(),
                           location.line(),
                           msg);
                },
                (Some(location), None) => {
                    write!(l,
                           "panic in file '{}' at line {}",
                           location.file(),
                           location.line());
                },
                (None, Some(msg)) => {
                    write!(l, "panic: {:?}", msg);
                },
                (None, None) => {
                    write!(l, "panic occured, no info available");
                },
            }
        },
        None => {},
    }
    unsafe { intrinsics::abort() }
}
