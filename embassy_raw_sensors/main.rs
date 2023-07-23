#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(type_alias_impl_trait)]

use asm_delay::AsmDelay;
use core::fmt::Write;
use defmt;
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::exti::Channel;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Speed, Input, Pull};
use embassy_stm32::dma::NoDma;
use embassy_stm32::time::{mhz, Hertz};
use embassy_stm32::{bind_interrupts, peripherals, spi, usart};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Instant;
use heapless::String;
use mpu9250::Mpu9250;

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
async fn reader(mut rx: usart::UartRx<'static, peripherals::USART2, peripherals::DMA1_CH6>) {
    defmt::info!("starting reader loop");
    let mut msg: [u8; 1] = [0; 1];
    loop {
        match rx.read(&mut msg).await {
            Ok(_) => {
                defmt::info!("received: {}", msg[0]);
                if msg[0] == TOGGLE_QUIET {
                    defmt::info!("toggling quiet");
                    QUIET.signal(ToggleQuiet);
                } else {
                    // echo byte as is
                }
            },
            Err(e) => {
                defmt::error!("read error: {:?}", defmt::Debug2Format(&e));
                    // TODO: log to usart too
            },
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    defmt::info!("Starting MPU Embassy demo!");

    let mut config = embassy_stm32::Config::default();
    let sysclk = 64_000_000;
    config.rcc.hse = None;
    config.rcc.sysclk = Some(Hertz(sysclk));
    config.rcc.pclk1 = Some(Hertz(32_000_000));
    config.rcc.pclk2 = Some(Hertz(32_000_000));
    let device = embassy_stm32::init(config);
    defmt::info!("Device initialized!");

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
        device.DMA1_CH7,
        device.DMA1_CH6,
        usart_config,
    );
    let (mut tx, rx) = usart.split();
    defmt::info!("Usart initialized!");
    log_to_usart!(tx, log_buf, "usart ok!\r\n");
    log_to_usart!(tx, log_buf, "starting USART interrupt reader task!\r\n");
    defmt::unwrap!(spawner.spawn(reader(rx)));
    defmt::info!("Started reader!");

    let mut spi_config = spi::Config::default();
    spi_config.mode = mpu9250::MODE;
    let spi = spi::Spi::new(
        device.SPI1,
        device.PA5, // scl_sck
        device.PB5, // mosi
        device.PB4, // miso
        NoDma,
        NoDma,
        mhz(1),
        spi_config,
    );

    log_to_usart!(tx, log_buf, "spi ok!\r\n");
    defmt::info!("Spi ok!");

    // TODO: use embassy impl of Delay
    let mut delay = AsmDelay::new(asm_delay::bitrate::Hertz(sysclk));
    log_to_usart!(tx, log_buf, "delay ok!\r\n");

    let ncs_pin = Output::new(device.PB0, Level::High, Speed::Low);
    let gyro_rate = mpu9250::GyroTempDataRate::DlpfConf(mpu9250::Dlpf::_2);
    let mut mpu = Mpu9250::imu(
        spi,
        ncs_pin,
        &mut delay,
        &mut mpu9250::MpuConfig::imu()
            .gyro_temp_data_rate(gyro_rate)
            .sample_rate_divisor(3))
        .unwrap();

    defmt::unwrap!(tx.blocking_write(b"mpu ok !\r\n"));

    let accel_biases: [f32; 3] = match mpu.calibrate_at_rest(&mut delay) {
        Ok(ab) => ab,
        Err(e) => {
            log_to_usart!(tx, log_buf, "mpu calib err  {:?}!\r\n", e);
            defmt::panic!("mpu calib error: {:?}", defmt::Debug2Format(&e));
        }
    };
    log_to_usart!(tx, log_buf, "calib ok  {:?}!\r\n", accel_biases);
    defmt::info!("calib ok!");

    mpu.enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN).unwrap();
    let enabled_int = mpu.get_enabled_interrupts();
    defmt::info!("mpu int enabled; now: {:?}", defmt::Debug2Format(&enabled_int));

    let mpu_interrupt_pin = Input::new(device.PA11, Pull::Up);
    let mut mpu_interrupt_pin = ExtiInput::new(mpu_interrupt_pin.degrade(),
                                               device.EXTI13.degrade());
    defmt::info!("mpupin enabled");

    let mut prev_t_ms = Instant::now().as_millis();

    log_to_usart!(
        tx,
        log_buf,
        "All ok, now: {:?}; Press 'q' to toggle verbosity!\r\n",
        prev_t_ms
    );
    defmt::info!("all ok, starting loop!");

    let mut quiet = true;
    loop {
        mpu_interrupt_pin.wait_for_any_edge().await;
        defmt::info!("interrupt!");
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
                    log_to_usart!(
                        tx,
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
                    defmt::trace!("Measured mpu");
                }
                if QUIET.signaled() {
                    quiet = !quiet;
                    defmt::info!("Signaled quiet: new state: {}", quiet);
                    QUIET.reset();
                }
            }
            Err(e) => {
                log_to_usart!(tx, log_buf, "Err: {:?}; {:?}", t_ms, e);
                defmt::error!("mpu error!");
            }
        }
    }
}
