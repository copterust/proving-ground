#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use rtfm::app;

use hal::gpio::{LowSpeed, Output, PullNone, PushPull};
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
    static mut CB: hal::dma::CircBuffer<[u8; HALF], hal::dma::dma1::C6> = ();

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
        let serial =
            device.USART2
            .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
        let (tx, rx) = serial.split();
        let dma_channels = device.DMA1.split(&mut rcc.ahb);
        let tele = DmaTelemetry::create(dma_channels.7, tx);
        let new_tele = tele.send(|b| fill_with_str(b, "Dma ok!\r\n"));
        let mut led = gpioa.pa5.output().pull_type(PullNone);
        let _ = led.set_low();
        let mut rx_chan = dma_channels.6;
        rx_chan.listen(hal::dma::Event::HalfTransfer);
        rx_chan.listen(hal::dma::Event::TransferComplete);
        let ib = unsafe { &mut IN_BUFFER };
        let cb = rx.circ_read(rx_chan, ib);
        init::LateResources { LED: led,
                              CB: cb,
                              TELE: Some(new_tele) }
    }

    #[interrupt(binds = DMA1_CH6, resources = [LED, CB, TELE])]
    fn handle_read(ctx: handle_read::Context) {
        let led = ctx.resources.LED;
        let _ = led.toggle();
        let maybe_tele = ctx.resources.TELE.take();
        let cb = ctx.resources.CB;

        if let Some(tele) = maybe_tele {
            let mut msg = [0u8; 32];
            let ret = cb.peek(|buf, _half| {
                msg.copy_from_slice(buf);
            });
            match ret {
                Ok(()) => {
                    let new_tele = tele.send(|b| fill_with_bytes(b, &msg));
                    *ctx.resources.TELE = Some(new_tele);
                },
                Err(e) => {
                    cortex_m_semihosting::hprintln!("e: {:?}", e).unwrap();
                }
            }
        }

    }
};
