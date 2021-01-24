#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;

use hal::gpio::{Input, LowSpeed, Output, PullNone, PullUp, PushPull};
use hal::prelude::*;
use hal::time::Bps;
use heapless::consts::*;
use heapless::Vec;
use rtic::cyccnt::U32Ext as _;

type USART = hal::pac::USART2;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C7;
type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut BUFFER: TxBuffer = Vec(heapless::i::Vec::new());

const FAST: u32 = 8_000_000;

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
        led: hal::gpio::PA5<PullNone, Output<PushPull, LowSpeed>>,
        extih: hal::exti::BoundInterrupt<
            hal::gpio::PA0<PullUp, Input>,
            hal::exti::EXTI1,
        >,
        tele: Option<DmaTelemetry>,
        fast_calibration: bool,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let device = ctx.device;
        let mut rcc = device.RCC.constrain();
        let mut flash = device.FLASH.constrain();
        let clocks = rcc
            .cfgr
            .sysclk(64.mhz())
            .pclk1(32.mhz())
            .freeze(&mut flash.acr);
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
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

        let dma_channels = device.DMA1.split(&mut rcc.ahb);
        write!(tx, "dma...\r\n").unwrap();
        let tele = DmaTelemetry::create(dma_channels.7, tx);
        let new_tele = tele.send(|b| fill_with_str(b, "Dma ok!\r\n"));
        let mut led = gpioa.pa5.output().pull_type(PullNone);
        let _ = led.set_high();

        calibrate::schedule(ctx.start + FAST.cycles()).unwrap();

        init::LateResources {
            led,
            extih: handle,
            tele: Some(new_tele),
            fast_calibration: true,
        }
    }

    #[task(resources = [tele])]
    fn calibrate(mut ctx: calibrate::Context) {
        ctx.resources.tele.lock(|maybe_tele| {
            if let Some(tele) = maybe_tele.take() {
                let new_tele = tele.send(|b| fill_with_str(b, "timer!\r\n"));
                *maybe_tele = Some(new_tele);
            }
        });

        calibrate::schedule(ctx.scheduled + FAST.cycles()).unwrap();
    }

    #[task(binds=EXTI0, resources = [led, tele, extih])]
    fn handle_mpu(mut ctx: handle_mpu::Context) {
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
