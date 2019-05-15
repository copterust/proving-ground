#![no_std]
#![no_main]

#[allow(unused)]
use panic_semihosting;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::hprintln;
// use dcmimu::DCMIMU;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use heapless::consts::*;
use heapless::Vec;
use nb;

const BUFFER_SIZE: usize = 256;
const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

pub struct Cmd {
    buffer: [u8; BUFFER_SIZE],
    pos: usize,
}

impl Cmd {
    #[inline]
    const fn new() -> Cmd {
        Cmd { buffer: [0; BUFFER_SIZE],
              pos: 0 }
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

type USART = hal::pac::USART2;
type TxUsart = hal::serial::Tx<USART>;
type TxCh = hal::dma::dma1::C7;
type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy =
    hal::dma::Transfer<hal::dma::R, &'static mut TxBuffer, TxCh, TxUsart>;
static mut BUFFER: TxBuffer = Vec::new();

#[derive(Debug, Clone, Copy)]
enum TransferState<Ready, Busy> {
    Ready(Ready),
    MaybeBusy(Busy),
}

struct DmaTelemetry<Ready, Busy> {
    state: TransferState<Ready, Busy>,
}

impl<Ready, Busy> DmaTelemetry<Ready, Busy> {
    fn with_state(ns: TransferState<Ready, Busy>) -> Self {
        DmaTelemetry { state: ns }
    }
}

impl DmaTelemetry<TxReady, TxBusy> {
    pub fn create(ch: TxCh, tx: TxUsart) -> Self {
        let bf = unsafe { &mut BUFFER };
        let state = TransferState::Ready((bf, ch, tx));
        DmaTelemetry::with_state(state)
    }

    fn send(self, arg: &[u8]) -> Self {
        let ns = match self.state {
            TransferState::Ready((mut buffer, ch, tx)) => {
                buffer.extend_from_slice(arg);
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

        DmaTelemetry::with_state(ns)
    }
}

#[entry]
fn main() -> ! {
    let device = hal::pac::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let mut flash = device.FLASH.constrain();
    let clocks = rcc.cfgr
                    .sysclk(64.mhz())
                    .pclk1(32.mhz())
                    .pclk2(36.mhz())
                    .freeze(&mut flash.acr);
    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    // USART1
    let mut serial =
        device.USART2
              .serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
    serial.listen(serial::Event::Rxne);
    let (mut tx, mut rx) = serial.split();
    let dma_channels = device.DMA1.split(&mut rcc.ahb);
    let mut cmd = Cmd::new();
    let mut tele = DmaTelemetry::create(dma_channels.7, tx);
    hprintln!("ready...").unwrap();
    loop {
        match nb::block!(rx.read()) {
            Ok(b) => {
                if let Some(word) = cmd.push(b) {
                    tele = tele.send(word);
                }
            }
            Err(e) => match e {
                serial::Error::Overrun => {
                    rx.clear_overrun_error();
                }
                serial::Error::Framing => {
                    rx.clear_framing_error();
                }
                serial::Error::Noise => {
                    rx.clear_noise_error();
                }
                _ => {
                    hprintln!("re: {:?}", e).unwrap();
                }
            },
        };
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("HardFault at {:#?}", ef);
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    hprintln!("Unhandled exception (IRQn = {})", irqn);
    panic!("Unhandled exception (IRQn = {})", irqn);
}
