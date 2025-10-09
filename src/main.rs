use esp32_water_meter::cli::{CommandHandler, CommandParser, Terminal};
use esp32_water_meter::mqtt::MqttClient;
use esp32_water_meter::mtu::{GpioMtuTimerV2, MtuCommand, MtuConfig};
use esp32_water_meter::wifi::WifiManager;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Input, Output, PinDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::uart::{config::Config as UartConfig, UartDriver};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::mqtt::client::QoS;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys;
use std::sync::{Arc, Mutex};

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF system services
    sys::link_patches();

    // Initialize logging
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP32 Water Meter MTU Interface with CLI");
    log::info!("Initializing...");

    let peripherals = Peripherals::take()?;

    log::info!("✅ ESP32 initialized with ESP-IDF");

    // Initialize system event loop and NVS for WiFi
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // WiFi Configuration
    const WIFI_SSID: &str = "Ian Storrs 1";
    const WIFI_PASSWORD: &str = "abbaabba";

    // MQTT Configuration - Mosquitto public test broker
    const MQTT_BROKER: &str = "mqtt://test.mosquitto.org:1883";
    const MQTT_CLIENT_ID: &str = "esp32-mtu-istorrs";
    const MQTT_PUBLISH_TOPIC: &str = "istorrs/mtu/data";
    const MQTT_CONTROL_TOPIC: &str = "istorrs/mtu/control";

    // Initialize WiFi (optional - comment out if WiFi not needed)
    let wifi = if WIFI_SSID != "YOUR_SSID" {
        log::info!("🌐 Initializing WiFi...");
        log::info!("  SSID: {}", WIFI_SSID);
        log::info!("  Password length: {} chars", WIFI_PASSWORD.len());

        match WifiManager::new(
            peripherals.modem,
            sysloop.clone(),
            nvs.clone(),
            WIFI_SSID,
            WIFI_PASSWORD,
        ) {
            Ok(wifi) => {
                log::info!("✅ WiFi initialization successful");
                Some(Arc::new(Mutex::new(wifi)))
            }
            Err(e) => {
                log::error!("❌ WiFi initialization failed: {:?}", e);
                log::warn!("⚠️  Continuing without WiFi - use 'wifi_connect' command to retry");
                log::warn!("⚠️  Note: WiFi requires modem peripheral which is consumed on first init");
                log::warn!("⚠️  Recommendation: Fix WiFi credentials and reboot");
                None
            }
        }
    } else {
        log::info!("WiFi disabled (update WIFI_SSID/WIFI_PASSWORD to enable)");
        None
    };

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

    // Initialize MQTT (optional - requires WiFi)
    // Note: MQTT is initialized after MTU so the callback can access MTU command sender
    let mqtt = if wifi.is_some() {
        log::info!("Initializing MQTT client...");

        // Clone the MTU command sender for MQTT callback
        let mqtt_mtu_sender = mtu_cmd_sender.clone();

        match MqttClient::new(
            MQTT_BROKER,
            MQTT_CLIENT_ID,
            Arc::new(move |topic, data| {
                if let Ok(message) = std::str::from_utf8(data) {
                    log::info!("MQTT message on {}: {}", topic, message);

                    // Handle control messages
                    if topic == MQTT_CONTROL_TOPIC {
                        let cmd = message.trim().to_lowercase();
                        match cmd.as_str() {
                            "start" => {
                                log::info!("MQTT: Starting MTU (30s default)");
                                let _ =
                                    mqtt_mtu_sender.send(MtuCommand::Start { duration_secs: 30 });
                            }
                            msg if msg.starts_with("start ") => {
                                if let Some(duration_str) = msg.strip_prefix("start ") {
                                    if let Ok(duration) = duration_str.parse::<u64>() {
                                        log::info!("MQTT: Starting MTU for {}s", duration);
                                        let _ = mqtt_mtu_sender.send(MtuCommand::Start {
                                            duration_secs: duration,
                                        });
                                    }
                                }
                            }
                            "stop" => {
                                log::info!("MQTT: Stopping MTU");
                                let _ = mqtt_mtu_sender.send(MtuCommand::Stop);
                            }
                            _ => {
                                log::warn!("MQTT: Unknown control command: {}", cmd);
                            }
                        }
                    }
                } else {
                    log::warn!("MQTT: Received non-UTF8 data on {}", topic);
                }
            }),
        ) {
            Ok(client) => {
                log::info!("MQTT client created, waiting for connection...");

                // Spawn a thread to wait for connection then subscribe
                let client_clone = Arc::new(client);
                let subscribe_client = Arc::clone(&client_clone);
                std::thread::spawn(move || {
                    log::info!("MQTT subscription thread started, waiting for connection...");
                    // Wait up to 10 seconds for connection
                    for i in 0..20 {
                        if subscribe_client.is_connected() {
                            log::info!("MQTT connected! Subscribing to control topic...");
                            match subscribe_client.subscribe(MQTT_CONTROL_TOPIC, QoS::AtLeastOnce) {
                                Ok(_) => log::info!(
                                    "✅ Subscribed to control topic: {}",
                                    MQTT_CONTROL_TOPIC
                                ),
                                Err(e) => {
                                    log::warn!("❌ Failed to subscribe to control topic: {:?}", e)
                                }
                            }
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if i % 4 == 0 {
                            log::info!("Waiting for MQTT connection... ({}s)", i / 2);
                        }
                    }
                    if !subscribe_client.is_connected() {
                        log::warn!("❌ MQTT connection timeout - subscription failed");
                    }
                });

                Some(client_clone)
            }
            Err(e) => {
                log::warn!("❌ MQTT initialization failed: {:?}", e);
                log::warn!("Continuing without MQTT...");
                None
            }
        }
    } else {
        log::info!("MQTT disabled (requires WiFi)");
        None
    };

    // Initialize CLI components
    let mut terminal = Terminal::new(uart_tx, uart_rx);
    let mut command_handler = CommandHandler::new().with_mtu(Arc::clone(&mtu), mtu_cmd_sender);

    // Add WiFi and MQTT to command handler if available
    if let Some(ref wifi_manager) = wifi {
        command_handler = command_handler.with_wifi(Arc::clone(wifi_manager));
    }
    if let Some(ref mqtt_client) = mqtt {
        command_handler = command_handler.with_mqtt(Arc::clone(mqtt_client));
    }

    log::info!("✅ CLI initialized");

    // Send welcome message
    terminal.write_line("")?;
    terminal.write_line("ESP32 Water Meter MTU Interface")?;
    terminal.write_line("Type 'help' for available commands")?;
    terminal.write_line("Use TAB for command autocompletion")?;
    terminal.write_line("MTU Clock: GPIO4 | Data: GPIO5")?;

    // Show WiFi/MQTT status in welcome message
    if wifi.is_some() {
        terminal.write_line("WiFi: Enabled")?;
    }
    if mqtt.is_some() {
        terminal.write_line("MQTT: Enabled")?;
    }
    terminal.print_prompt()?;

    log::info!("Entering CLI loop...");

    // Track last published cycle count for MQTT auto-publishing
    // Publish based on MTU read cycles, not message content (allows duplicate messages)
    let mut last_published_cycles = 0u64;
    let mut publish_counter = 0u32;

    // Main CLI loop
    loop {
        // Auto-publish MTU data to MQTT if available and new data exists
        if let Some(ref mqtt_client) = mqtt {
            if let Some(current_message) = mtu.get_last_message() {
                // Get statistics for the JSON payload
                let (successful, corrupted, cycles) = mtu.get_stats();

                // Publish if we have a new MTU read cycle (successful or corrupted count increased)
                let total_reads = successful + corrupted;
                let should_publish = u64::from(total_reads) > last_published_cycles;

                if should_publish {
                    let baud_rate = mtu.get_baud_rate();

                    // Build JSON payload
                    let payload = serde_json::json!({
                        "message": current_message.as_str(),
                        "baud_rate": baud_rate,
                        "cycles": cycles,
                        "successful": successful,
                        "corrupted": corrupted,
                        "count": publish_counter,
                    });

                    if let Ok(json_str) = serde_json::to_string(&payload) {
                        // Check WiFi status before publishing
                    let wifi_ok = if let Some(ref wifi_manager) = wifi {
                        wifi_manager
                            .lock()
                            .ok()
                            .and_then(|w| w.is_connected().ok())
                            .unwrap_or(false)
                    } else {
                        true // No WiFi manager means no WiFi check needed
                    };

                    if !wifi_ok {
                        log::warn!("❌ WiFi disconnected, skipping MQTT publish");
                    } else {
                        match mqtt_client.publish(
                            MQTT_PUBLISH_TOPIC,
                            json_str.as_bytes(),
                            QoS::AtLeastOnce,
                            false,
                        ) {
                            Ok(_) => {
                                // Only log every 10th successful publish to reduce spam
                                if publish_counter % 10 == 0 {
                                    log::info!(
                                        "📤 Published #{} to {}: {}",
                                        publish_counter,
                                        MQTT_PUBLISH_TOPIC,
                                        current_message.as_str()
                                    );
                                }
                                publish_counter += 1;
                                last_published_cycles = u64::from(total_reads);
                            }
                            Err(e) => {
                                log::warn!("❌ Failed to publish to MQTT: {:?}", e);

                                // Check if WiFi is still connected
                                if let Some(ref wifi_manager) = wifi {
                                    if let Ok(wifi_guard) = wifi_manager.lock() {
                                        let wifi_connected = wifi_guard.is_connected().unwrap_or(false);
                                        drop(wifi_guard); // Release lock

                                        if !wifi_connected {
                                            log::warn!("⚠️  WiFi disconnected - attempting reconnect...");

                                            if let Ok(mut wifi_guard) = wifi_manager.lock() {
                                                match wifi_guard.reconnect(None, None) {
                                                    Ok(_) => {
                                                        log::info!("✅ WiFi reconnected successfully");
                                                        // Give MQTT client time to reconnect
                                                        std::thread::sleep(std::time::Duration::from_millis(2000));
                                                    }
                                                    Err(e) => {
                                                        log::error!("❌ WiFi reconnect failed: {:?}", e);
                                                        log::warn!("⚠️  Use 'wifi_connect' command to manually retry");
                                                    }
                                                }
                                            }
                                        } else {
                                            log::warn!("⚠️  WiFi connected but MQTT publish failed - MQTT will retry");
                                        }
                                    }
                                }
                                // Don't update last_published_cycles on failure so we retry
                            }
                        }
                    }
                    }
                }
            }
        }

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
