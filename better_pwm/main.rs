#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;

use hal::gpio;
use hal::prelude::*;
use hal::timer;

use cortex_m_rt::{entry, exception, ExceptionFrame};

use cortex_m::asm;

#[entry]
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let pin = device.GPIOB.split(&mut rcc.ahb).pb8.pull_type(gpio::PullUp);
    let clocks = rcc.cfgr
                    .sysclk(64.mhz())
                    .pclk1(32.mhz())
                    .freeze(&mut flash.acr);
    let tim = timer::tim4::Timer::new(device.TIM4, 1.mhz(), clocks);
    let (channels, mut tim) = tim.use_pwm();
    let mut pwm = pin.to_pwm(channels.2, gpio::MediumSpeed);
    pwm.enable();
    tim.enable();

    loop {
        for i in 10..50 {
            pwm.set_duty(i);
            tick_delay(25000);
        }
        for i in 10..50 {
            pwm.set_duty(50 - i);
            tick_delay(25000);
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

fn tick_delay(ticks: usize) {
    (0..ticks).for_each(|_| asm::nop());
}
