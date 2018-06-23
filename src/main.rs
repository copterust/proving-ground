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
    // Setup TIM4 as PWM
    unsafe {
        // Set period
        prphs.TIM4.arr.write(|w| w.bits(49));
        // Set prescaler
        prphs.TIM4.psc.write(|w| w.bits(71));
        // Enable output for channel 1
        prphs.TIM4.ccer.write(|w| w.cc1e().bit(true));
        // Set channel 1 as PWM1
        prphs.TIM4.ccmr1_output.write(|w| w.oc1m().bits(0b0110));
        // Enable timer
        prphs.TIM4.cr1.write(|w| w.cen().bit(true));
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
