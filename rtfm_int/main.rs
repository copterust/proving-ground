#![deny(warnings)]
#![no_main]
#![no_std]

#[allow(unused)]
use panic_abort;

use asm_delay::bitrate::*;
use asm_delay::AsmDelay;
use ehal::blocking::delay::DelayMs;
use rtic::app;
use stm32f3::stm32f303;

#[app(device = stm32f3::stm32f303, peripherals = true)]
mod app {
    use super::*;

    #[local]
    struct Local {
        device: stm32f303::Peripherals,
        delay: AsmDelay,
    }

    #[shared]
    struct Shared {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device: stm32f303::Peripherals = ctx.device;
        let delay = AsmDelay::new(32u32.mhz());

        // pa5 -- led
        device.RCC.ahbenr.modify(|_, w| w.iopaen().set_bit());
        device.GPIOA.moder.modify(|_, w| w.moder5().output());
        device.GPIOA.bsrr.write(|w| w.bs5().clear_bit());

        // pc13 -- interrupt
        device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
        device.GPIOC.moder.modify(|_, w| w.moder13().input());
        device
            .GPIOC
            .pupdr
            .modify(|_, w| unsafe { w.pupdr13().bits(0b01) });

        device.RCC.apb2enr.write(|w| w.syscfgen().enabled());
        device
            .SYSCFG
            .exticr4
            .modify(|_, w| unsafe { w.exti13().bits(0b010) });

        device.EXTI.imr1.modify(|_, w| w.mr13().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr13().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr13().set_bit());

        (Shared {}, Local { device, delay }, init::Monotonics())
    }

    #[task(binds=EXTI15_10, local = [device, delay])]
    fn int(ctx: int::Context) {
        for _ in 1..3 {
            ctx.local.device.GPIOA.bsrr.write(|w| w.bs5().set_bit());
            ctx.local.delay.delay_ms(100u32);
            ctx.local.device.GPIOA.brr.write(|w| w.br5().set_bit());
            ctx.local.delay.delay_ms(100u32);
        }
        ctx.local.device.EXTI.pr1.modify(|_, w| w.pr13().set_bit());
    }
}
