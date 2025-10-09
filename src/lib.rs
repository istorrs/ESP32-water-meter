//! ESP32 Water Meter MTU Interface Library
//!
//! This library provides modules for ESP32-based water meter MTU communication.

pub mod cli;
pub mod meter;
pub mod mqtt;
pub mod mtu;
pub mod network_config;
pub mod wifi;

pub use cli::{
    CliCommand, CliError, CommandHandler, CommandParser, MeterCommand, MeterCommandHandler,
    MeterCommandParser, Terminal,
};
pub use meter::{MeterConfig, MeterHandler, MeterType};
pub use mqtt::{MqttClient, MqttStatus};
pub use mtu::{
    GpioMtu, GpioMtuTimer, GpioMtuTimerV2, MtuCommand, MtuConfig, MtuError, MtuResult, UartFraming,
};
pub use network_config::{MqttConfig, MtuMqttTopics, WifiConfig};
pub use wifi::WifiManager;
