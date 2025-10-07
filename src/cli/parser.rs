use super::CliCommand;

pub struct CommandParser;

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandParser {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_commands() -> &'static [&'static str] {
        &[
            "help",
            "version",
            "status",
            "uptime",
            "clear",
            "reset",
            "echo",
            "mtu_start",
            "mtu_stop",
            "mtu_status",
            "mtu_baud",
            "mtu_reset",
        ]
    }

    pub fn autocomplete(partial: &str) -> Vec<&'static str> {
        let commands = Self::get_available_commands();
        commands
            .iter()
            .filter(|&&cmd| cmd.starts_with(partial))
            .copied()
            .collect()
    }

    pub fn parse_command(input: &str) -> CliCommand {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return CliCommand::Empty;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "help" => CliCommand::Help,
            "version" => CliCommand::Version,
            "status" => CliCommand::Status,
            "uptime" => CliCommand::Uptime,
            "clear" => CliCommand::Clear,
            "reset" => CliCommand::Reset,
            "mtu_start" => {
                if let Some(arg) = parts.next() {
                    if let Ok(duration) = arg.parse::<u16>() {
                        if duration > 0 && duration <= 300 {
                            CliCommand::MtuStart(Some(duration))
                        } else {
                            CliCommand::Unknown(
                                "mtu_start: duration must be 1-300 seconds".to_string(),
                            )
                        }
                    } else {
                        CliCommand::Unknown("mtu_start: invalid duration".to_string())
                    }
                } else {
                    CliCommand::MtuStart(None) // Default duration
                }
            }
            "mtu_stop" => CliCommand::MtuStop,
            "mtu_status" => CliCommand::MtuStatus,
            "mtu_baud" => {
                if let Some(baud_str) = parts.next() {
                    if let Ok(baud_rate) = baud_str.parse::<u32>() {
                        if (1..=115200).contains(&baud_rate) {
                            CliCommand::MtuBaud(baud_rate)
                        } else {
                            CliCommand::Unknown("mtu_baud: rate must be 1-115200".to_string())
                        }
                    } else {
                        CliCommand::Unknown("mtu_baud: invalid baud rate".to_string())
                    }
                } else {
                    CliCommand::Unknown("mtu_baud: baud rate required".to_string())
                }
            }
            "echo" => {
                let args: Vec<&str> = parts.collect();
                let echo_string = args.join(" ");
                CliCommand::Echo(echo_string)
            }
            "mtu_reset" => CliCommand::MtuReset,
            _ => CliCommand::Unknown(cmd.to_string()),
        }
    }
}
