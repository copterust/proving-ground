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
use hal::gpio::wip;
use hal::rcc::RccExt;
use wip::GpioExt;

use rt::ExceptionFrame;

use cortex_m::asm;

type PT = hal::gpio::wip::gpiob::PB6<
    hal::gpio::wip::PullUp,
    hal::gpio::wip::AltFn<
        hal::gpio::wip::AF2,
        hal::gpio::wip::PushPull,
        hal::gpio::wip::MediumSpeed,
    >,
>;

#[used]
static mut P: Option<PT> = None;

entry!(main);
fn main() -> ! {
    let device = hal::stm32f30x::Peripherals::take().unwrap();
    // let core = cortex_m::Peripherals::take().unwrap();
    let mut rcc = device.RCC.constrain();
    let gpiob = device.GPIOB.split(&mut rcc.ahb);

    // Turn on TIM4
    unsafe {
        let rcc = &*stm32f30x::RCC::ptr();
        rcc.apb1enr.write(|w| w.tim4en().enabled());
    }

    // Setup PORTB6
    let pb6 = gpiob
        .pb6
        .pull_type(wip::PullUp)
        .alternating(wip::AF2)
        .output_speed(wip::MediumSpeed)
        .output_type(wip::PushPull);
    unsafe {
        P = Some(pb6);
    }
    // ^ this is to avoid warnings about unused stuff and to avoid optimazing port away
    // in real program pb6 would be consumed by pwm, but that will come later

    // Setup TIM4 as PWM
    unsafe {
        // Set period
        device.TIM4.arr.write(|w| w.bits(49));
        // Set prescaler
        device.TIM4.psc.write(|w| w.bits(71));
        // Enable output for channel 1
        device.TIM4.ccer.write(|w| w.cc1e().bit(true));
        // Set channel 1 as PWM1
        device.TIM4.ccmr1_output.write(|w| w.oc1m().bits(0b0110));
        // Enable timer
        device.TIM4.cr1.write(|w| w.cen().bit(true));
    }

    loop {
        for i in 10..50 {
            unsafe {
                device.TIM4.ccr1.write(|w| w.bits(i));
                tick_delay(25000);
            }
        }
        for i in 10..50 {
            unsafe {
                device.TIM4.ccr1.write(|w| w.bits(50 - i));
                tick_delay(25000);
            }
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
