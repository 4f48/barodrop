#![no_std]
#![no_main]

use bmp390_rs::{Bmp390, Interrupts, config::Configuration};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts, dma,
    gpio::{Input, Output},
    i2c,
    peripherals::{DMA_CH0, DMA_CH1, I2C1, USB},
    pwm, spi, usb,
};
use embassy_time::Delay;
use embedded_io_async::Read;
use libm::powf;
use log::info;
use panic_halt as _;
use rp_servo::Servo;
use sd_fat::{
    block_device::sdcard::SdCard,
    fs::{Dir, DirEntry, FileSystem, fat32::Fat32},
};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

#[embassy_executor::task]
async fn logger_task(driver: usb::Driver<'static, USB>) {
    embassy_usb_logger::run!(512, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let usb_driver = usb::Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(usb_driver).unwrap());

    let spi = spi::Spi::new(
        p.SPI0,
        p.PIN_18,
        p.PIN_19,
        p.PIN_20,
        p.DMA_CH0,
        p.DMA_CH1,
        Irqs,
        Default::default(),
    );
    let cs = Output::new(p.PIN_23, embassy_rp::gpio::Level::High);
    let sd = SdCard::new(spi, cs).await.unwrap();
    let mut fs = Fat32::mount(sd).await.unwrap();
    let mut root = fs.open_dir("/").await.unwrap();
    let config_entry = root.find("CONFIG~1.TOM").await.unwrap();
    let mut config_file = fs.open_file_at(config_entry.cluster(), config_entry.size());

    let mut buf = [0u8; 128];
    let n = config_file.read(&mut buf).await.unwrap();

    let content = core::str::from_utf8(&buf[..n]).unwrap().trim();
    let (_, value) = content.split_once("=").unwrap();
    let target: f32 = value.parse().unwrap();

    let pwm = pwm::Pwm::new_output_a(p.PWM_SLICE6, p.PIN_12, Default::default());
    let mut servo = Servo::new(pwm, 500, 2550);
    servo.set(0).unwrap();

    let i2c = i2c::I2c::new_async(p.I2C1, p.PIN_27, p.PIN_26, Irqs, Default::default());
    let mut int = Input::new(p.PIN_24, embassy_rp::gpio::Pull::None);

    let bmp390_conf = Configuration::default()
        .enable_pressure_measurement(true)
        .enable_temperature_measurement(true)
        .iir_filter_coefficient(bmp390_rs::register::config::IIRFilterCoefficient::Coef3)
        .output_data_rate(bmp390_rs::register::odr::OutputDataRate::R0p78Hz)
        .power_mode(bmp390_rs::register::pwr_ctrl::PowerMode::Normal)
        .pressure_oversampling(bmp390_rs::register::osr::Oversampling::X8)
        .temperature_oversampling(bmp390_rs::register::osr::Oversampling::X1);
    let mut bmp390 = Bmp390::new_i2c(
        i2c,
        bmp390_rs::SdoPinState::High,
        bmp390_conf,
        bmp390_rs::ResetPolicy::Soft,
        &mut Delay,
    )
    .await
    .unwrap();
    bmp390
        .mask_interrupts(Interrupts::new().fifo_full().fifo_watermark())
        .await
        .unwrap();

    for _ in 0..3 {
        int.wait_for_rising_edge().await;
        bmp390.read_sensor_data().await.unwrap().pressure();
    }
    int.wait_for_rising_edge().await;
    let base = bmp390.read_sensor_data().await.unwrap().pressure();
    servo.set(90).unwrap();

    loop {
        int.wait_for_rising_edge().await;
        let pressure = bmp390.read_sensor_data().await.unwrap().pressure();
        let altitude = altitude(base, pressure);

        if altitude > target {
            servo.set(0).unwrap();
        }

        info!("{} / {}", altitude, target);
    }
}

fn altitude(base: f32, pressure: f32) -> f32 {
    44330.0 * (1.0 - powf(pressure / base, 0.1903))
}
