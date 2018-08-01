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

use hal::gpio::PullUp;
use hal::prelude::*;
use hal::pwm::PwmBinding;
use hal::timer;

use rt::ExceptionFrame;

use cortex_m::asm;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .freeze(&mut flash.acr);
    let (ch1, ch2, ch3, ch4, mut tim) =
        timer::tim2::Timer::new(device.TIM2, 1.mhz(), clocks, &mut rcc.apb1).take_all();
    let mut motor_pa0 = PwmBinding::bind_pa0_tim2_ch1(gpioa.pa0.pull_type(PullUp), ch1);
    let mut motor_pa1 = PwmBinding::bind_pa1_tim2_ch2(gpioa.pa1.pull_type(PullUp), ch2);
    let mut motor_pa2 = PwmBinding::bind_pa2_tim2_ch3(gpioa.pa2.pull_type(PullUp), ch3);
    let mut motor_pa3 = PwmBinding::bind_pa3_tim2_ch4(gpioa.pa3.pull_type(PullUp), ch4);
    motor_pa0.enable();
    motor_pa1.enable();
    motor_pa2.enable();
    motor_pa3.enable();
    tim.enable();

    motor_pa0.set_duty(20);
    motor_pa1.set_duty(20);
    motor_pa2.set_duty(20);
    motor_pa3.set_duty(20);

    loop {
        tick_delay(25000);
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
