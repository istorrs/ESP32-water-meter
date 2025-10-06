use esp_idf_hal::delay::FreeRtos;
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
    log::info!("Peripherals available for GPIO configuration");

    loop {
        log::info!("Hello from ESP32!");
        FreeRtos::delay_ms(1000);
    }
}
