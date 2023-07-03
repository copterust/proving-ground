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
    let accel_biases: [f32; 3] = match mpu.calibrate_at_rest(&mut delay) {
        Ok(ab) => ab,
        Err(e) => {
            writeln!(l, "Mpu calib error: {:?}", e).unwrap();
            panic!("mpu err");
        }
    };
    writeln!(l, "calibration ok: {:?}", accel_biases).unwrap();
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

    // // EEPROM this
    // let mag_offs = [
    //     0., 0., 0.,
    //     // Dev1
    //     // 400.149575,
    //     // -75.52005,
    //     // -145.13623,

    //     // // Dev2
    //     // 505.4114185,
    //     // 503.12718,
    //     // 291.256415,
    // ];

    let mut marg = ahrs::MargEkf::new();

    // magic
    let a_1 = [
        0.8584555038527345,
        0.10586940235249466,
        -0.03836484610343783,
        0.10586940235249466,
        0.9443944364324307,
        0.026852422933079816,
        -0.03836484610343783,
        0.026852422933079816,
        0.9564666809032134,
    ];
    let b = [-171.46636520969122, 440.13276013298366, -197.88863777137766];
    // end of magic

    loop {
        let t_ms = now_ms();
        let dt_ms = t_ms.wrapping_sub(prev_t_ms);
        prev_t_ms = t_ms;
        match mpu.all::<[f32; 3]>() {
            Ok(meas) => {
                let gyro = meas.gyro;

                let accel = meas.accel;
                let mag = [meas.mag[0], meas.mag[1], meas.mag[2]];
                let cal = calibrated_sample(&mag, &a_1, &b);

                marg.predict(
                    gyro[0],
                    gyro[1],
                    gyro[2],
                    (dt_ms as f32) / 1000.0,
                );
                marg.update(accel, cal);

                write!(
                    l,
                    "[{}, {:?}, {:?}, {:?}, {:?}, {:?}]\r\n",
                    dt_ms, accel, gyro, cal, marg.state, meas.mag
                )
                .unwrap();

                while now_ms() < t_ms + 20 {}
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

pub fn calibrated_sample(
    sample: &[f32; 3],
    a_1: &[f32; 9],
    b: &[f32; 3],
) -> [f32; 3] {
    let s = *sample;

    let sb = [s[0] - b[0], s[1] - b[1], s[2] - b[2]];

    let transformed_s = [
        a_1[0] * sb[0] + a_1[1] * sb[1] + a_1[2] * sb[2],
        a_1[3] * sb[0] + a_1[4] * sb[1] + a_1[5] * sb[2],
        a_1[6] * sb[0] + a_1[7] * sb[1] + a_1[8] * sb[2],
    ];

    transformed_s
}
