#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(used)]

extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate panic_abort;

extern crate stm32f30x;
extern crate stm32f30x_hal as hal;
use hal::flash::FlashExt;
// use hal::gpio;

use hal::prelude::*;
use hal::pwm::PwmBinding;
use hal::timer;

use rt::ExceptionFrame;

use cortex_m::asm;

#[used]
static mut P: Option<i32> = None;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let gpiob = device.GPIOB.split(&mut rcc.ahb);
    // get port b
    let pb6 = gpiob.pb6;
    let clocks = rcc
        .cfgr
        .sysclk(64.mhz())
        .pclk1(32.mhz())
        .freeze(&mut flash.acr);
    let tim4 = timer::tim4::Timer::new(device.TIM4, 650.khz(), clocks, &mut rcc.apb1);
    let (ch1, mut tim4) = tim4.take_ch1();
    tim4.enable();
    let mut pwm = PwmBinding::bind_pb6_tim4_ch1(pb6, ch1);
    pwm.enable();

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
