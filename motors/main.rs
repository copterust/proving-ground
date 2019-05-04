#![deny(warnings)]
#![no_std]
#![no_main]

// used to provide panic_implementation
#[allow(unused)]
use panic_abort;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use hal::gpio::{MediumSpeed, PullUp};
use hal::prelude::*;
use hal::timer;

use cortex_m::asm;

#[entry]
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let clocks = rcc.cfgr
                    .sysclk(64.mhz())
                    .pclk1(32.mhz())
                    .freeze(&mut flash.acr);
    let ((ch1, ch2, ch3, ch4), mut tim1) =
        timer::tim2::Timer::new(device.TIM2, 1.mhz(), clocks).use_pwm();
    let ((ch5, ch6, _, _), mut tim2) =
        timer::tim3::Timer::new(device.TIM3, 1.mhz(), clocks).use_pwm();

    let mut motor_pa0 = gpioa.pa0.pull_type(PullUp).to_pwm(ch1, MediumSpeed);
    let mut motor_pa1 = gpioa.pa1.pull_type(PullUp).to_pwm(ch2, MediumSpeed);
    let mut motor_pa2 = gpioa.pa2.pull_type(PullUp).to_pwm(ch3, MediumSpeed);
    let mut motor_pa3 = gpioa.pa3.pull_type(PullUp).to_pwm(ch4, MediumSpeed);
    let mut motor_pa6 = gpioa.pa6.pull_type(PullUp).to_pwm(ch5, MediumSpeed);
    let mut motor_pa7 = gpioa.pa7.pull_type(PullUp).to_pwm(ch6, MediumSpeed);

    motor_pa0.enable();
    motor_pa1.enable();
    motor_pa2.enable();
    motor_pa3.enable();
    motor_pa6.enable();
    motor_pa7.enable();

    tim1.enable();
    tim2.enable();

    motor_pa0.set_duty(20);
    motor_pa1.set_duty(20);
    motor_pa2.set_duty(20);
    motor_pa3.set_duty(20);
    motor_pa6.set_duty(20);
    motor_pa7.set_duty(20);

    loop {
        tick_delay(25000);
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
