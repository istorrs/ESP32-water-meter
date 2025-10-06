# Hardware Timer Implementation Notes

## ✅ SUCCESS: ISR→Task Pattern Implementation

### The Solution

The correct approach is the **ISR→Task pattern** using FreeRTOS notifications:

1. **Timer ISR** (interrupt context):
   - Runs at precise hardware timing (2400 Hz for 1200 baud)
   - Only touches atomic counters and sends notifications
   - **Does NOT access GPIO** - fully thread-safe
   - Uses `esp_idf_hal::timer::TimerDriver::subscribe()`

2. **GPIO Task** (normal FreeRTOS task context):
   - Waits for notifications from ISR
   - Safely owns and manipulates GPIO pins
   - Toggles clock pin and samples data pin
   - Uses `esp_idf_hal::task::notification::Notification`

### Implementation

File: `src/mtu/gpio_mtu_timer_v2.rs`

```rust
// ISR: Only atomics and notifications (Send + Sync ✓)
unsafe {
    timer.subscribe(move || {
        let cycle = cycles.fetch_add(1, Ordering::Relaxed);
        let phase = (cycle % 2) as u32;
        if let Some(bits) = NonZeroU32::new(phase + 1) {
            notifier.notify_and_yield(bits);  // Signal task
        }
    })?;
}

// Task: Safe GPIO access
while running {
    if let Some(bitset) = notification.wait(timeout) {
        let phase = bitset.get() - 1;
        if phase == 0 {
            clock_pin.set_high()?;
            let data = data_pin.is_high();
        } else {
            clock_pin.set_low()?;
        }
    }
}
```

### Performance Results

**Hardware Test (Oct 6, 2025):**
- ✅ Timer tick rate: 1,000,000 Hz (1 MHz crystal)
- ✅ Alarm interval: 416 ticks = 2,400 Hz (2× 1200 baud)
- ✅ ISR generated: 24,059 ticks in 10 seconds = 2,405.9 Hz
- ✅ Task handled: 24,040 GPIO updates
- ✅ **Efficiency: 99.9%** - Task kept up nearly perfectly!
- ✅ **Actual baud rate: 1,203 Hz** (0.25% error from target 1200 Hz)

### Why This Works

- **ISR**: FreeRTOS ISRs can use `Send` types like atomics and notifications
- **Task**: Normal task context can safely own `!Sync` GPIO types
- **Communication**: FreeRTOS notification is a zero-copy signal (just a u32 bitset)
- **Performance**: Task overhead is minimal, achieving 99.9% efficiency

### Configuration

Added `sdkconfig.defaults`:
```
CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192
```

Increased stack size prevents overflow when running GPIO task alongside main loop.

## Previous Attempts

### ❌ Attempt 1: Direct GPIO Access from ISR (`gpio_mtu_timer.rs`)

**Problem**: GPIO types are `!Sync` - cannot be accessed from ISR callbacks even with `unsafe`.

**Error**:
```
error[E0277]: `*const ()` cannot be shared between threads safely
```

GPIO pin types contain `PhantomData<*const ()>` which intentionally prevents sharing between threads.

### ❌ Attempt 2: Software Delays (`gpio_mtu.rs`)

**Problem**: FreeRTOS scheduler overhead causes ~24× timing error.

**Results**:
- Configured: 1200 baud (833μs per bit)
- Actual: ~50 Hz (20ms per cycle)
- Root cause: `FreeRtos::delay_us()` has significant overhead

## Conclusion

The **ISR→Task pattern** is the correct real-time approach for ESP32 GPIO bit-banging:
- ✅ Safe Rust (no unsafe GPIO access)
- ✅ Precise hardware timing (1 MHz timer)
- ✅ Excellent efficiency (99.9%)
- ✅ Standard FreeRTOS pattern

This validates the fundamental real-time design principle: **Separate timing from I/O**.

## Alternative: RMT Peripheral

For even better performance, the ESP32 RMT (Remote Control) peripheral could be used:
- Hardware generates waveforms without any CPU intervention
- No ISR or task overhead
- Designed specifically for bit-banging protocols
- May be explored in future iterations
