#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt;
use embassy_executor::Spawner;
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::{Duration, Timer};
use embassy_stm32::gpio::{Level, Output, Speed};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.hse = None;
    config.rcc.sysclk = Some(Hertz(64_000_000));
    config.rcc.pclk1 = Some(Hertz(32_000_000));
    config.rcc.pclk2 = Some(Hertz(32_000_000));
    let p = embassy_stm32::init(config);
    defmt::info!("embassy init");
    let mut led = Output::new(p.PB3, Level::Low, Speed::Low);
    led.set_high();
    let mut b = true;
    defmt::info!("embassy led");
    loop {
        Timer::after(Duration::from_secs(3)).await;
        if b {
            led.set_high();
        } else {
            led.set_low();
        }
        b = !b;
        defmt::error!("tick; next {}", if b { "high" } else { "low" });
    }
}
