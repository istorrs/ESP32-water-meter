//! ESP32 Water Meter MTU Interface Library
//!
//! This library provides modules for ESP32-based water meter MTU communication.

pub mod mtu;
pub mod meter;
pub mod cli;

pub use mtu::{MtuConfig, MtuError, MtuResult, GpioMtu, GpioMtuTimer, GpioMtuTimerV2, MtuCommand, UartFraming};
pub use meter::{MeterConfig, MeterType, MeterHandler};
pub use cli::{CliCommand, CliError, CommandHandler, CommandParser, Terminal};
