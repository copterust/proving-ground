// STM32F303K8 Nucleo
// SCL: D13 
// SDA: D12
// AD0: D11
// NCS: D3
conf! {
    dev: device.SPI1,
    scl: gpiob.pb3,
    miso: gpiob.pb4,
    mosi: gpiob.pb5,   
    cs_mpu: gpiob.pb0, 
};
