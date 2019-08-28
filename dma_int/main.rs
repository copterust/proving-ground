#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;
use rtfm::app;

use hal::gpio::{LowSpeed, Output, PullNone, PullUp, PushPull};
use hal::prelude::*;
use hal::time::Bps;
use heapless::consts::*;
use heapless::Vec;

type USART = hal::pac::USART2;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C7;
type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut BUFFER: TxBuffer = Vec(heapless::i::Vec::new());

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
        where F: for<'a> FnMut<(&'a mut TxBuffer,), Output = ()>
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

#[app(device = hal::pac)]
const APP: () = {
    static mut LED: hal::gpio::PA5<PullNone, Output<PushPull, LowSpeed>> = ();
    static mut EXTIH: hal::exti::Exti<hal::exti::EXTI13> = ();
    static mut TELE: Option<DmaTelemetry> = ();

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let device: hal::pac::Peripherals = ctx.device;
        let mut rcc = device.RCC.constrain();
        let mut flash = device.FLASH.constrain();
        let clocks = rcc.cfgr
                        .sysclk(64.mhz())
                        .pclk1(32.mhz())
                        .freeze(&mut flash.acr);
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpioc = device.GPIOC.split(&mut rcc.ahb);
        // USART1
        let serial =
            device.USART2
                  .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
        let (mut tx, _) = serial.split();
        write!(tx, "init...\r\n").unwrap();
        let mut syscfg = device.SYSCFG.constrain(&mut rcc.apb2);
        write!(tx, "syscfg...\r\n").unwrap();
        let mut exti = device.EXTI.constrain();
        write!(tx, "exti...\r\n").unwrap();
        let interrupt_pin = gpioa.pa0.pull_type(PullUp).input();

        exti.EXTI1.bind(interrupt_pin, &mut syscfg);
        write!(tx, "bound...\r\n").unwrap();

        let dma_channels = device.DMA1.split(&mut rcc.ahb);
        write!(tx, "dma...\r\n").unwrap();
        let tele = DmaTelemetry::create(dma_channels.7, tx);
        let new_tele = tele.send(|b| fill_with_str(b, "Dma ok!\r\n"));
        let mut led = gpioa.pa5.output().pull_type(PullNone);
        let _ = led.set_high();

        init::LateResources { LED: led,
                              EXTIH: exti.EXTI13,
                              TELE: Some(new_tele) }
    }

    #[interrupt(binds=EXTI0, resources = [LED, TELE, EXTIH])]
    fn handle_mpu(ctx: handle_mpu::Context) {
        let led = ctx.resources.LED;
        let _ = led.set_low();
        let maybe_tele = ctx.resources.TELE.take();
        if let Some(tele) = maybe_tele {
            let new_tele = tele.send(|b| fill_with_str(b, "interrupt!\n"));
            *ctx.resources.TELE = Some(new_tele);
        }
        ctx.resources.EXTIH.unpend();
    }
};
