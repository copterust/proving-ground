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
use hal::gpio::gpioa::PA0;
use hal::prelude::*;
use hal::pwm::PwmBinding;
use hal::timer;
use hal::timer::tim2;

use rt::ExceptionFrame;

use cortex_m::asm;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let gpioa = device.GPIOA.split(&mut rcc.ahb);
    // get port b
    let pin = gpioa.pa0.pull_type(gpio::PullUp);
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .freeze(&mut flash.acr);
    let tim = timer::tim2::Timer::new(device.TIM2, 1.mhz(), clocks, &mut rcc.apb1);
    let (ch, mut tim) = tim.take_ch1();
    // Two ways to create binding: via named func or via turbo fishing:
    // let mut pwm = PwmBinding::bind_pb6_tim4_ch1(pb6, ch1);
    let mut pwm = PwmBinding::<PA0<_, _>, tim2::Channel<timer::CH1, _>, gpio::AF1>::new(pin, ch);
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
