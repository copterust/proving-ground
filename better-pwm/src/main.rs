#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(used)]

extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate panic_abort;

extern crate alt_stm32f30x_hal as hal;
extern crate stm32f30x;

use hal::gpio;
use hal::prelude::*;
use hal::timer;

use rt::ExceptionFrame;

use cortex_m::asm;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let pin = device.GPIOB.split(&mut rcc.ahb).pb8.pull_type(gpio::PullUp);
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .freeze(&mut flash.acr);
    let tim = timer::tim4::Timer::new(device.TIM4, 1.mhz(), clocks, &mut rcc.apb1);
    let (ch, mut tim) = tim.take_ch3();
    let mut pwm = pin.to_pwm(ch, gpio::MediumSpeed);
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

exception!(HardFault, hard_fault);
fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);
fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

fn tick_delay(ticks: usize) {
    (0..ticks).for_each(|_| asm::nop());
}
