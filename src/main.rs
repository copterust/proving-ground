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
        prphs.GPIOA.ospeedr.write(|w| w.ospeedr8().bits(0b10));

        prphs.TIM1.ccer.write(|w| w.cc1e().set_bit());
        prphs.TIM1.ccer.write(|w| w.cc2e().set_bit());
        prphs.TIM1.ccer.write(|w| w.cc3e().set_bit());
        prphs.TIM1.ccer.write(|w| w.cc4e().set_bit());

        prphs.TIM1.arr.write(|w| w.bits(100));
        prphs.TIM1.psc.write(|w| w.bits(8));
        prphs.TIM1.bdtr.write(|w| w.moe().set_bit());
        prphs.TIM1.cr1.write(|w| w.cen().set_bit());
        prphs.TIM1.ccr1.write(|w| w.bits(6553));
    }

    loop {
//        unsafe {
//            prphs.TIM1.ccr1.write(|w| w.bits(50));
//        }
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
