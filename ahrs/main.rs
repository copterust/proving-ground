#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use core::fmt::Write;
use core::intrinsics;
use core::panic::PanicInfo;

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::pac::interrupt;
use hal::prelude::*;
use hal::time::Bps;
use hal::{delay, serial};
use nb;

use dcmimu::DCMIMU;
use mpu9250::{self, Mpu9250};

static mut L: Option<hal::serial::Tx<hal::pac::USART1>> = None;
static mut RX: Option<hal::serial::Rx<hal::pac::USART1>> = None;
static mut QUIET: bool = false;
const TURN_QUIET: u8 = 'q' as u8;
static mut NOW_MS: u32 = 0;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(64.mhz())
                    .pclk1(32.mhz())
                    .pclk2(32.mhz())
                    .freeze(&mut flash.acr);

    let mut serial =
        device.USART1
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
    // SPI1
    let ncs = gpiob.pb9.output().push_pull();
    let spi = device.SPI1.spi(// scl_sck, ad0_sd0_miso, sda_sdi_mosi,
                              (gpiob.pb3, gpiob.pb4, gpiob.pb5),
                              mpu9250::MODE,
                              1.mhz(),
                              clocks);
    write!(l, "spi ok\r\n").unwrap();
    let mut mpu = Mpu9250::imu(
        spi,
        ncs,
        &mut delay,
        mpu9250::MpuConfig::imu().accel_scale(mpu9250::AccelScale::_4G),
    )
    .expect("mpu error");
    write!(l, "mpu ok\r\n").unwrap();
    let mut accel_biases: [f32; 3] =
        mpu.calibrate_at_rest(&mut delay).expect("calib error");
    // Correct axis for gravity;
    accel_biases[2] -= mpu9250::G;
    write!(l, "calibration ok: {:?}\r\n", accel_biases).unwrap();

    let mut dcmimu = DCMIMU::new();
    let mut syst = delay.free();
    unsafe { cortex_m::interrupt::enable() };
    syt_tick_config(&mut syst, clocks.sysclk().0 / 1000);
    unsafe { cortex_m::peripheral::NVIC::unmask(ser_int) };

    let mut prev_t_ms = now_ms();
    write!(l,
           "All ok, now: {:?}; Press 'q' to toggle logging!\r\n",
           prev_t_ms).unwrap();
    loop {
        match mpu.all::<[f32; 3]>() {
            Ok(meas) => {
                let gyro = meas.gyro;
                let accel = [meas.accel[0] - accel_biases[0],
                             meas.accel[1] - accel_biases[1],
                             meas.accel[2] - accel_biases[2]];
                let t_ms = now_ms();
                let dt_ms = t_ms.wrapping_sub(prev_t_ms);
                prev_t_ms = t_ms;
                let dt_s = (dt_ms as f32) / 1000.;
                let (dcm, _biased) =
                    dcmimu.update((gyro[0], gyro[1], gyro[2]),
                                  (accel[0], accel[1], accel[2]),
                                  dt_s);
                if unsafe { !QUIET } {
                    write!(l,
                           "IMU: dt={}s; roll={}; yaw={}; pitch={}\r\n",
                           dt_s,
                           rad_to_degrees(dcm.roll),
                           rad_to_degrees(dcm.yaw),
                           rad_to_degrees(dcm.pitch)).unwrap();
                }
            }
            Err(e) => {
                write!(l, "Err: {:?}\r\n", e).unwrap();
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

fn syt_tick_config(syst: &mut cortex_m::peripheral::SYST, ticks: u32) {
    syst.set_reload(ticks - 1);
    syst.clear_current();
    syst.set_clock_source(SystClkSource::Core);
    syst.enable_interrupt();
    syst.enable_counter();
}

fn now_ms() -> u32 {
    unsafe { NOW_MS }
}

#[exception]
unsafe fn SysTick() {
    NOW_MS = NOW_MS.wrapping_add(1);
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
                    write!(l,
                           "\r\npanic in file '{}' at line {}: {:?}\r\n",
                           location.file(),
                           location.line(),
                           msg).unwrap();
                }
                (Some(location), None) => {
                    write!(l,
                           "panic in file '{}' at line {}",
                           location.file(),
                           location.line()).unwrap();
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
