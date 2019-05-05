use core::fmt;
pub use core::fmt::Write;
use nalgebra::Vector3;

use nb;

type Stream = hal::serial::Tx<hal::pac::USART2>;
type Stdout = Logger<Stream>;

static mut STDOUT: Option<Stdout> = None;

pub fn stdout() -> &'static mut Stdout {
    unsafe { STDOUT.as_mut().unwrap() }
}

pub fn set_stdout(s: Stream) {
    unsafe { STDOUT = Some(Logger::new(s)) }
}

macro_rules! print {
    ($($tt:tt)*) => (write!(crate::logger::stdout(), $($tt)*).unwrap());
}

macro_rules! println {
    ($($tt:tt)*) => (writeln!(crate::logger::stdout(), $($tt)*).unwrap());
}

pub struct Logger<W> {
    tx: W,
}

impl<W> Logger<W> {
    pub fn new(tx: W) -> Logger<W> {
        Logger { tx }
    }
}

impl<W: ehal::serial::Write<u8>> fmt::Write for Logger<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match self.write_char(c) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        match self.tx.flush() {
            Ok(_) => {}
            Err(_) => {}
        };

        Ok(())
    }

    fn write_char(&mut self, s: char) -> fmt::Result {
        match nb::block!(self.tx.write(s as u8)) {
            Ok(_) => {}
            Err(_) => {}
        }
        Ok(())
    }
}

pub struct Vs(pub Vector3<f32>);

impl fmt::Display for Vs {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:8.3} {:8.3} {:8.3}", self.0[0], self.0[1], self.0[2])
    }
}
