use super::{CliCommand, CliError};
use crate::mtu::{GpioMtuTimerV2, MtuCommand};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Instant;

pub struct CommandHandler {
    start_time: Instant,
    mtu: Option<Arc<GpioMtuTimerV2>>,
    mtu_cmd_sender: Option<Sender<MtuCommand>>,
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHandler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            mtu: None,
            mtu_cmd_sender: None,
        }
    }

    pub fn with_mtu(mut self, mtu: Arc<GpioMtuTimerV2>, cmd_sender: Sender<MtuCommand>) -> Self {
        self.mtu = Some(mtu);
        self.mtu_cmd_sender = Some(cmd_sender);
        self
    }

    pub fn execute_command(&mut self, command: CliCommand) -> Result<String, CliError> {
        let mut response = String::new();

        match command {
            CliCommand::Empty => {
                // Empty command - just return empty response (no error)
            }
            CliCommand::Help => {
                // Help is handled in terminal.rs
                response.push_str("Help displayed");
            }
            CliCommand::Version => {
                log::info!("CLI: Version requested");
                response.push_str("ESP32 Water Meter MTU Interface v1.0.0\r\n");
                response.push_str("Built with ESP-IDF");
            }
            CliCommand::Status => {
                log::info!("CLI: Status requested");
                response.push_str("System Status:\r\n");
                response.push_str("  Firmware: ESP32 Water Meter MTU v1.0.0\r\n");
                response.push_str("  Platform: ESP32 with ESP-IDF\r\n");
                response.push_str("  MTU: GPIO4 (clock), GPIO5 (data)\r\n");
                response.push_str("  UART: USB-C (UART0)");
            }
            CliCommand::Uptime => {
                log::info!("CLI: Uptime requested");
                let uptime = self.start_time.elapsed();
                let uptime_secs = uptime.as_secs();
                let hours = uptime_secs / 3600;
                let minutes = (uptime_secs % 3600) / 60;
                let seconds = uptime_secs % 60;

                response.push_str("Uptime: ");
                if hours > 0 {
                    response.push_str(&format!("{}h ", hours));
                }
                if minutes > 0 || hours > 0 {
                    response.push_str(&format!("{}m ", minutes));
                }
                response.push_str(&format!("{}s", seconds));
            }
            CliCommand::Clear => {
                // Clear is handled in terminal.rs
                response.push_str("Screen cleared");
            }
            CliCommand::Reset => {
                log::info!("CLI: Reset requested");
                response.push_str("Resetting system...");
                // Perform system reset using ESP-IDF
                unsafe {
                    esp_idf_svc::sys::esp_restart();
                }
            }
            CliCommand::Echo(text) => {
                log::info!("CLI: Echo requested: {}", text);
                response.push_str(&text);
            }
            CliCommand::MtuStart(duration) => {
                log::info!("CLI: MTU start requested");
                if let Some(ref sender) = self.mtu_cmd_sender {
                    if let Some(ref mtu) = self.mtu {
                        let duration_secs = duration.unwrap_or(30);

                        if mtu.is_running() {
                            response.push_str("MTU is already running. Use 'mtu_stop' first.");
                        } else {
                            // Send start command to MTU thread
                            match sender.send(MtuCommand::Start {
                                duration_secs: duration_secs.into(),
                            }) {
                                Ok(_) => {
                                    response.push_str(&format!(
                                        "MTU operation started for {} seconds",
                                        duration_secs
                                    ));
                                }
                                Err(_) => {
                                    response
                                        .push_str("Error: Failed to send command to MTU thread");
                                }
                            }
                        }
                    } else {
                        response.push_str("MTU not configured");
                    }
                } else {
                    response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuStop => {
                log::info!("CLI: MTU stop requested");
                if let Some(ref sender) = self.mtu_cmd_sender {
                    if let Some(ref mtu) = self.mtu {
                        if mtu.is_running() {
                            // Send stop command to MTU thread
                            match sender.send(MtuCommand::Stop) {
                                Ok(_) => {
                                    response.push_str("MTU stop signal sent");
                                }
                                Err(_) => {
                                    response
                                        .push_str("Error: Failed to send command to MTU thread");
                                }
                            }
                        } else {
                            response.push_str("MTU is not running");
                        }
                    } else {
                        response.push_str("MTU not configured");
                    }
                } else {
                    response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuStatus => {
                log::info!("CLI: MTU status requested");
                if let Some(ref mtu) = self.mtu {
                    let baud_rate = mtu.get_baud_rate();
                    let (successful, corrupted, cycles) = mtu.get_stats();
                    let total_reads = successful + corrupted;

                    response.push_str("MTU Status:\r\n");
                    response.push_str(&format!(
                        "  State: {}\r\n",
                        if mtu.is_running() {
                            "Running"
                        } else {
                            "Stopped"
                        }
                    ));
                    response.push_str(&format!("  Baud rate: {} bps\r\n", baud_rate));
                    response.push_str("  Pins: GPIO4 (clock), GPIO5 (data)\r\n");
                    response.push_str(&format!("  Total cycles: {}\r\n", cycles));
                    response.push_str("  Statistics:\r\n");
                    response.push_str(&format!("    Successful reads: {}\r\n", successful));
                    response.push_str(&format!("    Corrupted reads: {}\r\n", corrupted));

                    if total_reads > 0 {
                        let success_rate = (successful as f32 / total_reads as f32) * 100.0;
                        response.push_str(&format!("    Success rate: {:.1}%\r\n", success_rate));
                    }

                    if let Some(last_msg) = mtu.get_last_message() {
                        response.push_str(&format!("  Last message: {}", last_msg.as_str()));
                    } else {
                        response.push_str("  Last message: None");
                    }
                } else {
                    response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuBaud(baud_rate) => {
                log::info!("CLI: MTU baud rate set to {}", baud_rate);
                if let Some(ref mtu) = self.mtu {
                    if mtu.is_running() {
                        response.push_str("Cannot change baud rate while MTU is running.\r\n");
                        response.push_str("Use 'mtu_stop' first.");
                    } else {
                        mtu.set_baud_rate(baud_rate);
                        response.push_str(&format!("MTU baud rate set to {} bps", baud_rate));
                    }
                } else {
                    response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuReset => {
                log::info!("CLI: MTU statistics reset requested");
                if let Some(ref mtu) = self.mtu {
                    mtu.reset_stats();
                    response.push_str("MTU statistics reset");
                } else {
                    response.push_str("MTU not configured");
                }
            }
            CliCommand::Unknown(cmd) => {
                log::info!("CLI: Unknown command: {}", cmd);
                response.push_str("Unknown command: ");
                response.push_str(&cmd);
                response.push_str(". Type 'help' for available commands.");
            }
        }

        Ok(response)
    }
}
