#![deny(warnings)]
#![no_main]
#![no_std]

#[allow(unused)]
use panic_abort;

use asm_delay::bitrate::*;
use asm_delay::AsmDelay;
use ehal::blocking::delay::DelayMs;
use rtfm::app;
use stm32f3::stm32f303;

#[macro_use]
mod mcr;

#[app(device = stm32f3::stm32f303)]
const APP: () = {
    static mut DEVICE: stm32f303::Peripherals = ();
    static mut DELAY: AsmDelay = ();

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let mut _core: rtfm::Peripherals = ctx.core;

        let device: stm32f303::Peripherals = ctx.device;
        let delay = AsmDelay::new(32u32.mhz());

        device.RCC.ahbenr.modify(|_, w| w.iopaen().set_bit());
        device.GPIOA.moder.modify(|_, w| w.moder5().output());
        device.GPIOA.bsrr.write(|w| w.bs5().clear_bit());
        // flash!(device.GPIOA, delay);

        device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
        device.GPIOC.moder.modify(|_, w| w.moder13().input());
        device.GPIOC
              .pupdr
              .modify(|_, w| unsafe { w.pupdr13().bits(0b01) });
        // flash!(device.GPIOA, delay);

        device.RCC.apb2enr.write(|w| w.syscfgen().enabled());
        // flash!(device.GPIOA, delay);
        device.SYSCFG
              .exticr4
              .modify(|_, w| unsafe { w.exti13().bits(0b010) });

        device.EXTI.imr1.modify(|_, w| w.mr13().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr13().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr13().set_bit());

        init::LateResources { DEVICE: device,
                              DELAY: delay }
    }

    #[interrupt(binds=EXTI15_10, resources = [DEVICE, DELAY])]
    fn int(ctx: int::Context) {
        flash!(ctx.resources.DEVICE.GPIOA, ctx.resources.DELAY);
        ctx.resources
           .DEVICE
           .EXTI
           .pr1
           .modify(|_, w| w.pr13().set_bit());
    }
};
