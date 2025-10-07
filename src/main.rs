use esp32_water_meter::cli::{CommandHandler, CommandParser, Terminal};
use esp32_water_meter::mtu::{GpioMtuTimerV2, MtuConfig};
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

    log::info!("ESP32 Water Meter MTU Interface with CLI");
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

    // Initialize GPIO pins for MTU
    // Using GPIO4 for clock output and GPIO5 for data input
    log::info!("Initializing MTU GPIO pins...");
    log::info!("  Clock pin: GPIO4 (output, starting LOW - no power to meter)");
    log::info!("  Data pin:  GPIO5 (input)");

    // Initialize clock pin LOW to simulate no power to meter at startup
    let mut clock_pin = PinDriver::output(peripherals.pins.gpio4)?;
    clock_pin.set_low()?;
    log::info!("✅ Clock pin initialized LOW");

    let data_pin = PinDriver::input(peripherals.pins.gpio5)?;

    // SAFETY: We need 'static lifetime for pins to move into background thread
    // The pins will be owned by the MTU thread for the entire program lifetime
    let clock_pin_static: PinDriver<'static, esp_idf_hal::gpio::Gpio4, Output> =
        unsafe { core::mem::transmute(clock_pin) };
    let data_pin_static: PinDriver<'static, esp_idf_hal::gpio::Gpio5, Input> =
        unsafe { core::mem::transmute(data_pin) };

    // Get timer peripheral for MTU
    let timer = peripherals.timer00;

    // Create MTU instance with default config
    let config = MtuConfig::default();
    let mtu = Arc::new(GpioMtuTimerV2::new(config));

    log::info!("✅ MTU GPIO pins configured");
    log::info!("✅ MTU instance created with {} baud", mtu.get_baud_rate());

    // Spawn MTU background thread and get command sender
    let mtu_cmd_sender = GpioMtuTimerV2::spawn_mtu_thread(
        Arc::clone(&mtu),
        clock_pin_static,
        data_pin_static,
        timer,
    );

    log::info!("✅ MTU background thread spawned");

    // Initialize CLI components
    let mut terminal = Terminal::new(uart_tx, uart_rx);
    let mut command_handler = CommandHandler::new().with_mtu(Arc::clone(&mtu), mtu_cmd_sender);

    log::info!("✅ CLI initialized");

    // Send welcome message
    terminal.write_line("")?;
    terminal.write_line("ESP32 Water Meter MTU Interface")?;
    terminal.write_line("Type 'help' for available commands")?;
    terminal.write_line("Use TAB for command autocompletion")?;
    terminal.write_line("MTU Clock: GPIO4 | Data: GPIO5")?;
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
                        let command = CommandParser::parse_command(&command_line);

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
                            esp32_water_meter::cli::CliCommand::Help => {
                                let _ = terminal.show_help();
                            }
                            esp32_water_meter::cli::CliCommand::Clear => {
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
