#![deny(warnings)]
#![no_std]
#![no_main]

extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate panic_abort;

extern crate stm32f30x;

use rt::ExceptionFrame;

entry!(main);
fn main() -> !{
    let prphs = stm32f30x::Peripherals::take().unwrap();
    // Turn on PORTB
    prphs.RCC.ahbenr.write(|w| w.iopben().enabled());
    // Turn on TIM4
    prphs.RCC.apb1enr.write(|w| w.tim4en().enabled());
    // Setup PORTB6
    unsafe {
        // Medium speed
        prphs.GPIOB.ospeedr.write(|w| w.ospeedr6().bits(0x02));
        // Push-pull output
        prphs.GPIOB.otyper.write(|w| w.ot6().bit(false));
        // Alternative function mode
        prphs.GPIOB.moder.write(|w| w.moder6().alternate());
        // Pull-up resistor enabled
        prphs.GPIOB.pupdr.write(|w| w.pupdr6().bits(0x01));
        // AF2
        prphs.GPIOB.afrl.write(|w| w.afrl6().bits(0x02));
    }

    loop {

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
