#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;
use rtic::app;

use hal::gpio::{Input, LowSpeed, Output, PullNone, PullUp, PushPull};
use hal::prelude::*;
use hal::time::Bps;
use heapless::Vec;

type USART = hal::pac::USART2;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C7;
type TxBuffer = Vec<u8, 256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut BUFFER: TxBuffer = Vec::new();

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

#[app(device = hal::pac, peripherals = true)]
mod app {
    use super::*;

    #[local]
    struct Local {
        led: hal::gpio::PA5<PullNone, Output<PushPull, LowSpeed>>,
        extih: hal::exti::BoundInterrupt<
            hal::gpio::PA0<PullUp, Input>,
            hal::exti::EXTI1,
        >,
        tele: Option<DmaTelemetry>,
    }

    #[shared]
    struct Shared {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
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

        (
            Shared {},
            Local {
                led,
                extih: handle,
                tele: Some(new_tele),
            },
            init::Monotonics(),
        )
    }

    #[task(binds=EXTI0, local = [led, tele, extih])]
    fn handle_mpu(ctx: handle_mpu::Context) {
        let _ = ctx.local.led.set_low();
        if let Some(tele) = ctx.local.tele.take() {
            let new_tele = tele.send(|b| fill_with_str(b, "interrupt!\n"));
            *ctx.local.tele = Some(new_tele);
        }
        ctx.local.extih.unpend();
    }
}
