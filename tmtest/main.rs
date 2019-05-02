#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;

use hal::gpio;
use hal::prelude::*;
use hal::timer::tim4;

use cortex_m_rt::{entry, exception, ExceptionFrame};

#[entry]
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    let mut beeper = gpiob
        .pb3
        .pull_type(gpio::PullUp)
        .output()
        .output_type(gpio::PushPull);
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .freeze(&mut flash.acr);
    let mut tim4 = tim4::Timer::new(device.TIM4, 8888.hz(), clocks);
    let mut b = true;
    loop {
        tim4.start(1.hz());
        while let Err(nb::Error::WouldBlock) = tim4.wait() {}
        if b {
            beeper.set_high();
            b = false;
        } else {
            beeper.set_low();
            b = true
        }
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
