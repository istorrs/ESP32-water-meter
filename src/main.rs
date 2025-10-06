use esp32_water_meter::mtu::{GpioMtuTimer, MtuConfig};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::sys as sys;

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF system services
    sys::link_patches();

    // Initialize logging
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP32 Water Meter MTU Interface (Timer-based)");
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

    log::info!("Creating hardware timer-based MTU instance:");
    log::info!("  Baud rate: {} baud", config.baud_rate);
    log::info!("  Bit duration: {} μs", config.bit_duration_micros());
    log::info!("  Timer frequency: {} Hz (2x baud for HIGH/LOW)", config.baud_rate * 2);

    let mtu = GpioMtuTimer::new(config);

    log::info!("✅ MTU initialized successfully");
    log::info!("Starting hardware timer-based MTU operation for 10 seconds...");

    // Run a test MTU operation with hardware timer
    match mtu.run_mtu_operation_with_timer(
        &mut clock_pin,
        &mut data_pin,
        peripherals.timer00,
        10,
    ) {
        Ok(_) => {
            let (success, corrupt, cycles) = mtu.get_stats();
            log::info!("✅ MTU timer operation completed successfully");
            log::info!("   Total clock cycles: {}", cycles);
            log::info!("   Expected: ~{} cycles at {} baud", 1200 * 10, 1200);
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
