#![no_std]
#![no_main]
#![feature(unboxed_closures)]

#[allow(unused)]
use panic_semihosting;

use core::str::FromStr;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::hprintln;
use dcmimu::DCMIMU;
use hal::prelude::*;
use hal::serial;
use hal::time::Bps;
use heapless::consts::*;
use heapless::Vec;
use nb;

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

enum TransferState {
    Ready(TxReady),
    MaybeBusy(TxBusy),
}

struct DmaTelemetry {
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

fn fill_with_bytes(buffer: &mut TxBuffer, arg: &[u8]) {
    buffer.extend_from_slice(arg).unwrap();
}

fn fill_with_floats(buffer: &mut TxBuffer, arg: &[f32]) {
    for f in arg.into_iter() {
        let mut b = ryu::Buffer::new();
        let s = b.format(*f);
        buffer.extend_from_slice(s.as_bytes()).unwrap();
        buffer.push(',' as u8).unwrap();
    }
    buffer.push('\n' as u8).unwrap();
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
    let (tx, mut rx) = serial.split();
    let dma_channels = device.DMA1.split(&mut rcc.ahb);
    let mut cmd = Cmd::new();
    let mut tele = DmaTelemetry::create(dma_channels.7, tx);
    let mut dcm = DCMIMU::new();
    hprintln!("ready...").unwrap();
    loop {
        match nb::block!(rx.read()) {
            Ok(b) => {
                if let Some(word) = cmd.push(b) {
                    let v = unsafe { core::str::from_utf8_unchecked(word) };
                    let (acc, gyro, dt_s, (oy, op, or)) = parse(v);
                    let (ypr, _biased_gyro) = dcm.update(gyro, acc, dt_s);
                    let to_send = [ypr.yaw, ypr.pitch, ypr.roll, oy, op, or];
                    // let (ax, ay, az) = acc;
                    // let (gx, gy, gz) = gyro;
                    // let to_send = [ax, ay, az, gx, gy, gz, dt_s, oy, op, or];
                    // tele = tele.send(|b| fill_with_bytes(b, word))
                    tele = tele.send(|b| fill_with_floats(b, &to_send));
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

macro_rules! parse_assign {
    ($i:ident, $e: expr) => {{
        $i = f32::from_str($e).unwrap();
    }};
}

fn parse(inp: &str)
         -> ((f32, f32, f32), (f32, f32, f32), f32, (f32, f32, f32)) {
    let mut ax = 0.;
    let mut ay = 0.;
    let mut az = 0.;
    let mut gx = 0.;
    let mut gy = 0.;
    let mut gz = 0.;
    let mut dt_s = 0.;
    let mut y = 0.;
    let mut p = 0.;
    let mut r = 0.;
    let mut i = 0;
    for part in inp.split(" ") {
        match i {
            0 => parse_assign!(ax, part),
            1 => parse_assign!(ay, part),
            2 => parse_assign!(az, part),
            3 => parse_assign!(gx, part),
            4 => parse_assign!(gy, part),
            5 => parse_assign!(gz, part),
            6 => parse_assign!(dt_s, part),
            7 => parse_assign!(y, part),
            8 => parse_assign!(p, part),
            9 => parse_assign!(r, part),
            _ => {}
        }

        i += 1;
    }
    return ((ax, ay, az), (gx, gy, gz), dt_s, (y, p, r));
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
