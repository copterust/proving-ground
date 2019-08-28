#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;

use asm_delay::AsmDelay;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::pac::interrupt;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;

use mpu9250::Mpu9250;

static mut L: Option<hal::serial::Tx<hal::pac::USART2>> = None;
static mut RX: Option<hal::serial::Rx<hal::pac::USART2>> = None;
static mut QUIET: bool = true;
static mut NOW_MS: u32 = 0;
const TURN_QUIET: u8 = 'q' as u8;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(64.mhz())
                    .pclk1(32.mhz())
                    .pclk2(32.mhz())
                    .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let mut serial =
        device.USART2
              .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
    let ser_int = serial.get_interrupt();
    serial.listen(serial::Event::Rxne);
    let (mut tx, rx) = serial.split();
    writeln!(tx, "tx ok").unwrap();
    unsafe {
        L = Some(tx);
        RX = Some(rx);
    };
    let l = unsafe { extract(&mut L) };
    writeln!(l, "logger ok").unwrap();
    // SPI1
    let ncs = gpiob.pb0.output().push_pull();
    let spi = device.SPI1.spi(// scl_sck, ad0_sd0_miso, sda_sdi_mosi,
                              (gpiob.pb3, gpiob.pb4, gpiob.pb5),
                              mpu9250::MODE,
                              1.mhz(),
                              clocks);
    writeln!(l, "spi ok").unwrap();
    let mut delay = AsmDelay::new(clocks.sysclk());
    writeln!(l, "delay ok").unwrap();
    let mut mpu = match Mpu9250::imu_default(spi, ncs, &mut delay) {
        Ok(m) => m,
        Err(e) => {
            writeln!(l, "Mpu init error: {:?}", e).unwrap();
            panic!("mpu err");
        }
    };
    writeln!(l, "mpu ok").unwrap();
    let accel_biases = match mpu.calibrate_at_rest(&mut delay) {
        Ok(ab) => ab,
        Err(e) => {
            writeln!(l, "Mpu calib error: {:?}", e).unwrap();
            panic!("mpu err");
        }
    };
    writeln!(l, "calibration ok: {:?}", accel_biases).unwrap();
    let mut syst = core.SYST;
    unsafe { cortex_m::interrupt::enable() };
    let reload = (clocks.sysclk().0 / 1000) - 1;
    syst.set_reload(reload);
    syst.clear_current();
    syst.enable_interrupt();
    syst.enable_counter();
    unsafe { cortex_m::peripheral::NVIC::unmask(ser_int) };

    let mut prev_t_ms = now_ms();
    write!(l,
           "All ok, now: {:?}; Press 'q' to toggle verbosity!\r\n",
           prev_t_ms).unwrap();
    loop {
        let t_ms = now_ms();
        let dt_ms = t_ms.wrapping_sub(prev_t_ms);
        prev_t_ms = t_ms;
        match mpu.all() {
            Ok(meas) => {
                let gyro = meas.gyro;
                let accel = meas.accel - accel_biases;
                if unsafe { !QUIET } {
                    write!(
                        l,
                        "IMU: t:{}ms; dt:{}ms; g({};{};{}); a({};{};{})\r\n",
                        t_ms, dt_ms, gyro.x, gyro.y, gyro.z, accel.x, accel.y, accel.z
                    )
                    .unwrap();
                }
            }
            Err(e) => {
                write!(l, "Err: {:?}; {:?}", t_ms, e).unwrap();
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

fn now_ms() -> u32 {
    unsafe { core::ptr::read_volatile(&NOW_MS as *const u32) }
}

#[exception]
unsafe fn SysTick() {
    NOW_MS = NOW_MS.wrapping_add(1);
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    let l = extract(&mut L);
    write!(l, "hard fault at {:?}", ef).unwrap();
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    let l = extract(&mut L);
    write!(l, "Interrupt: {}", irqn).unwrap();
    panic!("Unhandled exception (IRQn = {})", irqn);
}
