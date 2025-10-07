//! ESP32 Water Meter MTU Interface Library
//!
//! This library provides modules for ESP32-based water meter MTU communication.

pub mod cli;
pub mod meter;
pub mod mtu;

pub use cli::{CliCommand, CliError, CommandHandler, CommandParser, Terminal};
pub use meter::{MeterConfig, MeterHandler, MeterType};
pub use mtu::{
    GpioMtu, GpioMtuTimer, GpioMtuTimerV2, MtuCommand, MtuConfig, MtuError, MtuResult, UartFraming,
};
