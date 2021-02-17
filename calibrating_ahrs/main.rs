#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;

use hal::gpio::{
    self, AltFn, HighSpeed, Input, LowSpeed, Output, PullNone, PullUp,
    PushPull, AF5,
};
use hal::prelude::*;
use hal::spi::Spi;
use hal::time::Bps;
use heapless::consts::*;
use heapless::Vec;
use rtic::cyccnt::U32Ext as _;

use asm_delay::{AsmDelay, CyclesToTime};
use mpu9250::{MargMeasurements, Mpu9250, MpuConfig};
use ryu;

type SpiT = hal::pac::SPI1;
type SCLPin<B> = gpio::PA5<PullNone, B>;
type MISOPin<B> = gpio::PB4<PullNone, B>;
type MOSIPin<B> = gpio::PB5<PullNone, B>;
type SpiPins = (
    SCLPin<AltFn<AF5, PushPull, HighSpeed>>,
    MISOPin<AltFn<AF5, PushPull, HighSpeed>>,
    MOSIPin<AltFn<AF5, PushPull, HighSpeed>>,
);
type SPI = Spi<SpiT, SpiPins>;
type NcsPinDef<B> = gpio::PB0<PullNone, B>;
type NcsPinT = NcsPinDef<Output<PushPull, HighSpeed>>;
type Dev = mpu9250::SpiDevice<SPI, NcsPinT>;
type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Marg>;

type USART = hal::pac::USART2;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C7;
type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut BUFFER: TxBuffer = Vec(heapless::i::Vec::new());

const FAST: u32 = 8_000_000;

pub trait Chrono: Sized {
    type Time;
    /// Get the last measurements without updating state
    fn last(&self) -> Self::Time;

    /// Starts new cycle
    fn reset(&mut self) {
        self.split_time_ms();
    }

    /// Get elapsed time (ms) since last measurement and start new cycle
    fn split_time_ms(&mut self) -> f32;

    /// Get elapsed time (s) since last measurement and start new cycle
    fn split_time_s(&mut self) -> f32 {
        self.split_time_ms() / 1000.
    }
}

pub struct DwtClock {
    cc: CyclesToTime,
    last: u32,
}

impl DwtClock {
    pub fn new(cc: CyclesToTime) -> Self {
        let dwt = unsafe { &(*cortex_m::peripheral::DWT::ptr()) };
        DwtClock {
            cc,
            last: dwt.cyccnt.read(),
        }
    }
}

impl Chrono for DwtClock {
    type Time = u32;

    fn last(&self) -> Self::Time {
        self.last
    }

    fn split_time_ms(&mut self) -> f32 {
        let dwt = unsafe { &(*cortex_m::peripheral::DWT::ptr()) };
        let now: u32 = dwt.cyccnt.read();
        let duration = now - self.last;
        self.last = now;
        self.cc.to_ms(duration)
    }
}

enum TransferState {
    Ready(TxReady),
    MaybeBusy(TxBusy),
}

pub struct DmaTelemetry {
    state: TransferState,
}

impl DmaTelemetry {
    fn with_state(ns: TransferState) -> Self {
        DmaTelemetry { state: ns }
    }
}

impl DmaTelemetry {
    pub fn create(ch: TxCh, tx: TxUsart) -> Self {
        let bf = unsafe { &mut BUFFER };
        let state = TransferState::Ready((bf, ch, tx));
        DmaTelemetry::with_state(state)
    }

    fn send<F>(self, mut buffer_filler: F) -> Self
    where
        F: for<'a> FnMut<(&'a mut TxBuffer,), Output = ()>,
    {
        let ns = match self.state {
            TransferState::Ready((mut buffer, ch, tx)) => {
                buffer_filler(&mut buffer);
                TransferState::MaybeBusy(tx.write_all(ch, buffer))
            }
            TransferState::MaybeBusy(transfer) => {
                if transfer.is_done() {
                    let (buffer, ch, tx) = transfer.wait();
                    buffer.clear();
                    TransferState::Ready((buffer, ch, tx))
                } else {
                    TransferState::MaybeBusy(transfer)
                }
            }
        };

        match ns {
            TransferState::MaybeBusy(_) => DmaTelemetry::with_state(ns),
            TransferState::Ready(_) => {
                DmaTelemetry::with_state(ns).send(buffer_filler)
            }
        }
    }
}

fn fill_with_str(buffer: &mut TxBuffer, arg: &str) {
    buffer.extend_from_slice(arg.as_bytes()).unwrap();
}

#[rtic::app(device = hal::pac,
            peripherals = true,
            dispatchers = [UART4_EXTI34],
            monotonic = rtic::cyccnt::CYCCNT)]
mod app {
    use super::*;

    #[resources]
    struct Resources {
        led: hal::gpio::PB3<PullNone, Output<PushPull, LowSpeed>>,
        extih: hal::exti::BoundInterrupt<
            hal::gpio::PA0<PullUp, Input>,
            hal::exti::EXTI1,
        >,
        tele: Option<DmaTelemetry>,
        #[task_local]
        timer: DwtClock,
        #[task_local]
        mpu: MPU9250,
        #[task_local]
        previous_sample: MargMeasurements<[f32; 3]>,
    }

    #[init()]
    fn init(mut ctx: init::Context) -> init::LateResources {
        let device = ctx.device;
        let mut rcc = device.RCC.constrain();
        let mut flash = device.FLASH.constrain();
        let clocks = rcc
            .cfgr
            .sysclk(64.mhz())
            .pclk1(32.mhz())
            .freeze(&mut flash.acr);
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpiob = device.GPIOB.split(&mut rcc.ahb);

        let serial =
            device
                .USART2
                .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
        let (mut tx, _) = serial.split();
        write!(tx, "init...\r\n").unwrap();
        let mut syscfg = device.SYSCFG.constrain(&mut rcc.apb2);
        write!(tx, "syscfg...\r\n").unwrap();
        let exti = device.EXTI.constrain();
        write!(tx, "exti...\r\n").unwrap();
        let interrupt_pin = gpioa.pa0.pull_type(PullUp).input();

        let handle = exti.EXTI1.bind(interrupt_pin, &mut syscfg);
        write!(tx, "bound...\r\n").unwrap();

        // SPI1
        let ncs = gpiob.pb0.output().push_pull().output_speed(HighSpeed);
        let scl_sck = gpioa.pa5;
        let sda_sdi_mosi = gpiob.pb5;
        let ad0_sdo_miso = gpiob.pb4;
        let spi = device.SPI1.spi(
            (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
            mpu9250::MODE,
            1.mhz(),
            clocks,
        );
        write!(tx, "spi...\r\n").unwrap();

        let mut delay = AsmDelay::new(clocks.sysclk());
        let gyro_rate = mpu9250::GyroTempDataRate::DlpfConf(mpu9250::Dlpf::_2);
        let mpu = Mpu9250::marg_with_reinit(
            spi,
            ncs,
            &mut delay,
            &mut MpuConfig::marg()
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
        )
        .unwrap();
        write!(tx, "mpu...\r\n").unwrap();

        let mut led = gpiob.pb3.output().pull_type(PullNone);
        let _ = led.set_high();
        write!(tx, "led...\r\n").unwrap();

        let dma_channels = device.DMA1.split(&mut rcc.ahb);
        write!(tx, "dma...\r\n").unwrap();
        let tele = DmaTelemetry::create(dma_channels.7, tx);
        let new_tele = tele.send(|b| fill_with_str(b, "Dma ok!\r\n"));

        ctx.core.DWT.enable_cycle_counter();

        calibrate::schedule(ctx.start + FAST.cycles()).unwrap();

        let timer = DwtClock::new(CyclesToTime::new(clocks.sysclk()));

        init::LateResources {
            led,
            extih: handle,
            tele: Some(new_tele),
            mpu,
            timer,
            previous_sample: MargMeasurements {
                accel: [0., 0., 0.],
                gyro: [0., 0., 0.],
                mag: [0., 0., 0.],
                temp: 0.,
            },
        }
    }

    #[task(resources = [tele, previous_sample, mpu, timer])]
    fn calibrate(mut ctx: calibrate::Context) {
        let timer = ctx.resources.timer;
        let mpu = ctx.resources.mpu;
        let previous = ctx.resources.previous_sample;

        ctx.resources.tele.lock(|maybe_tele| {
            let dt_s = timer.split_time_s();
            let sample = mpu.all::<[f32; 3]>().unwrap_or(*previous);
            if sample.accel != previous.accel
                || sample.gyro != previous.gyro
                || sample.mag != previous.mag
            {
                if let Some(tele) = maybe_tele.take() {
                    let new_tele = tele.send(|buffer| {
                        // ax,ay,az,gx,gy,gz,mx,my,mz,temp,dt_s
                        let flts = sample
                            .accel
                            .iter()
                            .chain(sample.gyro.iter())
                            .chain(sample.mag.iter())
                            .chain(core::iter::once(&sample.temp))
                            .chain(core::iter::once(&dt_s));
                        // ignore errors around buffer manipulation
                        for f in flts {
                            let _ = buffer.extend_from_slice(
                                ryu::Buffer::new().format(*f).as_bytes(),
                            );
                            let _ = buffer.push(b';');
                        }
                        let _ = buffer.push(b'\r');
                        let _ = buffer.push(b'\n');
                    });

                    *maybe_tele = Some(new_tele);
                }
            }
        });

        calibrate::schedule(ctx.scheduled + FAST.cycles()).unwrap();
    }

    #[task(binds=EXTI0, resources = [led, tele, extih])]
    fn handle_interrupt(mut ctx: handle_interrupt::Context) {
        ctx.resources.led.lock(|led| {
            let _ = led.set_low();
        });
        ctx.resources.tele.lock(|maybe_tele| {
            if let Some(tele) = maybe_tele.take() {
                let new_tele = tele.send(|b| fill_with_str(b, "interrupt!\n"));
                *maybe_tele = Some(new_tele);
            }
        });
        ctx.resources.extih.lock(|extih| extih.unpend());
    }
}
