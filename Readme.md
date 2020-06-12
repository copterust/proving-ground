# Examples

Collection of tiny programs for STM32 boards written in Rust. [![Build Status](https://travis-ci.org/copterust/proving-ground.svg?branch=master)](https://travis-ci.org/copterust/proving-ground)

# General guidelines

* Install [rustup](https://www.rustup.rs/)
* Use nightly toolchain: `rustup default nightly`
* Install appropriate target: `rustup target add thumbv7em-none-eabihf`
* Install toolchain: `sudo apt-get install gcc-arm-none-eabi` on Ubuntu or [via brew](https://github.com/eblot/homebrew-armeabi) on MacOS
* Install `bobbin-cli` to speed up development: `cargo install bobbin-cli`
* run `cargo -v build --bin <EXAMPLENAME>` to build, or `bobbin -v load --bin <example name>` to flash device.

Most of the examples depend on some features so command may fail,
but error message will contain name of the feature you need to enable: `cargo -v build --bin pwm features==with_device`.


# Note on targets:

* Use `thumbv6m-none-eabi` for ARM Cortex-M0 and Cortex-M0+
* Use `thumbv7m-none-eabi` for ARM Cortex-M3
* Use `thumbv7em-none-eabi` for ARM Cortex-M4 and Cortex-M7 (*no* FPU support)
* Use `thumbv7em-none-eabihf` for ARM Cortex-M4**F** and Cortex-M7**F** (*with* FPU support)

You will have to change default target in `.cargo/config`...
