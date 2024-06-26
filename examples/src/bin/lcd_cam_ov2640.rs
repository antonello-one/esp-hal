//! Drives the camera on a Freenove ESP32-S3 WROOM (also works as is on the ESP32S3-EYE)
//!
//! This example reads a JPEG from an OV2640 and writes it to the console as hex.
//!
//! Pins used:
//! SIOD      GPIO4
//! SIOC      GPIO5
//! XCLK      GPIO15
//! VSYNC     GPIO6
//! HREF      GPIO7
//! PCLK      GPIO13
//! D2        GPIO11
//! D3        GPIO9
//! D4        GPIO8
//! D5        GPIO10
//! D6        GPIO12
//! D7        GPIO18
//! D8        GPIO17
//! D9        GPIO16

//% CHIPS: esp32s3

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    dma::{Dma, DmaPriority},
    dma_buffers,
    gpio::Io,
    i2c,
    i2c::I2C,
    lcd_cam::{
        cam::{Camera, RxEightBits},
        LcdCam,
    },
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    Blocking,
};
use esp_println::println;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    let dma = Dma::new(peripherals.DMA);
    let channel = dma.channel0;

    let (_, _, rx_buffer, rx_descriptors) = dma_buffers!(0, 32678);

    let channel = channel.configure(false, DmaPriority::Priority0);

    let cam_siod = io.pins.gpio4;
    let cam_sioc = io.pins.gpio5;
    let cam_xclk = io.pins.gpio15;
    let cam_vsync = io.pins.gpio6;
    let cam_href = io.pins.gpio7;
    let cam_pclk = io.pins.gpio13;
    let cam_data_pins = RxEightBits::new(
        io.pins.gpio11,
        io.pins.gpio9,
        io.pins.gpio8,
        io.pins.gpio10,
        io.pins.gpio12,
        io.pins.gpio18,
        io.pins.gpio17,
        io.pins.gpio16,
    );

    let lcd_cam = LcdCam::new(peripherals.LCD_CAM);
    let mut camera = Camera::new(
        lcd_cam.cam,
        channel.rx,
        rx_descriptors,
        cam_data_pins,
        20u32.MHz(),
        &clocks,
    )
    .with_master_clock(cam_xclk)
    .with_ctrl_pins(cam_vsync, cam_href, cam_pclk);

    let delay = Delay::new(&clocks);

    let mut buffer = rx_buffer;
    buffer.fill(0u8);

    delay.delay_millis(500u32);

    let i2c = I2C::new(
        peripherals.I2C0,
        cam_siod,
        cam_sioc,
        100u32.kHz(),
        &clocks,
        None,
    );

    let mut sccb = Sccb::new(i2c);

    // Checking camera address
    sccb.probe(OV2640_ADDRESS).unwrap();
    println!("Probe successful!");

    sccb.write(OV2640_ADDRESS, 0xFF, 0x01).unwrap(); // bank sensor
    let pid = sccb.read(OV2640_ADDRESS, 0x0A).unwrap();
    println!("Found PID of {:#02X}, and was expecting 0x26", pid);

    // Start waiting for camera before initialising it to prevent missing the first few bytes.
    // This can be improved with a VSYNC interrupt but would complicate this example.
    let transfer = camera.read_dma(&mut buffer).unwrap();

    for (reg, value) in FIRST_BLOCK {
        sccb.write(OV2640_ADDRESS, *reg, *value).unwrap();
    }
    delay.delay_millis(10u32);
    for (reg, value) in SECOND_BLOCK {
        sccb.write(OV2640_ADDRESS, *reg, *value).unwrap();
        if *reg == 0xDD && *value == 0x7F {
            delay.delay_millis(10u32);
        }
    }

    transfer.wait().unwrap();

    // Note: JPEGs starts with "FF, D8, FF, E0" and end with "FF, D9"

    let index_of_end = buffer.windows(2).position(|c| c[0] == 0xFF && c[1] == 0xD9);
    let index_of_end = if let Some(idx) = index_of_end {
        idx + 2
    } else {
        println!("Failed to find JPEG terminator");
        buffer.len()
    };

    println!("Frame data (parse with `xxd -r -p <uart>.txt image.jpg`):");
    println!("{:02X?}", &buffer[..index_of_end]);

    loop {}
}

pub const OV2640_ADDRESS: u8 = 0x30;

pub struct Sccb<'d, T> {
    i2c: I2C<'d, T, Blocking>,
}

impl<'d, T> Sccb<'d, T>
where
    T: i2c::Instance,
{
    pub fn new(i2c: I2C<'d, T, Blocking>) -> Self {
        Self { i2c }
    }

    pub fn probe(&mut self, address: u8) -> Result<(), i2c::Error> {
        self.i2c.write(address, &[])
    }

    pub fn read(&mut self, address: u8, reg: u8) -> Result<u8, i2c::Error> {
        self.i2c.write(address, &[reg])?;

        let mut bytes = [0u8; 1];
        self.i2c.read(address, &mut bytes)?;
        Ok(bytes[0])
    }

    pub fn write(&mut self, address: u8, reg: u8, data: u8) -> Result<(), i2c::Error> {
        self.i2c.write(address, &[reg, data])
    }
}

const FIRST_BLOCK: &[(u8, u8)] = &[(0xFF, 0x01), (0x12, 0x80)];

const SECOND_BLOCK: &[(u8, u8)] = &[
    (0xFF, 0x00),
    (0x2C, 0xFF),
    (0x2E, 0xDF),
    (0xFF, 0x01),
    (0x3C, 0x32),
    (0x11, 0x01),
    (0x09, 0x02),
    (0x04, 0x28),
    (0x13, 0xE5),
    (0x14, 0x48),
    (0x2C, 0x0C),
    (0x33, 0x78),
    (0x3A, 0x33),
    (0x3B, 0xFB),
    (0x3E, 0x00),
    (0x43, 0x11),
    (0x16, 0x10),
    (0x39, 0x92),
    (0x35, 0xDA),
    (0x22, 0x1A),
    (0x37, 0xC3),
    (0x23, 0x00),
    (0x34, 0xC0),
    (0x06, 0x88),
    (0x07, 0xC0),
    (0x0D, 0x87),
    (0x0E, 0x41),
    (0x4C, 0x00),
    (0x4A, 0x81),
    (0x21, 0x99),
    (0x24, 0x40),
    (0x25, 0x38),
    (0x26, 0x82),
    (0x5C, 0x00),
    (0x63, 0x00),
    (0x61, 0x70),
    (0x62, 0x80),
    (0x7C, 0x05),
    (0x20, 0x80),
    (0x28, 0x30),
    (0x6C, 0x00),
    (0x6D, 0x80),
    (0x6E, 0x00),
    (0x70, 0x02),
    (0x71, 0x94),
    (0x73, 0xC1),
    (0x3D, 0x34),
    (0x5A, 0x57),
    (0x4F, 0xBB),
    (0x50, 0x9C),
    (0x12, 0x20),
    (0x17, 0x11),
    (0x18, 0x43),
    (0x19, 0x00),
    (0x1A, 0x25),
    (0x32, 0x89),
    (0x37, 0xC0),
    (0x4F, 0xCA),
    (0x50, 0xA8),
    (0x6D, 0x00),
    (0x3D, 0x38),
    (0xFF, 0x00),
    (0xE5, 0x7F),
    (0xF9, 0xC0),
    (0x41, 0x24),
    (0xE0, 0x14),
    (0x76, 0xFF),
    (0x33, 0xA0),
    (0x42, 0x20),
    (0x43, 0x18),
    (0x4C, 0x00),
    (0x87, 0x50),
    (0x88, 0x3F),
    (0xD7, 0x03),
    (0xD9, 0x10),
    (0xD3, 0x82),
    (0xC8, 0x08),
    (0xC9, 0x80),
    (0x7C, 0x00),
    (0x7D, 0x00),
    (0x7C, 0x03),
    (0x7D, 0x48),
    (0x7D, 0x48),
    (0x7C, 0x08),
    (0x7D, 0x20),
    (0x7D, 0x10),
    (0x7D, 0x0E),
    (0x90, 0x00),
    (0x91, 0x0E),
    (0x91, 0x1A),
    (0x91, 0x31),
    (0x91, 0x5A),
    (0x91, 0x69),
    (0x91, 0x75),
    (0x91, 0x7E),
    (0x91, 0x88),
    (0x91, 0x8F),
    (0x91, 0x96),
    (0x91, 0xA3),
    (0x91, 0xAF),
    (0x91, 0xC4),
    (0x91, 0xD7),
    (0x91, 0xE8),
    (0x91, 0x20),
    (0x92, 0x00),
    (0x93, 0x06),
    (0x93, 0xE3),
    (0x93, 0x05),
    (0x93, 0x05),
    (0x93, 0x00),
    (0x93, 0x04),
    (0x93, 0x00),
    (0x93, 0x00),
    (0x93, 0x00),
    (0x93, 0x00),
    (0x93, 0x00),
    (0x93, 0x00),
    (0x93, 0x00),
    (0x96, 0x00),
    (0x97, 0x08),
    (0x97, 0x19),
    (0x97, 0x02),
    (0x97, 0x0C),
    (0x97, 0x24),
    (0x97, 0x30),
    (0x97, 0x28),
    (0x97, 0x26),
    (0x97, 0x02),
    (0x97, 0x98),
    (0x97, 0x80),
    (0x97, 0x00),
    (0x97, 0x00),
    (0xA4, 0x00),
    (0xA8, 0x00),
    (0xC5, 0x11),
    (0xC6, 0x51),
    (0xBF, 0x80),
    (0xC7, 0x10),
    (0xB6, 0x66),
    (0xB8, 0xA5),
    (0xB7, 0x64),
    (0xB9, 0x7C),
    (0xB3, 0xAF),
    (0xB4, 0x97),
    (0xB5, 0xFF),
    (0xB0, 0xC5),
    (0xB1, 0x94),
    (0xB2, 0x0F),
    (0xC4, 0x5C),
    (0xC3, 0xFD),
    (0x7F, 0x00),
    (0xE5, 0x1F),
    (0xE1, 0x67),
    (0xDD, 0x7F),
    (0xDA, 0x00),
    (0xE0, 0x00),
    (0x05, 0x00),
    (0x05, 0x01),
    (0xFF, 0x01),
    (0x12, 0x40),
    (0x03, 0x0A),
    (0x32, 0x09),
    (0x17, 0x11),
    (0x18, 0x43),
    (0x19, 0x00),
    (0x1A, 0x4B),
    (0x37, 0xC0),
    (0x4F, 0xCA),
    (0x50, 0xA8),
    (0x5A, 0x23),
    (0x6D, 0x00),
    (0x3D, 0x38),
    (0x39, 0x92),
    (0x35, 0xDA),
    (0x22, 0x1A),
    (0x37, 0xC3),
    (0x23, 0x00),
    (0x34, 0xC0),
    (0x06, 0x88),
    (0x07, 0xC0),
    (0x0D, 0x87),
    (0x0E, 0x41),
    (0x42, 0x03),
    (0x4C, 0x00),
    (0xFF, 0x00),
    (0xE0, 0x04),
    (0xC0, 0x64),
    (0xC1, 0x4B),
    (0x8C, 0x00),
    (0x51, 0xC8),
    (0x52, 0x96),
    (0x53, 0x00),
    (0x54, 0x00),
    (0x55, 0x00),
    (0x57, 0x00),
    (0x86, 0x3D),
    (0x50, 0x80),
    (0x51, 0xC8),
    (0x52, 0x96),
    (0x53, 0x00),
    (0x54, 0x00),
    (0x55, 0x00),
    (0x57, 0x00),
    (0x5A, 0xA0),
    (0x5B, 0x78),
    (0x5C, 0x00),
    (0xFF, 0x01),
    (0x11, 0x00),
    (0xFF, 0x00),
    (0xD3, 0x10),
    (0x05, 0x00),
    (0xE0, 0x14),
    (0xDA, 0x12),
    (0xD7, 0x03),
    (0xE1, 0x77),
    (0xE5, 0x1F),
    (0xD9, 0x10),
    (0xDF, 0x80),
    (0x33, 0x80),
    (0x3C, 0x10),
    (0xEB, 0x30),
    (0xDD, 0x7F),
    (0xE0, 0x00),
    (0xE0, 0x14),
    (0xDA, 0x12),
    (0xD7, 0x03),
    (0xE1, 0x77),
    (0xE5, 0x1F),
    (0xD9, 0x10),
    (0xDF, 0x80),
    (0x33, 0x80),
    (0x3C, 0x10),
    (0xEB, 0x30),
    (0xDD, 0x7F),
    (0xE0, 0x00),
    (0xFF, 0x01),
    (0x14, 0x08),
    (0xFF, 0x00),
    (0x87, 0x50),
    (0x87, 0x10),
    (0xC3, 0xFD),
    (0x44, 0x0C),
];
