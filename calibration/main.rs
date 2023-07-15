#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use core::fmt::Write;
use core::intrinsics;
use core::panic::PanicInfo;

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
            .USART2
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
    let spi = device.SPI1.spi(
        // scl_sck, ad0_sd0_miso, sda_sdi_mosi,
        (gpioa.pa5, gpiob.pb4, gpiob.pb5),
        mpu9250::MODE,
        1.mhz(),
        clocks,
    );
    writeln!(l, "spi ok").unwrap();
    let mut delay = AsmDelay::new(clocks.sysclk());
    writeln!(l, "delay ok").unwrap();
    let mut mpu = match Mpu9250::marg_default(spi, ncs, &mut delay) {
        Ok(m) => m,
        Err(e) => {
            writeln!(l, "Mpu init error: {:?}", e).unwrap();
            panic!("mpu err");
        }
    };
    writeln!(l, "mpu ok").unwrap();
    let mag_health = mpu.magnetometer_healthy();
    writeln!(l, "Magnetometer Healthy: {:?}", mag_health);
    // let accel_biases: [f32; 3] = match mpu.calibrate_at_rest(&mut delay) {
    //     Ok(ab) => ab,
    //     Err(e) => {
    //         writeln!(l, "Mpu calib error: {:?}", e).unwrap();
    //         panic!("mpu err");
    //     }
    // };
    // writeln!(l, "calibration ok: {:?}", accel_biases).unwrap();
    let mut syst = core.SYST;
    unsafe { cortex_m::interrupt::enable() };
    let reload = clocks.sysclk().0 / 8000 - 1;
    syst.set_reload(reload);
    syst.clear_current();
    syst.enable_interrupt();
    syst.enable_counter();
    unsafe { cortex_m::peripheral::NVIC::unmask(ser_int) };

    let mut prev_t_ms = now_ms();

    write!(l, "{} {}\r\n", clocks.sysclk().0, reload).unwrap();

    // EEPROM this
    let mag_offs = [
        0., 0.,
        0.,
        // Dev1
        // 400.149575,
        // -75.52005,
        // -145.13623,

        // // Dev2
        // 505.4114185,
        // 503.12718,
        // 291.256415,
    ];

    loop {
        let t_ms = now_ms();
        let dt_ms = t_ms.wrapping_sub(prev_t_ms);
        prev_t_ms = t_ms;
        match mpu.all::<[f32; 3]>() {
            Ok(meas) => {
                let gyro = meas.gyro;
                // let accel = [
                //     meas.accel[0] - accel_biases[0],
                //     meas.accel[1] - accel_biases[1],
                //     meas.accel[2] - accel_biases[2],
                // ];

                let accel = meas.accel;
                let mag = [
                    (meas.mag[0] - mag_offs[0]),
                    (meas.mag[1] - mag_offs[1]),
                    (meas.mag[2] - mag_offs[2]),
                ];

                while now_ms() < t_ms + 100 {}
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
fn USART2_EXTI26() {
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
