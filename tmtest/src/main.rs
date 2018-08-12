#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;

use hal::gpio;
use hal::prelude::*;
use hal::timer::tim4;

use rt::{entry, exception, ExceptionFrame};

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    // get port b
    let mut pb8 = gpiob
        .pb8
        .pull_type(gpio::PullUp)
        .output()
        .output_type(gpio::PushPull);
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .freeze(&mut flash.acr);
    let mut tim4 = tim4::Timer::new(device.TIM4, 8888.hz(), clocks, &mut rcc.apb1);
    let mut b = true;
    loop {
        tim4.start(1.hz());
        while let Err(nb::Error::WouldBlock) = tim4.wait() {}
        if b {
            pb8.set_high();
            b = false;
        } else {
            pb8.set_low();
            b = true
        }
    }
}

exception!(HardFault, hard_fault);
fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);
fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
