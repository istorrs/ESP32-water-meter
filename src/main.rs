#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    prelude::*,
};
use esp_println::println;

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    println!("ESP32 Water Meter MTU Interface");
    println!("Initializing...");

    let delay = Delay::new();

    loop {
        println!("Hello from ESP32!");
        delay.delay_millis(1000);
    }
}
