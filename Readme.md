# Examples

Collection of tiny ARM programs

# General guidelines

* Install [rustup](https://www.rustup.rs/)
* Use nightly toolchain: `rustup default nightly`
* Install appropriate target: `rustup target add thumbv7em-none-eabi`
* Install toolchain: `sudo apt-get install gcc-arm-none-eabi` on Ubuntu or `brew cask install gcc-arm-embedded` on MacOS
* Install `bobbin-cli` to speed up development: `cargo install bobbin-cli`
* `cd` to exapmle dir and run `bobbin -v build` to build, or `bobbin -v load --bin <example name>` to flash device.


# Note on targets:

* Use `thumbv6m-none-eabi` for ARM Cortex-M0 and Cortex-M0+
* Use `thumbv7m-none-eabi` for ARM Cortex-M3
* Use `thumbv7em-none-eabi` for ARM Cortex-M4 and Cortex-M7 (*no* FPU support)
* Use `thumbv7em-none-eabihf` for ARM Cortex-M4**F** and Cortex-M7**F** (*with* FPU support)

You will have to change default target in `.cargo/config`.
