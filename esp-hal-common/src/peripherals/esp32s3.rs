use esp32s3 as pac;
// We need to export this for users to use
pub use pac::Interrupt;

// We need to export this in the hal for the drivers to use
pub(crate) use self::peripherals::*;

crate::peripherals! {
    AES,
    APB_CTRL,
    APB_SARADC,
    DEBUG_ASSIST,
    DMA,
    DS,
    EFUSE,
    EXTMEM,
    GPIO,
    GPIOSD,
    HMAC,
    I2C0,
    I2C1,
    I2S0,
    I2S1,
    INTERRUPT_CORE0,
    INTERRUPT_CORE1,
    IO_MUX,
    LCD_CAM,
    LEDC,
    PCNT,
    PERI_BACKUP,
    PWM0,
    PWM1,
    RMT,
    RNG,
    RSA,
    RTC_CNTL,
    RTC_I2C,
    RTCIO,
    SENS,
    SENSITIVE,
    SHA,
    SPI0,
    SPI1,
    SPI2,
    SPI3,
    SYSTEM,
    SYSTIMER,
    TIMG0,
    TIMG1,
    TWAI,
    UART0,
    UART1,
    UART2,
    UHCI0,
    UHCI1,
    USB0,
    USB_DEVICE,
    USB_WRAP,
    WCL,
    XTS_AES,
}

mod peripherals {
    pub use super::pac::*;

    crate::create_peripherals! {
        I2C0,
        I2C1,
        RNG,
        SHA,
        SPI0,
        SPI1,
        SPI2,
        SPI3,
        SYSTIMER,
        UART0,
        UART1,
        UART2,
        USB_DEVICE,
        SYSTEM,
        LEDC,
        RMT,
        I2S0,
        I2S1,
        PWM0,
        PWM1,
    }
}
