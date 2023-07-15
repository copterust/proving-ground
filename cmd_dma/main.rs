#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_abort;

use core::fmt::Write;
use rtic::app;

use hal::gpio::{LowSpeed, Output, PullNone, PushPull};
use hal::prelude::*;
use hal::time::Bps;
use heapless::consts::*;
use heapless::spsc::{Consumer, Producer, Queue};
use heapless::Vec;

type USART = hal::pac::USART2;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C7;
type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut BUFFER: TxBuffer = Vec(heapless::i::Vec::new());
static mut QUEUE: Queue<u8, U16> = Queue(heapless::i::Queue::new());

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

const BUFFER_SIZE: usize = 512;
const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

pub struct Cmd {
    buffer: [u8; BUFFER_SIZE],
    pos: usize,
}

impl Cmd {
    #[inline]
    const fn new() -> Cmd {
        Cmd {
            buffer: [0; BUFFER_SIZE],
            pos: 0,
        }
    }

    #[inline]
    fn push(&mut self, b: u8) -> Option<&[u8]> {
        if b == CR || b == LF {
            if self.pos == 0 {
                None
            } else {
                let result = &self.buffer[0..self.pos];
                self.pos = 0;
                Some(result)
            }
        } else {
            self.buffer[self.pos] = b;
            self.pos = (self.pos + 1) & (BUFFER_SIZE - 1);
            None
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
    static mut RX: hal::serial::Rx<USART> = ();
    static mut CMD: Cmd = ();
    static mut P: Producer<'static, u8, U16> = ();
    static mut C: Consumer<'static, u8, U16> = ();

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        cortex_m_semihosting::hprintln!("init").unwrap();
        let device: hal::pac::Peripherals = ctx.device;
        let mut rcc = device.RCC.constrain();
        let mut flash = device.FLASH.constrain();
        let clocks = rcc
            .cfgr
            .sysclk(64.mhz())
            .pclk1(32.mhz())
            .freeze(&mut flash.acr);
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let mut serial =
            device
                .USART2
                .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
        cortex_m_semihosting::hprintln!("serial").unwrap();
        serial.listen(hal::serial::Event::Rxne);
        cortex_m_semihosting::hprintln!("listen").unwrap();
        let (mut tx, rx) = serial.split();
        write!(tx, "init...\r\n").unwrap();
        let dma_channels = device.DMA1.split(&mut rcc.ahb);
        write!(tx, "dma...\r\n").unwrap();
        let tele = DmaTelemetry::create(dma_channels.7, tx);
        let new_tele = tele.send(|b| fill_with_str(b, "Dma ok!\r\n"));
        let mut led = gpioa.pa5.output().pull_type(PullNone);
        let _ = led.set_low();
        cortex_m_semihosting::hprintln!("init done").unwrap();

        let (p, c) = unsafe { QUEUE.split() };
        init::LateResources {
            LED: led,
            RX: rx,
            CMD: Cmd::new(),
            TELE: Some(new_tele),
            P: p,
            C: c,
        }
    }

    #[idle(resources=[C, CMD, TELE])]
    fn idle(ctx: idle::Context) -> ! {
        let cmd = ctx.resources.CMD;
        loop {
            if let Some(byte) = ctx.resources.C.dequeue() {
                if let Some(word) = cmd.push(byte) {
                    let maybe_tele = ctx.resources.TELE.take();
                    if let Some(tele) = maybe_tele {
                        let new_tele = tele.send(|b| fill_with_bytes(b, word));
                        *ctx.resources.TELE = Some(new_tele);
                    }
                }
            }
        }
    }

    #[interrupt(binds=USART2_EXTI26, resources = [LED, RX, P])]
    fn handle_rx(ctx: handle_rx::Context) {
        let led = ctx.resources.LED;
        let rx = ctx.resources.RX;

        let _ = led.toggle();
        match rx.read() {
            Ok(b) => {
                if let Err(e) = ctx.resources.P.enqueue(b) {
                    cortex_m_semihosting::hprintln!("err: {:?}", e).unwrap();
                }
            }
            Err(e) => {
                cortex_m_semihosting::hprintln!("err: {:?}", e).unwrap();
            }
        }
    }
};
