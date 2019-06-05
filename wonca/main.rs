#![no_std]
#![no_main]

#[macro_use]
mod utils;
#[macro_use]
mod logger;

#[allow(unused)]
use panic_abort;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::delay;
use hal::prelude::*;
use hal::time::Bps;

use mpu9250::Mpu9250;
use nalgebra::Vector3;
use libm::F32Ext;

use logger::{Vs, Write};

const G: f32 = 9.80665;

fn accel_error<Dev, Imu>(mpu: &mut Mpu9250<Dev, Imu>, delay: &mut delay::Delay) -> Result<f32, Dev::Error>
where
    Dev: mpu9250::Device,
{
    let mut i = mpu.accel()?;
    let mut a = 0.0;
    for _ in 0..50 {
        delay.delay_ms(20u8);
        let j = mpu.accel()?;
        a += (j - i).norm();
        i = j;
    }
    Ok(a * 0.02)
}

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

    let spi_conf = conf! {
        dev: device.SPI1,
        scl: gpiob.pb3,
        miso: gpiob.pb4,
        mosi: gpiob.pb5,
        cs_mpu: gpiob.pb0,
    };

    let usart_conf = conf! {
        dev: device.USART2,
        tx: gpioa.pa2,
        rx: gpioa.pa15,
        bps: 460800,
    };

    let serial = usart_conf.dev.serial((usart_conf.tx, usart_conf.rx),
                                       Bps(usart_conf.bps),
                                       clocks);
    let (tx, _) = serial.split();

    logger::set_stdout(tx);

    print!("\x1b[H\x1b[J");
    println!("Getting ready");

    let mut delay = delay::Delay::new(core.SYST, clocks);

    // SPI1
    let cs_mpu = spi_conf.cs_mpu.output().push_pull();
    let spi = spi_conf.dev
                      .spi((spi_conf.scl, spi_conf.miso, spi_conf.mosi),
                           mpu9250::MODE,
                           1.mhz(),
                           clocks);

    println!("- spi");

    let mut mpu = match Mpu9250::imu_default(spi, cs_mpu, &mut delay) {
        Ok(m) => m,
        Err(e) => {
            println!("Mpu init error: {:?}", e);
            loop {}
        }
    };

    println!("- not calibrating mpu");

    println!("- calibrating rest position");

    let mut rest = 1.0;
    loop {
        let prev = rest;
        rest = lerp(0.1, rest, mpu.gyro().unwrap().norm());
        if (prev - rest).abs() < 0.0001 {
            break;
        }
    }

    println!("- mpu gyro rest norm: {}", rest);

    println!("- mpu ok");

    let mut readings = [[0.0f32; 3]; 6];

    println!("Calibrating using g0 = {}", G);

    println!("- calibrating noise");
    let noise_level = accel_error(&mut mpu, &mut delay).unwrap();
    println!(" = {}", noise_level);

    loop {
        for pos in 0..6 {
            println!("Put device in position {}", pos);
            let mut mov = 0.0;

            while mov < rest * 2.0 {
                mov = lerp(0.05, mov, mpu.gyro().unwrap().norm());
                print!("\r{:5.2}", mov);
                delay.delay_ms(20u8);
            }

            println!("\r- found movement, waiting to settle");

            loop {
                let error = accel_error(&mut mpu, &mut delay).unwrap();
                print!("\r{}", error);
                if error < noise_level * 1.0001 {
                    break;
                }
            }

            println!("\r- measuring, stay put");

            let mut r = mpu.accel().unwrap();
            for _ in 0..50 {
                r += mpu.accel().unwrap();
                delay.delay_ms(20u8);
            }

            r *= 0.02;

            println!("\r- ok, readings: {} = {:8.3}", Vs(r), r.norm());

            readings[pos] = [r[0], r[1], r[2]];
        }

        if let Some(adj) = estimate(&readings) {
            println!("Calibration result: {:?}", adj);
            for pos in 0..6 {
                let r = Vector3::from(readings[pos]);
                let a = Vector3::new(adj[0].estimate(r[0]),
                                     adj[1].estimate(r[1]),
                                     adj[2].estimate(r[2]));
                let err = (G - a.norm()).abs();
                println!(" - orig reading: {} = {}", Vs(r), r.norm());
                println!("   adjusted:     {} = {}, error: {}",
                         Vs(a),
                         a.norm(),
                         err);
            }
        } else {
            println!("Calibration failed, try again.");
        }
    }
}

fn lerp<A>(a: f32, x: A, y: A) -> A
    where A: core::ops::Mul<f32, Output = A> + core::ops::Add<A, Output = A>
{
    x * (1.0 - a) + y * a
}

fn estimate(vals: &[[f32; 3]]) -> Option<[won2010::Adj; 3]> {
    let mut cal = won2010::Cal::new(G, 0.1);
    for _ in 0..50 {
        if cal.step(vals) {
            return Some(cal.adj());
        }
    }
    None
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    println!("hard fault at {:?}", ef);
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    println!("Interrupt: {}", irqn);
    panic!("Unhandled exception (IRQn = {})", irqn);
}
