#![deny(warnings)]
#![no_std]
#![no_main]

#[allow(unused)]
use panic_abort;

use core::f32::{INFINITY, NEG_INFINITY};
use core::fmt::Write;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::delay;
use hal::prelude::*;
use hal::time::Bps;

use mpu9250::Mpu9250;

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(64.mhz())
                    .pclk1(32.mhz())
                    .pclk2(36.mhz())
                    .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    let serial = device.USART1
                       .serial((gpioa.pa9, gpioa.pa10), Bps(115200), clocks);
    let (mut tx, _rx) = serial.split();
    // COBS frame
    tx.write(0x00).unwrap();
    let mut l = tx;
    write!(l, "logger ok\r\n").unwrap();
    let mut delay = delay::Delay::new(core.SYST, clocks);
    // SPI1
    let ncs = gpiob.pb9.output().push_pull();
    let scl_sck = gpiob.pb3;
    let sda_sdi_mosi = gpiob.pb5;
    let ad0_sdo_miso = gpiob.pb4;
    let spi = device.SPI1.spi((scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                              mpu9250::MODE,
                              1.mhz(),
                              clocks);
    let mut mpu = Mpu9250::marg_default(spi, ncs, &mut delay).unwrap();

    let sample_count = 500;
    let mut mag_max_x: f32 = NEG_INFINITY;
    let mut mag_max_y: f32 = NEG_INFINITY;
    let mut mag_max_z: f32 = NEG_INFINITY;
    let mut mag_min_x: f32 = INFINITY;
    let mut mag_min_y: f32 = INFINITY;
    let mut mag_min_z: f32 = INFINITY;

    let [mag_sensitivity_x, mag_sensitivity_y, mag_sensitivity_z] =
        mpu.mag_sensitivity_adjustments::<[f32; 3]>();
    write!(l,
           "factory sensitivity adjustments: {:?}, {:?}, {:?}\r\n",
           mag_sensitivity_x, mag_sensitivity_y, mag_sensitivity_z).unwrap();

    write!(
        l,
        "Mag Calibration: Wave device in a figure eight until done!\r\n"
    ).unwrap();
    delay.delay_ms(200u32);

    for _ in 0..sample_count {
        match mpu.mag::<[f32; 3]>() {
            Ok([mx, my, mz]) => {
                // x
                if mx > mag_max_x {
                    mag_max_x = mx;
                }
                if mx < mag_min_x {
                    mag_min_x = mx;
                }
                // y
                if my > mag_max_y {
                    mag_max_y = my;
                }
                if my < mag_min_y {
                    mag_min_y = my;
                }

                // z
                if mz > mag_max_z {
                    mag_max_z = mz;
                }
                if mz < mag_min_z {
                    mag_min_z = mz;
                }
            }
            Err(e) => {
                write!(l, "err: {:?}\r\n", e).unwrap();
            }
        }
        delay.delay_ms(5u32);
    }

    // Get hard iron correction
    let mag_avg_bias_x = ((mag_max_x + mag_min_x) as f32) / 2.; // get average x mag bias in counts
    let mag_avg_bias_y = ((mag_max_y + mag_min_y) as f32) / 2.; // get average y mag bias in counts
    let mag_avg_bias_z = ((mag_max_z + mag_min_z) as f32) / 2.; // get average z mag bias in counts

    let mag_res = mpu.mag_resolution();

    // save mag biases in G for main program
    let mag_bias_x = mag_avg_bias_x * mag_res * mag_sensitivity_x;
    let mag_bias_y = mag_avg_bias_y * mag_res * mag_sensitivity_y;
    let mag_bias_z = mag_avg_bias_z * mag_res * mag_sensitivity_z;

    // Get soft iron correction estimate
    let mag_scale_x = ((mag_max_x - mag_min_x) as f32) / 2.; // get average x axis max chord length in counts
    let mag_scale_y = ((mag_max_y - mag_min_y) as f32) / 2.; // get average y axis max chord length in counts
    let mag_scale_z = ((mag_max_z - mag_min_z) as f32) / 2.; // get average z axis max chord length in counts

    let mut avg_rad = mag_scale_x + mag_scale_y + mag_scale_z;
    avg_rad /= 3.0;

    let final_mag_scale_x = avg_rad / (mag_scale_x);
    let final_mag_scale_y = avg_rad / (mag_scale_y);
    let final_mag_scale_z = avg_rad / (mag_scale_z);

    write!(l,
           "loop done; bias: ({}, {}, {}); scale: ({}, {}, {})\r\n",
           mag_bias_x,
           mag_bias_y,
           mag_bias_z,
           final_mag_scale_x,
           final_mag_scale_y,
           final_mag_scale_z).unwrap();
    loop {}
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
