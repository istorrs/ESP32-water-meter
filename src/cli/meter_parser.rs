use crate::meter::MeterType;

#[derive(Debug, Clone)]
pub enum MeterCommand {
    Help,
    Clear,
    Version,
    Status,
    Uptime,
    Reset,
    SetType(MeterType),
    SetMessage(String),
    Enable,
    Disable,
    Empty,
    Unknown(String),
}

pub struct MeterCommandParser;

impl MeterCommandParser {
    pub fn parse_command(input: &str) -> MeterCommand {
        let input = input.trim();

        if input.is_empty() {
            return MeterCommand::Empty;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return MeterCommand::Empty;
        }

        match parts[0] {
            "help" | "h" => MeterCommand::Help,
            "clear" | "cls" => MeterCommand::Clear,
            "version" | "ver" => MeterCommand::Version,
            "status" | "stat" => MeterCommand::Status,
            "uptime" => MeterCommand::Uptime,
            "reset" => MeterCommand::Reset,
            "enable" => MeterCommand::Enable,
            "disable" => MeterCommand::Disable,
            "type" => {
                if parts.len() >= 2 {
                    match parts[1] {
                        "sensus" | "s" => MeterCommand::SetType(MeterType::Sensus),
                        "neptune" | "n" => MeterCommand::SetType(MeterType::Neptune),
                        _ => MeterCommand::Unknown(format!(
                            "Invalid meter type: '{}'. Use 'sensus' or 'neptune'",
                            parts[1]
                        )),
                    }
                } else {
                    MeterCommand::Unknown(
                        "Usage: type <sensus|neptune>. Type 'help' for more info.".to_string(),
                    )
                }
            }
            "message" | "msg" => {
                if parts.len() >= 2 {
                    // Join all parts after "message" as the message content
                    let message_text = parts[1..].join(" ");

                    // Add carriage return if not present
                    let mut message = message_text;
                    if !message.ends_with('\r') {
                        message.push('\r');
                    }

                    MeterCommand::SetMessage(message)
                } else {
                    MeterCommand::Unknown(
                        "Usage: message <text>. Carriage return (\\r) will be added automatically."
                            .to_string(),
                    )
                }
            }
            _ => MeterCommand::Unknown(format!(
                "Unknown command: '{}'. Type 'help' for available commands.",
                parts[0]
            )),
        }
    }

    pub fn available_commands() -> &'static [&'static str] {
        &[
            "help", "clear", "version", "status", "uptime", "reset", "type", "message", "enable",
            "disable",
        ]
    }
}
