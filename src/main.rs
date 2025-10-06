use esp32_water_meter::mtu::{GpioMtu, MtuConfig};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{PinDriver, Input, Output};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::sys as sys;

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF system services
    sys::link_patches();

    // Initialize logging
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP32 Water Meter MTU Interface");
    log::info!("Initializing...");

    let peripherals = Peripherals::take()?;

    log::info!("✅ ESP32 initialized with ESP-IDF");

    // Initialize GPIO pins for MTU
    // Using GPIO4 for clock output and GPIO5 for data input
    log::info!("Initializing MTU GPIO pins...");
    log::info!("  Clock pin: GPIO4 (output)");
    log::info!("  Data pin:  GPIO5 (input)");

    let mut clock_pin = PinDriver::output(peripherals.pins.gpio4)?;
    let mut data_pin = PinDriver::input(peripherals.pins.gpio5)?;

    // Create MTU instance with default config
    let mut config = MtuConfig::default();
    config.baud_rate = 1200; // 1200 baud for water meter

    log::info!("Creating MTU instance with config:");
    log::info!("  Baud rate: {} baud", config.baud_rate);
    log::info!("  Bit duration: {} μs", config.bit_duration_micros());
    log::info!("  Power-up delay: {} ms", config.power_up_delay_ms);

    let mtu = GpioMtu::new(config);

    log::info!("✅ MTU initialized successfully");
    log::info!("Starting MTU operation for 10 seconds...");

    // Run a test MTU operation
    match mtu.run_mtu_operation(&mut clock_pin, &mut data_pin, 10) {
        Ok(_) => {
            log::info!("✅ MTU operation completed successfully");
        }
        Err(e) => {
            log::error!("❌ MTU operation failed: {:?}", e);
        }
    }

    log::info!("MTU test complete - entering idle loop");

    // Main loop
    loop {
        FreeRtos::delay_ms(1000);
        log::info!("System running...");
    }
}
