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
    prphs.RCC.apb2enr.write(|w| w.tim1en().enabled());
    prphs.RCC.ahbenr.write(|w| w.iopaen().enabled());

    prphs.GPIOA.moder.write(|w| w.moder8().alternate());
    unsafe {
        prphs.GPIOA.afrh.write(|w| w.afrh8().bits(0b0000_0110));
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
