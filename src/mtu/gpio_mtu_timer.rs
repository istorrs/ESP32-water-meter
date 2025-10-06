use super::config::MtuConfig;
use super::error::{MtuError, MtuResult};
use core::sync::atomic::{AtomicBool, AtomicUsize, AtomicU8, Ordering};
use esp_idf_hal::gpio::{Input, Output, PinDriver};
use esp_idf_hal::timer::{TimerDriver, config::Config as TimerConfig, TIMER00};
use heapless::String;
use std::sync::{Arc, Mutex};

/// MTU implementation using hardware timer for precise clock generation
pub struct GpioMtuTimer {
    config: Mutex<MtuConfig>,
    running: Arc<AtomicBool>,
    clock_cycles: Arc<AtomicUsize>,
    last_bit: Arc<AtomicU8>,
    last_message: Mutex<Option<String<256>>>,
}

impl GpioMtuTimer {
    pub fn new(config: MtuConfig) -> Self {
        Self {
            config: Mutex::new(config),
            running: Arc::new(AtomicBool::new(false)),
            clock_cycles: Arc::new(AtomicUsize::new(0)),
            last_bit: Arc::new(AtomicU8::new(0)),
            last_message: Mutex::new(None),
        }
    }

    pub fn get_baud_rate(&self) -> u32 {
        let config = self.config.lock().unwrap();
        config.baud_rate
    }

    pub fn get_stats(&self) -> (u32, u32, usize) {
        let config = self.config.lock().unwrap();
        let cycles = self.clock_cycles.load(Ordering::Relaxed);
        (config.successful_reads, config.corrupted_reads, cycles)
    }

    /// Run MTU operation using hardware timer for precise clock generation
    pub fn run_mtu_operation_with_timer<'a, 'b, P1, P2>(
        &self,
        clock_pin: &mut PinDriver<'a, P1, Output>,
        data_pin: &mut PinDriver<'a, P2, Input>,
        timer_peripheral: TIMER00,
        duration_secs: u64,
    ) -> MtuResult<()>
    where
        P1: esp_idf_hal::gpio::Pin + Send + Sync,
        P2: esp_idf_hal::gpio::Pin + Send + Sync,
    {
        let config = self.config.lock().unwrap();
        let baud_rate = config.baud_rate;
        let power_up_delay_ms = config.power_up_delay_ms;
        drop(config);

        log::info!("MTU: Starting timer-based operation for {} seconds", duration_secs);
        log::info!("MTU: Baud rate: {} Hz", baud_rate);

        // Power up sequence
        clock_pin.set_high().map_err(|_| MtuError::GpioError)?;
        log::info!("MTU: Power-up hold {}ms", power_up_delay_ms);
        esp_idf_hal::delay::FreeRtos::delay_ms(power_up_delay_ms as u32);

        // Set running flag
        self.running.store(true, Ordering::Relaxed);
        self.clock_cycles.store(0, Ordering::Relaxed);

        // Run timer in a scope so it's dropped before we access pins again
        {
            // Create hardware timer
            let timer_config = TimerConfig::new().auto_reload(true);
            let mut timer = TimerDriver::new(timer_peripheral, &timer_config)
                .map_err(|_| MtuError::GpioError)?;

            // Calculate timer frequency: 2x baud rate (for HIGH and LOW phases)
            let timer_freq_hz = baud_rate * 2;
            let alarm_ticks = timer.tick_hz() / timer_freq_hz as u64;

            log::info!("MTU: Timer tick rate: {} Hz", timer.tick_hz());
            log::info!("MTU: Alarm every {} ticks ({} Hz)", alarm_ticks, timer_freq_hz);

            timer.set_alarm(alarm_ticks).map_err(|_| MtuError::GpioError)?;

            // Use subscribe_nonstatic to borrow GPIO pins directly
            // Safety: We ensure timer doesn't outlive the borrowed pins
            unsafe {
                timer.subscribe_nonstatic(|| {
                    let cycle = self.clock_cycles.fetch_add(1, Ordering::Relaxed);
                    let is_high = cycle % 2 == 0;

                    if is_high {
                        // Clock HIGH phase - sample data at this point
                        let _ = clock_pin.set_high();
                        let data_val = data_pin.is_high();
                        self.last_bit.store(if data_val { 1 } else { 0 }, Ordering::Relaxed);
                    } else {
                        // Clock LOW phase
                        let _ = clock_pin.set_low();
                    }
                }).map_err(|_| MtuError::GpioError)?;
            }

            timer.enable_interrupt().map_err(|_| MtuError::GpioError)?;
            timer.enable_alarm(true).map_err(|_| MtuError::GpioError)?;
            timer.enable(true).map_err(|_| MtuError::GpioError)?;

            log::info!("MTU: Timer started, running for {} seconds...", duration_secs);

            // Wait for duration
            let start = std::time::Instant::now();
            let mut last_cycles = 0usize;

            while start.elapsed().as_secs() < duration_secs {
                esp_idf_hal::delay::FreeRtos::delay_ms(1000);

                let current_cycles = self.clock_cycles.load(Ordering::Relaxed);
                let cycles_per_sec = current_cycles - last_cycles;
                last_cycles = current_cycles;

                let elapsed = start.elapsed().as_secs();
                let bit = self.last_bit.load(Ordering::Relaxed);

                log::info!(
                    "MTU: {}/{}s - {} cycles total, {} cycles/sec, last bit: {}",
                    elapsed,
                    duration_secs,
                    current_cycles,
                    cycles_per_sec,
                    bit
                );
            }

            // Stop timer
            self.running.store(false, Ordering::Relaxed);
            timer.enable(false).map_err(|_| MtuError::GpioError)?;

            // Timer will be dropped here, releasing the borrow on pins
        }

        // Now we can access the pins again
        clock_pin.set_high().map_err(|_| MtuError::GpioError)?;

        let total_cycles = self.clock_cycles.load(Ordering::Relaxed);
        log::info!("MTU: Timer operation completed - {} total cycles", total_cycles);

        Ok(())
    }
}
