#!/bin/sh

cargo -v build --bins --features=no_device
cargo -v build --bins --features=all
