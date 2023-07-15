// #![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(type_alias_impl_trait)]

use asm_delay::AsmDelay;
use defmt;
use core::fmt::Write;
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use heapless::String;
use embassy_stm32::dma::NoDma;
use embassy_time::Instant;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::time::{mhz, Hertz};
use embassy_stm32::{bind_interrupts, peripherals, spi, usart};
use mpu9250::Mpu9250;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;


struct ToggleQuiet;
static QUIET: Signal<CriticalSectionRawMutex, ToggleQuiet> = Signal::new();
const TOGGLE_QUIET: u8 = 'q' as u8;

bind_interrupts!(struct Irqs {
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});


macro_rules! log_to_usart {
    ($sink: ident, $buf: ident, $($args:tt)+) => ({
        core::write!(&mut $buf, $($args)+).unwrap();
        defmt::unwrap!($sink.blocking_write($buf.as_bytes()));
        // TODO: don't clear until full? %)
        $buf.clear();
    });
}


#[embassy_executor::task]
async fn reader(mut rx: usart::UartRx<'static, peripherals::USART2, NoDma>) {
    loop {
        match rx.nb_read() {
            Ok(b) => {
                if b == TOGGLE_QUIET {
                    QUIET.signal(ToggleQuiet);
                } else {
                    // echo byte as is
                }
            }
            Err(nb::Error::WouldBlock) => {}
            Err(nb::Error::Other(e)) => match e {
                // seems embassy clears errors automatically
                usart::Error::Overrun => {
                }
                usart::Error::Framing => {
                }
                usart::Error::Noise => {
                }
                _ => {
                    defmt::error!("read error: {:?}", defmt::Debug2Format(&e));
                    // TODO: log to usart too
                }
            },
        };
    }
}


#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    defmt::info!("Starting MPU Embassy demo!");

    let mut config = embassy_stm32::Config::default();
    let sysclk = 64_000_000;
    config.rcc.hse = Some(Hertz(8_000_000));
    config.rcc.sysclk = Some(Hertz(sysclk));
    config.rcc.pclk1 = Some(Hertz(32_000_000));
    config.rcc.pclk2 = Some(Hertz(32_000_000));
    let device = embassy_stm32::init(config);

    let mut log_buf: String<128> = String::new();

    let mut usart_config = usart::Config::default();
    usart_config.detect_previous_overrun = false;
    usart_config.baudrate = 460800;
    usart_config.assume_noise_free = true;
    let usart = usart::Uart::new(
        device.USART2,
        device.PA15,
        device.PA2,
        Irqs,
        NoDma,
        NoDma,
        usart_config,
    );
    let (mut tx, rx) = usart.split();
    log_to_usart!(tx, log_buf, "usart ok!\r\n");
    log_to_usart!(tx, log_buf, "starting USART interrupt reader task!\r\n");
    defmt::unwrap!(spawner.spawn(reader(rx)));

    let spi = spi::Spi::new(
        device.SPI1,
        device.PB3, // scl_sck
        device.PB5, // mosi
        device.PB4, // miso
        NoDma,
        NoDma,
        mhz(1),
        spi::Config::default(),
    );

    log_to_usart!(tx, log_buf, "spi ok!\r\n");

    let mut delay = AsmDelay::new(asm_delay::bitrate::Hertz(sysclk));
    log_to_usart!(tx, log_buf, "delay ok!\r\n");

    let ncs = Output::new(device.PB0, Level::High, Speed::Low);
    let mut mpu = match Mpu9250::imu_default(spi, ncs, &mut delay) {
        Ok(m) => m,
        Err(e) => {
            log_to_usart!(tx, log_buf, "mpu_err  {:?}!\r\n", e);
            defmt::panic!("mpu init error: {:?}", defmt::Debug2Format(&e));
        }
    };
    defmt::unwrap!(tx.blocking_write(b"mpu ok !\r\n"));

    let accel_biases: [f32; 3] = match mpu.calibrate_at_rest(&mut delay) {
        Ok(ab) => ab,
        Err(e) => {
            log_to_usart!(tx, log_buf, "mpu calib err  {:?}!\r\n", e);
            defmt::panic!("mpu calib error: {:?}", defmt::Debug2Format(&e));
        }
    };
    log_to_usart!(tx, log_buf, "calib ok  {:?}!\r\n", accel_biases);

    let mut prev_t_ms = Instant::now().as_millis();

    log_to_usart!(tx, log_buf, "All ok, now: {:?}; Press 'q' to toggle verbosity!\r\n",
                  prev_t_ms);

    // TODO: read mpu on interrupt; await signals here
    let mut quiet = false;
    loop {
        let t_ms = Instant::now().as_millis();
        let dt_ms = t_ms.wrapping_sub(prev_t_ms);
        prev_t_ms = t_ms;
        match mpu.all::<[f32; 3]>() {
            Ok(meas) => {
                let gyro = meas.gyro;
                let accel = [
                    meas.accel[0] - accel_biases[0],
                    meas.accel[1] - accel_biases[1],
                    meas.accel[2] - accel_biases[2],
                ];
                if !quiet {
                    log_to_usart!(tx,
                                  log_buf,
                        "IMU: t:{}ms; dt:{}ms; g({};{};{}); a({};{};{})\r\n",
                        t_ms,
                        dt_ms,
                        gyro[0],
                        gyro[1],
                        gyro[2],
                        accel[0],
                        accel[1],
                        accel[2]
                    );
                }
                if QUIET.signaled() {
                    quiet = !quiet;
                    QUIET.reset();
                }
            }
            Err(e) => {
                log_to_usart!(tx, log_buf, "Err: {:?}; {:?}", t_ms, e);
            }
        }
    }
}
