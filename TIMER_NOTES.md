# Hardware Timer Implementation Notes

## Challenge: GPIO Access from ISR with Safe Rust

The hardware timer implementation in `src/mtu/gpio_mtu_timer.rs` encounters a fundamental limitation with esp-idf-hal's GPIO types:

### The Problem

- ESP32 GPIO pin types (like `Gpio4`, `Gpio5`) are intentionally `!Sync`
- They contain `PhantomData<*const ()>` which prevents sharing between threads
- Timer ISR callbacks require `Send + Sync` bounds
- This makes it impossible to safely access GPIO pins from timer ISR callbacks

### Current Status

The software-delay implementation (`gpio_mtu.rs`) works but runs ~24x slower than configured:
- **Configured**: 1200 baud (833μs per bit)
- **Actual**: ~50 Hz (20ms per cycle)
- **Root cause**: FreeRTOS scheduler overhead and delay imprecision

The timer implementation compiles partially but hits the Sync limitation at the application level.

### Possible Solutions

1. **Use `unsafe` with static mut variables**
   - Store GPIO pin handles in static mut
   - Access from ISR using raw pointers
   - Requires careful synchronization
   - ⚠️ Bypasses Rust's safety guarantees

2. **Use ESP32 RMT (Remote Control) Peripheral**
   - Designed for precise bit-banging
   - Can generate precise waveforms without ISR
   - May require different API approach
   - ✅ Hardware-accelerated, safe

3. **Use raw esp-idf-sys bindings**
   - Bypass esp-idf-hal abstractions
   - Use C-style GPIO manipulation in ISR
   - More control but less safe
   - ⚠️ Requires FFI knowledge

4. **Optimize software delay approach**
   - Use busy-wait instead of FreeRTOS delay
   - Use ESP32 cycle counter for precise timing
   - Still CPU-intensive
   - ⚠️ Blocks other tasks

5. **Accept timing limitations**
   - Current ~50 Hz may be sufficient for testing
   - Focus on higher-level functionality
   - Optimize timing later if needed
   - ✅ Pragmatic for prototyping

## Recommendation

For production use, the **RMT peripheral** is likely the best approach for ESP32 bit-banging applications. It's designed exactly for this use case and provides hardware-accurate timing without ISR complications.

For immediate testing, the software-delay approach works but with reduced timing accuracy.
