#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;
use rtfm::app;

use hal::gpio::{LowSpeed, Output, PullNone, PushPull};
use hal::prelude::*;
use hal::time::Bps;
use heapless::consts::*;
use heapless::Vec;

type USART = hal::pac::USART1;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C4;
type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut OUT_BUFFER: TxBuffer = Vec::new();
const HALF: usize = 32;
static mut IN_BUFFER: [[u8; crate::HALF]; 2] = [[0; HALF]; 2];

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
        let bf = unsafe { &mut OUT_BUFFER };
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

fn fill_with_bytes(buffer: &mut TxBuffer, arg: &[u8]) {
    buffer.extend_from_slice(arg).unwrap();
}

fn fill_with_str(buffer: &mut TxBuffer, arg: &str) {
    buffer.extend_from_slice(arg.as_bytes()).unwrap();
}


#[app(device = hal::pac)]
const APP: () = {
    static mut LED: hal::gpio::PA5<PullNone, Output<PushPull, LowSpeed>> = ();
    static mut TELE: Option<DmaTelemetry> = ();
    static mut CB: hal::dma::CircBuffer<[u8; HALF], hal::dma::dma1::C5> = ();

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
        // USART1
        let serial =
            device.USART1
                  .serial((gpioa.pa9, gpioa.pa10), Bps(460800), clocks);
        let (mut tx, rx) = serial.split();
        write!(tx, "init...\r\n").unwrap();
        let mut dma_channels = device.DMA1.split(&mut rcc.ahb);
        write!(tx, "dma...\r\n").unwrap();
        let tele = DmaTelemetry::create(dma_channels.4, tx);
        let new_tele = tele.send(|b| fill_with_str(b, "Dma ok!\r\n"));
        let mut led = gpioa.pa5.output().pull_type(PullNone);
        let _ = led.set_high();
        dma_channels.5.listen(hal::dma::Event::HalfTransfer);
        dma_channels.5.listen(hal::dma::Event::TransferComplete);
        let ib = unsafe { &mut IN_BUFFER };
        let cb = rx.circ_read(dma_channels.5, ib);

        init::LateResources { LED: led,
                              CB: cb,
                              TELE: Some(new_tele) }
    }

    #[interrupt(binds = DMA1_CH5, resources = [LED, CB, TELE])]
    fn handle_read(ctx: handle_read::Context) {
        let led = ctx.resources.LED;
        let maybe_tele = ctx.resources.TELE.take();
        let cb = ctx.resources.CB;
        let mut some_new_tele = None;
        if let Some(tele) = maybe_tele {
            let _ = cb.peek(|buf, _half| {
                let new_tele = tele.send(|b| fill_with_bytes(b, buf));
                some_new_tele = Some(new_tele);
            });
        }
        if let Some(new_tele) = some_new_tele {
            *ctx.resources.TELE = Some(new_tele);
        }

        let _ = led.set_low();
    }
};
