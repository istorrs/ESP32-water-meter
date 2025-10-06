use super::config::MtuConfig;
use super::error::{MtuError, MtuResult};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use esp_idf_hal::gpio::{Input, Output, PinDriver};
use esp_idf_hal::timer::{TimerDriver, config::Config as TimerConfig, TIMER00};
use esp_idf_hal::task::notification::Notification;
use heapless::String;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

/// MTU implementation using hardware timer ISR -> Task pattern
/// ISR handles precise timing, signals task which handles GPIO
pub struct GpioMtuTimerV2 {
    config: Mutex<MtuConfig>,
    running: Arc<AtomicBool>,
    clock_cycles: Arc<AtomicUsize>,
    last_bit: Arc<AtomicU8>,
    last_message: Mutex<Option<String<256>>>,
}

use core::sync::atomic::AtomicU8;

impl GpioMtuTimerV2 {
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

    /// Run MTU operation: ISR generates timing signals, task handles GPIO
    pub fn run_mtu_operation_with_timer<'a, P1, P2>(
        &self,
        clock_pin: &mut PinDriver<'a, P1, Output>,
        data_pin: &mut PinDriver<'a, P2, Input>,
        timer_peripheral: TIMER00,
        duration_secs: u64,
    ) -> MtuResult<()>
    where
        P1: esp_idf_hal::gpio::Pin,
        P2: esp_idf_hal::gpio::Pin,
    {
        let config = self.config.lock().unwrap();
        let baud_rate = config.baud_rate;
        let power_up_delay_ms = config.power_up_delay_ms;
        drop(config);

        log::info!("MTU: Starting ISR->Task timer operation for {} seconds", duration_secs);
        log::info!("MTU: Baud rate: {} Hz", baud_rate);

        // Power up sequence
        clock_pin.set_high().map_err(|_| MtuError::GpioError)?;
        log::info!("MTU: Power-up hold {}ms", power_up_delay_ms);
        esp_idf_hal::delay::FreeRtos::delay_ms(power_up_delay_ms as u32);

        // Set running flag
        self.running.store(true, Ordering::Relaxed);
        self.clock_cycles.store(0, Ordering::Relaxed);

        // Create notification for ISR -> Task communication
        let notification = Notification::new();
        let notifier = notification.notifier();

        // Clone Arc for ISR closure
        let cycles = self.clock_cycles.clone();

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

        // ISR: Just increment counter and notify task
        // Safety: Only accesses atomics and notification, both are Send+Sync
        unsafe {
            timer.subscribe(move || {
                let cycle = cycles.fetch_add(1, Ordering::Relaxed);
                // Encode HIGH(0) or LOW(1) phase in the notification
                let phase = (cycle % 2) as u32;
                if let Some(bits) = NonZeroU32::new(phase + 1) {
                    notifier.notify_and_yield(bits);
                }
            }).map_err(|_| MtuError::GpioError)?;
        }

        timer.enable_interrupt().map_err(|_| MtuError::GpioError)?;
        timer.enable_alarm(true).map_err(|_| MtuError::GpioError)?;
        timer.enable(true).map_err(|_| MtuError::GpioError)?;

        log::info!("MTU: Timer started, GPIO task running...");

        // Task: Handle GPIO based on notifications from ISR
        let start = std::time::Instant::now();
        let mut last_log_time = start;
        let mut last_cycles = 0usize;
        let mut handled_count = 0usize;

        while start.elapsed().as_secs() < duration_secs {
            // Wait for notification from ISR (1 tick timeout ~= 1ms)
            if let Some(bitset) = notification.wait(1) {
                handled_count += 1;
                let phase = bitset.get() - 1;

                if phase == 0 {
                    // HIGH phase - sample data
                    clock_pin.set_high().map_err(|_| MtuError::GpioError)?;
                    let data_val = data_pin.is_high();
                    self.last_bit.store(if data_val { 1 } else { 0 }, Ordering::Relaxed);
                } else {
                    // LOW phase
                    clock_pin.set_low().map_err(|_| MtuError::GpioError)?;
                }
            }

            // Log status every second
            if start.elapsed().as_secs() > last_log_time.elapsed().as_secs() {
                let current_cycles = self.clock_cycles.load(Ordering::Relaxed);
                let cycles_per_sec = current_cycles - last_cycles;
                last_cycles = current_cycles;
                last_log_time = std::time::Instant::now();

                let elapsed = start.elapsed().as_secs();
                let bit = self.last_bit.load(Ordering::Relaxed);

                log::info!(
                    "MTU: {}/{}s - ISR: {} cycles, Task: {} handled, {} cycles/sec, bit: {}",
                    elapsed,
                    duration_secs,
                    current_cycles,
                    handled_count,
                    cycles_per_sec,
                    bit
                );
            }
        }

        // Stop timer
        self.running.store(false, Ordering::Relaxed);
        timer.enable(false).map_err(|_| MtuError::GpioError)?;

        // Set clock to idle (HIGH)
        clock_pin.set_high().map_err(|_| MtuError::GpioError)?;

        let total_cycles = self.clock_cycles.load(Ordering::Relaxed);
        log::info!("MTU: Timer operation completed");
        log::info!("  ISR generated: {} timer ticks", total_cycles);
        log::info!("  Task handled: {} GPIO updates", handled_count);
        log::info!("  Efficiency: {:.1}%", (handled_count as f32 / total_cycles as f32) * 100.0);

        Ok(())
    }
}
