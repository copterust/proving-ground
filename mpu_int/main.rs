#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use asm_delay::AsmDelay;
use cortex_m_semihosting::hprintln;
use hal::gpio::{
    self, AltFn, HighSpeed, Input, Output, PullNone, PullUp, PushPull, AF5,
};
use hal::prelude::*;
use hal::spi::Spi;

use mpu9250::{Mpu9250, MpuConfig};

type SpiT = hal::pac::SPI1;
type SCLPin<B> = gpio::PB3<PullNone, B>;
type MISOPin<B> = gpio::PB4<PullNone, B>;
type MOSIPin<B> = gpio::PB5<PullNone, B>;
type SpiPins = (SCLPin<AltFn<AF5, PushPull, HighSpeed>>,
                MISOPin<AltFn<AF5, PushPull, HighSpeed>>,
                MOSIPin<AltFn<AF5, PushPull, HighSpeed>>);
type SPI = Spi<SpiT, SpiPins>;
type NcsPinDef<B> = gpio::PB0<PullNone, B>;
type NcsPinT = NcsPinDef<Output<PushPull, HighSpeed>>;
type Dev = mpu9250::SpiDevice<SPI, NcsPinT>;
type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;

#[rtic::app(device = hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        extih: hal::exti::BoundInterrupt<hal::gpio::PA0<PullUp, Input>,
                                         hal::exti::EXTI1>,
        mpu: MPU9250,
    }

    #[init()]
    fn init(ctx: init::Context) -> init::LateResources {
        let device = ctx.device;
        let mut rcc = device.RCC.constrain();
        let mut flash = device.FLASH.constrain();
        let clocks = rcc.cfgr
                        .sysclk(64.mhz())
                        .pclk1(32.mhz())
                        .freeze(&mut flash.acr);
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpiob = device.GPIOB.split(&mut rcc.ahb);

        let mut syscfg = device.SYSCFG.constrain(&mut rcc.apb2);
        let exti = device.EXTI.constrain();
        let interrupt_pin = gpioa.pa0.pull_type(PullUp).input();
        hprintln!("init ok").unwrap();

        // SPI1
        let ncs_pin = gpiob.pb0.output().push_pull().output_speed(HighSpeed);
        let scl_sck = gpiob.pb3;
        let sda_sdi_mosi = gpiob.pb5;
        let ad0_sdo_miso = gpiob.pb4;
        let spi = device.SPI1.spi((scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                                  mpu9250::MODE,
                                  1.mhz(),
                                  clocks);
        hprintln!("spi ok").unwrap();
        // MPU
        // 8Hz
        let gyro_rate = mpu9250::GyroTempDataRate::DlpfConf(mpu9250::Dlpf::_2);

        let mut delay = AsmDelay::new(clocks.sysclk());
        let mut mpu9250 = Mpu9250::imu_with_reinit(
            spi,
            ncs_pin,
            &mut delay,
            &mut MpuConfig::imu()
                .gyro_temp_data_rate(gyro_rate)
                .sample_rate_divisor(3),
            |spi, ncs| {
                let (dev_spi, (scl, miso, mosi)) = spi.free();
                let new_spi = dev_spi.spi(
                    (scl, miso, mosi),
                    mpu9250::MODE,
                    20.mhz(),
                    clocks,
                );
                Some((new_spi, ncs))
            },
        ).unwrap();
        hprintln!("mpu ok").unwrap();

        mpu9250.enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN)
               .unwrap();
        hprintln!("int enabled").unwrap();

        let extih = exti.EXTI1.bind(interrupt_pin, &mut syscfg);
        hprintln!("int bound").unwrap();

        init::LateResources { extih,
                              mpu: mpu9250 }
    }

    #[task(binds=EXTI0, resources = [mpu, extih])]
    fn handle_mpu(ctx: handle_mpu::Context) {
        let mpu = ctx.resources.mpu;
        match mpu.all::<[f32; 3]>() {
            Ok(a) => {
                hprintln!("[a:({:?},{:?},{:?}),g:({:?},{:?},{:?}),t:{:?}]",
                          a.accel[0],
                          a.accel[1],
                          a.accel[2],
                          a.gyro[0],
                          a.gyro[1],
                          a.gyro[2],
                          a.temp,).unwrap();
            }
            Err(e) => {
                hprintln!("e: {:?}", e).unwrap();
            }
        }
        ctx.resources.extih.unpend();
    }
};
