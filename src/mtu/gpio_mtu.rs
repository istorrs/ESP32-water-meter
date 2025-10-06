use super::config::MtuConfig;
use super::error::{MtuError, MtuResult};
use core::sync::atomic::{AtomicBool, Ordering};
use embedded_hal::blocking::delay::DelayUs;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Input, Output, PinDriver};
use heapless::String;
use std::sync::Mutex;

pub struct GpioMtu {
    config: Mutex<MtuConfig>,
    running: AtomicBool,
    last_message: Mutex<Option<String<256>>>,
}

impl GpioMtu {
    pub fn new(config: MtuConfig) -> Self {
        Self {
            config: Mutex::new(config),
            running: AtomicBool::new(false),
            last_message: Mutex::new(None),
        }
    }

    pub fn set_baud_rate(&self, baud_rate: u32) {
        let mut config = self.config.lock().unwrap();
        config.baud_rate = baud_rate;
        log::info!("MTU: Baud rate set to {}", baud_rate);
    }

    pub fn get_baud_rate(&self) -> u32 {
        let config = self.config.lock().unwrap();
        config.baud_rate
    }

    pub fn get_config(&self) -> MtuConfig {
        let config = self.config.lock().unwrap();
        config.clone()
    }

    pub fn start(&self) -> MtuResult<()> {
        self.running.store(true, Ordering::Relaxed);
        log::info!("MTU: Starting operation");
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        log::info!("MTU: Stopping operation");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn get_last_message(&self) -> Option<String<256>> {
        let msg = self.last_message.lock().unwrap();
        msg.clone()
    }

    pub fn clear_last_message(&self) {
        let mut msg = self.last_message.lock().unwrap();
        *msg = None;
    }

    pub fn set_expected_message(&self, expected: String<256>) {
        let mut config = self.config.lock().unwrap();
        log::info!("MTU: Expected message set to: {}", expected.as_str());
        config.expected_message = expected;
    }

    pub fn get_expected_message(&self) -> String<256> {
        let config = self.config.lock().unwrap();
        config.expected_message.clone()
    }

    pub fn get_stats(&self) -> (u32, u32) {
        let config = self.config.lock().unwrap();
        (config.successful_reads, config.corrupted_reads)
    }

    pub fn reset_stats(&self) {
        let mut config = self.config.lock().unwrap();
        config.successful_reads = 0;
        config.corrupted_reads = 0;
        log::info!("MTU: Statistics reset");
    }

    // Helper method to evaluate and record a message result
    fn record_message_result(&self, received_message: Option<String<256>>) -> bool {
        let mut config = self.config.lock().unwrap();
        let expected = config.expected_message.clone();

        if let Some(received) = received_message {
            if received == expected {
                config.successful_reads += 1;
                log::info!(
                    "MTU: Message SUCCESS - Stats: {}/{}",
                    config.successful_reads,
                    config.successful_reads + config.corrupted_reads
                );
                true
            } else {
                config.corrupted_reads += 1;
                log::error!(
                    "MTU: Message CORRUPTED - Expected: '{}', Received: '{}' - Stats: {}/{}",
                    expected.as_str(),
                    received.as_str(),
                    config.successful_reads,
                    config.successful_reads + config.corrupted_reads
                );
                false
            }
        } else {
            config.corrupted_reads += 1;
            log::error!(
                "MTU: Message CORRUPTED - No message received - Stats: {}/{}",
                config.successful_reads,
                config.successful_reads + config.corrupted_reads
            );
            false
        }
    }

    /// Run a simple MTU operation
    /// This is a blocking, synchronous version for initial testing
    pub fn run_mtu_operation<'a, P1, P2>(
        &self,
        clock_pin: &mut PinDriver<'a, P1, Output>,
        data_pin: &mut PinDriver<'a, P2, Input>,
        duration_secs: u64,
    ) -> MtuResult<()>
    where
        P1: esp_idf_hal::gpio::Pin,
        P2: esp_idf_hal::gpio::Pin,
    {
        let config = self.config.lock().unwrap();
        let power_up_delay_ms = config.power_up_delay_ms;
        let bit_duration_micros = config.bit_duration_micros();
        let framing = config.framing;
        drop(config);

        log::info!("MTU: Starting meter reading for {} seconds", duration_secs);

        // Set running flag
        self.running.store(true, Ordering::Relaxed);

        // Power up sequence: Set clock HIGH and hold for power_up_delay_ms
        clock_pin.set_high().map_err(|_| MtuError::GpioError)?;
        log::info!(
            "MTU: Setting clock HIGH for {}ms power-up hold period",
            power_up_delay_ms
        );
        FreeRtos::delay_ms(power_up_delay_ms as u32);
        log::info!("MTU: Power-up hold complete, starting clock cycles");

        let mut clock_cycle_count = 0u64;
        let start_time = std::time::Instant::now();
        let mut delay = FreeRtos;

        // Main MTU operation loop
        while self.running.load(Ordering::Relaxed)
            && start_time.elapsed().as_secs() < duration_secs
        {
            clock_cycle_count += 1;

            // Clock LOW phase
            clock_pin.set_low().map_err(|_| MtuError::GpioError)?;

            // Delay for half the bit period (in microseconds)
            delay.delay_us((bit_duration_micros / 2) as u32);

            // Sample data line
            let data_val = data_pin.is_high();
            let data_bit = if data_val { 1 } else { 0 };

            // Clock HIGH phase
            clock_pin.set_high().map_err(|_| MtuError::GpioError)?;

            // Delay for half the bit period
            delay.delay_us((bit_duration_micros / 2) as u32);

            // TODO: Implement proper UART frame collection and character extraction
            // For now, this is a simplified version that just logs the bit values
            if clock_cycle_count % 100 == 0 {
                log::info!("MTU: Clock cycle {}, bit: {}", clock_cycle_count, data_bit);
            }
        }

        // Set clock to idle state (HIGH)
        clock_pin.set_high().map_err(|_| MtuError::GpioError)?;

        // Clear running flag
        self.running.store(false, Ordering::Relaxed);

        log::info!(
            "MTU: Operation completed after {} clock cycles",
            clock_cycle_count
        );
        Ok(())
    }
}
