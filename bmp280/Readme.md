# BMP280 pressure sensor

WIP for i2c sensor. One can use serial-to-usb converter and minicom to get results: `minicom -D /dev/tty.usbserial-A20027Ve -b 9600`.

To build:

```bash
cargo -v build --bin bmp280 --features=with_hal
```
