// #![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(type_alias_impl_trait)]

// # use core::fmt::Write;
use defmt::*;
use {defmt_rtt as _, panic_probe as _};
// use asm_delay::AsmDelay;
use cortex_m_rt::entry;

// use embassy_executor::Spawner;
use embassy_executor::Executor;

use embassy_stm32::time::Hertz;
use embassy_stm32::dma::NoDma;
use embassy_stm32::{bind_interrupts, peripherals, usart};
use static_cell::StaticCell;
// use mpu9250::Mpu9250;

// static mut L: Option<hal::serial::Tx<hal::pac::USART2>> = None;
// static mut RX: Option<hal::serial::Rx<hal::pac::USART2>> = None;
static mut QUIET: bool = true;
static mut NOW_MS: u32 = 0;
const TURN_QUIET: u8 = 'q' as u8;


bind_interrupts!(struct Irqs {
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("Hello World!");

    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        unwrap!(spawner.spawn(main_task()));
    })
}

#[embassy_executor::task]
async fn main_task() -> ! {
    let mut config = embassy_stm32::Config::default();
    config.rcc.hse = Some(Hertz(8_000_000));
    config.rcc.sysclk = Some(Hertz(64_000_000));
    config.rcc.pclk1 = Some(Hertz(32_000_000));
    config.rcc.pclk2 = Some(Hertz(32_000_000));
    let device = embassy_stm32::init(config);

    let mut usart_config = usart::Config::default();
    usart_config.baudrate = 460800;
    let mut usart = usart::Uart::new(device.USART2,
                                     device.PA15,
                                     device.PA2,
                                     Irqs, NoDma, NoDma,
                                     usart_config);
    unwrap!(usart.blocking_write(b"Hello Embassy World!\r\n"));
    info!("wrote Hello, starting echo");

    let mut buf = [0u8; 1];
    loop {
        unwrap!(usart.blocking_read(&mut buf));
        unwrap!(usart.blocking_write(&buf));
    }

    // let ser_int = serial.get_interrupt();
    // serial.listen(serial::Event::Rxne);
    // let (mut tx, rx) = serial.split();
    // writeln!(tx, "tx ok").unwrap();
    // unsafe {
    //     L = Some(tx);
    //     RX = Some(rx);
    // };
    // let l = unsafe { extract(&mut L) };
    // writeln!(l, "logger ok").unwrap();
    // // SPI1
    // let ncs = gpiob.pb0.output().push_pull();
    // let spi = device.SPI1.spi(
    //     // scl_sck, ad0_sd0_miso, sda_sdi_mosi,
    //     (gpiob.pb3, gpiob.pb4, gpiob.pb5),
    //     mpu9250::MODE,
    //     1.mhz(),
    //     clocks,
    // );
    // writeln!(l, "spi ok").unwrap();
    // let mut delay = AsmDelay::new(clocks.sysclk());
    // writeln!(l, "delay ok").unwrap();
    // let mut mpu = match Mpu9250::imu_default(spi, ncs, &mut delay) {
    //     Ok(m) => m,
    //     Err(e) => {
    //         writeln!(l, "Mpu init error: {:?}", e).unwrap();
    //         panic!("mpu err");
    //     }
    // };
    // writeln!(l, "mpu ok").unwrap();
    // let accel_biases: [f32; 3] = match mpu.calibrate_at_rest(&mut delay) {
    //     Ok(ab) => ab,
    //     Err(e) => {
    //         writeln!(l, "Mpu calib error: {:?}", e).unwrap();
    //         panic!("mpu err");
    //     }
    // };
    // writeln!(l, "calibration ok: {:?}", accel_biases).unwrap();
    // let mut syst = core.SYST;
    // unsafe { cortex_m::interrupt::enable() };
    // let reload = (clocks.sysclk().0 / 1000) - 1;
    // syst.set_reload(reload);
    // syst.clear_current();
    // syst.enable_interrupt();
    // syst.enable_counter();
    // unsafe { cortex_m::peripheral::NVIC::unmask(ser_int) };

    // let mut prev_t_ms = now_ms();
    // let mut prev_s = prev_t_ms / 1000;
    // write!(
    //     l,
    //     "All ok, now: {:?}; Press 'q' to toggle verbosity!\r\n",
    //     prev_t_ms
    // )
    // .unwrap();
    // loop {
    //     let t_ms = now_ms();
    //     let t_s = t_ms / 1000;
    //     let passed = t_s != prev_s;
    //     prev_s = t_s;
    //     let dt_ms = t_ms.wrapping_sub(prev_t_ms);
    //     prev_t_ms = t_ms;
    //     match mpu.all::<[f32; 3]>() {
    //         Ok(meas) => {
    //             let gyro = meas.gyro;
    //             let accel = [
    //                 meas.accel[0] - accel_biases[0],
    //                 meas.accel[1] - accel_biases[1],
    //                 meas.accel[2] - accel_biases[2],
    //             ];
    //             if unsafe { !QUIET } || passed {
    //                 write!(
    //                     l,
    //                     "IMU: t:{}ms; dt:{}ms; g({};{};{}); a({};{};{})\r\n",
    //                     t_ms,
    //                     dt_ms,
    //                     gyro[0],
    //                     gyro[1],
    //                     gyro[2],
    //                     accel[0],
    //                     accel[1],
    //                     accel[2]
    //                 )
    //                 .unwrap();
    //             }
    //         }
    //         Err(e) => {
    //             write!(l, "Err: {:?}; {:?}", t_ms, e).unwrap();
    //         }
    //     }
    // }
}

// unsafe fn extract<T>(opt: &'static mut Option<T>) -> &'static mut T {
//     match opt {
//         Some(ref mut x) => &mut *x,
//         None => defmt::panic!("extract"),
//     }
// }

// #[interrupt]
// fn USART2_EXTI26() {
//     let rx = unsafe { extract(&mut RX) };
//     let l = unsafe { extract(&mut L) };
//     match rx.read() {
//         Ok(b) => {
//             if b == TURN_QUIET {
//                 unsafe {
//                     QUIET = !QUIET;
//                 }
//             } else {
//                 // echo byte as is
//                 write!(l, "{}", b as char).unwrap();
//             }
//         }
//         Err(nb::Error::WouldBlock) => {}
//         Err(nb::Error::Other(e)) => match e {
//             serial::Error::Overrun => {
//                 rx.clear_overrun_error();
//             }
//             serial::Error::Framing => {
//                 rx.clear_framing_error();
//             }
//             serial::Error::Noise => {
//                 rx.clear_noise_error();
//             }
//             _ => {
//                 write!(l, "read error: {:?}", e).unwrap();
//             }
//         },
//     };
// }

// fn now_ms() -> u32 {
//     unsafe { core::ptr::read_volatile(&NOW_MS as *const u32) }
// }

// #[exception]
// unsafe fn SysTick() {
//     NOW_MS = NOW_MS.wrapping_add(1);
// }
