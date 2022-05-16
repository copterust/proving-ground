#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;

use cortex_m::asm;
use cortex_m_rt::{entry, exception, ExceptionFrame};
use stm32f3::stm32f303 as pac;

#[entry]
fn main() -> ! {
    let prphs = pac::Peripherals::take().unwrap();
    // Turn on PORTB
    prphs.RCC.ahbenr.write(|w| w.iopben().enabled());
    // Turn on TIM4
    prphs.RCC.apb1enr.write(|w| w.tim4en().enabled());
    // Setup PORTB6
    unsafe {
        // Medium speed
        prphs.GPIOB.ospeedr.write(|w| w.ospeedr8().bits(0x02));
        // Push-pull output
        prphs.GPIOB.otyper.write(|w| w.ot8().bit(false));
        // Alternative function mode
        prphs.GPIOB.moder.write(|w| w.moder8().alternate());
        // Pull-up resistor enabled
        prphs.GPIOB.pupdr.write(|w| w.pupdr8().bits(0x01));
        // AF2
        prphs.GPIOB.afrh.write(|w| w.afrh8().bits(0x02));
    }
    // Setup TIM4 as PWM
    unsafe {
        // Set period
        prphs.TIM4.arr.write(|w| w.bits(49));
        // Set prescaler
        prphs.TIM4.psc.write(|w| w.bits(71));
        // Enable output for channel 1
        prphs.TIM4.ccer.write(|w| w.cc3e().bit(true));
        // Set channel 1 as PWM1
        prphs.TIM4.ccmr2_output().write(|w| w.oc3m().bits(0b0110));
        // Enable timer
        prphs.TIM4.cr1.write(|w| w.cen().bit(true));
    }

    loop {
        for i in 10..50 {
            unsafe {
                prphs.TIM4.ccr3.write(|w| w.bits(i));
                tick_delay(25000);
            }
        }
        for i in 10..50 {
            unsafe {
                prphs.TIM4.ccr3.write(|w| w.bits(50 - i));
                tick_delay(25000);
            }
        }
    }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

fn tick_delay(ticks: usize) {
    (0..ticks).for_each(|_| asm::nop());
}
