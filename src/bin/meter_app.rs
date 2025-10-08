use esp32_water_meter::cli::{MeterCommand, MeterCommandHandler, MeterCommandParser, Terminal};
use esp32_water_meter::meter::{MeterConfig, MeterHandler};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Input, Output, PinDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::uart::{config::Config as UartConfig, UartDriver};
use esp_idf_svc::sys;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF system services
    sys::link_patches();

    // Initialize logging
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP32 Water Meter Simulator with CLI");
    log::info!("Initializing...");

    let peripherals = Peripherals::take()?;

    log::info!("✅ ESP32 initialized with ESP-IDF");

    // Initialize UART0 for CLI (USB-C connection)
    log::info!("Initializing UART0 for CLI (USB-C)...");
    let uart_config = UartConfig::new().baudrate(115200.into());
    let mut uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio1, // TX (U0TXD)
        peripherals.pins.gpio3, // RX (U0RXD)
        Option::<esp_idf_hal::gpio::Gpio0>::None,
        Option::<esp_idf_hal::gpio::Gpio0>::None,
        &uart_config,
    )?;

    // Split UART into tx and rx drivers
    let (uart_tx, uart_rx) = uart.split();

    log::info!("✅ UART0 initialized (115200 baud)");

    // Initialize GPIO pins for Meter
    // Using GPIO4 for clock input and GPIO5 for data output
    log::info!("Initializing Meter GPIO pins...");
    log::info!("  Clock pin: GPIO4 (input with interrupt)");
    log::info!("  Data pin:  GPIO5 (output, starting HIGH - idle state)");

    let clock_pin = PinDriver::input(peripherals.pins.gpio4)?;

    // Initialize data pin HIGH for idle state
    let mut data_pin = PinDriver::output(peripherals.pins.gpio5)?;
    data_pin.set_high()?;
    log::info!("✅ Data pin initialized HIGH (idle)");

    // SAFETY: We need 'static lifetime for pins to move into background thread
    // The pins will be owned by the Meter thread for the entire program lifetime
    let clock_pin_static: PinDriver<'static, esp_idf_hal::gpio::Gpio4, Input> =
        unsafe { core::mem::transmute(clock_pin) };
    let data_pin_static: PinDriver<'static, esp_idf_hal::gpio::Gpio5, Output> =
        unsafe { core::mem::transmute(data_pin) };

    // Create Meter instance with default config
    let config = MeterConfig::default();
    let meter = Arc::new(MeterHandler::new(config));

    log::info!("✅ Meter GPIO pins configured");
    log::info!(
        "✅ Meter instance created - message: '{}'",
        meter.get_config().response_message.as_str()
    );

    // Spawn Meter background thread
    MeterHandler::spawn_meter_thread(Arc::clone(&meter), clock_pin_static, data_pin_static);

    log::info!("✅ Meter background thread spawned");

    // Initialize CLI components
    let mut terminal = Terminal::new(uart_tx, uart_rx);
    let mut command_handler = MeterCommandHandler::new().with_meter(Arc::clone(&meter));

    log::info!("✅ CLI initialized");

    // Send welcome message
    terminal.write_line("")?;
    terminal.write_line("ESP32 Water Meter Simulator")?;
    terminal.write_line("Type 'help' for available commands")?;
    terminal.write_line("Use TAB for command autocompletion")?;
    terminal.write_line("Meter Clock: GPIO4 | Data: GPIO5")?;
    terminal.print_prompt()?;

    log::info!("Entering CLI loop...");

    // Main CLI loop
    loop {
        // Read character with non-blocking timeout
        match terminal.read_char() {
            Ok(Some(ch)) => {
                // Handle character and check if we got a complete command
                match terminal.handle_char(ch) {
                    Ok(Some(command_line)) => {
                        // Parse and execute the command
                        let command = MeterCommandParser::parse_command(&command_line);

                        // Clone command for later pattern matching
                        let command_clone = command.clone();

                        match command_handler.execute_command(command) {
                            Ok(response) => {
                                if !response.is_empty() {
                                    let _ = terminal.write_line(&response);
                                }
                            }
                            Err(_) => {
                                log::warn!("CLI command execution error");
                                let _ = terminal.write_line("Command execution error.");
                            }
                        }

                        // Handle special commands that need terminal interaction
                        match command_clone {
                            MeterCommand::Help => {
                                let _ = terminal.show_meter_help();
                            }
                            MeterCommand::Clear => {
                                let _ = terminal.clear_screen();
                            }
                            _ => {}
                        }

                        let _ = terminal.print_prompt();
                    }
                    Ok(None) => {
                        // Character processed but no complete command yet
                    }
                    Err(_) => {
                        log::warn!("Terminal input error");
                        let _ = terminal.write_line("Input error");
                        let _ = terminal.print_prompt();
                    }
                }
            }
            Ok(None) => {
                // No data available, small delay to avoid busy loop
                FreeRtos::delay_ms(10);
            }
            Err(_) => {
                // UART error, small delay
                FreeRtos::delay_ms(10);
            }
        }
    }
}
