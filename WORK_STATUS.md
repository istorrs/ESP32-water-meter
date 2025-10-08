# ESP32 Water Meter Project - Work Status

**Last Updated**: 2025-10-07
**Current Branch**: master
**GitHub**: https://github.com/istorrs/ESP32-water-meter

## Completed Today

### ✅ MTU Implementation with Serial CLI
- Full-featured CLI over UART0 (115200 baud, GPIO1/3)
- Command history, line editing, TAB completion
- Background MTU thread with message passing architecture
- Hardware timer ISR → Task pattern for precise 1200 baud communication
- Idle line synchronization (10 consecutive 1-bits) - eliminates startup framing errors
- Early exit on message completion (stops when '\r' received)
- Clock pin power control (LOW at boot/after operations)
- Statistics tracking (successful/corrupted reads)
- Zero compiler warnings, passes clippy
- Code formatted and committed

### 📁 Project Structure
```
src/
├── main.rs                    # MTU app entry point
├── lib.rs                     # Library exports
├── cli/                       # CLI infrastructure (shared)
│   ├── mod.rs
│   ├── commands.rs           # MTU CLI commands
│   ├── parser.rs             # MTU command parser
│   └── terminal.rs           # UART terminal with line editing
├── mtu/                      # MTU (reader) implementation
│   ├── mod.rs
│   ├── config.rs
│   ├── error.rs
│   ├── gpio_mtu.rs
│   ├── gpio_mtu_timer.rs
│   ├── gpio_mtu_timer_v2.rs  # Active implementation
│   └── uart_framing.rs
└── meter/                    # Meter (simulator) - NEEDS WORK
    ├── mod.rs
    ├── config.rs
    ├── handler.rs            # Basic frame building only
    └── (missing CLI parser)
```

### 🔧 Current MTU Configuration
- **UART0 CLI**: GPIO1 (TX), GPIO3 (RX) - 115200 baud
- **MTU Clock**: GPIO4 (output)
- **MTU Data**: GPIO5 (input)
- **Default Baud**: 1200 bps (configurable)
- **Protocol**: 7E1 UART framing

### 📊 Latest Commits
```
af6477f - Fix MTU statistics tracking
26b740d - Fix compiler warnings and improve code quality
f01d856 - Update README with accurate project information
de13f8c - Add serial CLI with background MTU thread and idle line sync
```

## 🎯 Next Task: Implement Meter Simulator

### Goal
Create a separate `meter_app` binary that simulates a water meter responding to MTU clock signals.

### Architecture
- **Separate Binary**: `meter_app.rs` (won't run out of space)
- **GPIO Pattern**: Clock pin interrupt → ISR notifies task → Task outputs next bit
- **Pre-computed Frames**: Message converted to bit sequence at startup (already implemented in MeterHandler)
- **Meter CLI**: Commands to configure message, enable/disable, view status

### Implementation Plan

#### Phase 1: Meter CLI Module ⏭️ NEXT
Files to create:
- `src/cli/meter_commands.rs` - Command handler
- `src/cli/meter_parser.rs` - Command parser

Commands needed:
```
meter_enable        - Enable meter response
meter_disable       - Disable meter response
meter_message <txt> - Set response message
meter_type <type>   - Set framing (sensus=7E1, neptune=7E2)
meter_status        - Show config and stats
help, version, status, uptime - Standard commands
```

Reference: `/home/rtp-lab/work/github/nRF52840-DK-rust/src/meter/parser.rs`

#### Phase 2: Update MeterHandler for GPIO Interrupt
Update `src/meter/handler.rs`:
- Add `spawn_meter_thread()` - background thread
- GPIO interrupt on clock pin rising edge
- ISR → Notification pattern (minimal ISR work)
- Task outputs bits from pre-computed buffer
- Track stats: pulses received, bits sent, messages completed

Reference: `/home/rtp-lab/work/github/nRF52840-DK-rust/src/bin/meter_app.rs` lines 44-149

#### Phase 3: Create meter_app.rs Binary
New file: `src/bin/meter_app.rs`
- UART0 CLI (115200 baud, GPIO1/3)
- GPIO4 (clock input, interrupt on rising edge)
- GPIO5 (data output, idle HIGH)
- MeterHandler with default message
- Spawn meter background thread
- CLI loop

#### Phase 4: Build Configuration
Update `Cargo.toml`:
```toml
[[bin]]
name = "mtu_app"
path = "src/main.rs"

[[bin]]
name = "meter_app"
path = "src/bin/meter_app.rs"
```

Update `Makefile`:
```makefile
flash-meter:
    cargo espflash flash --bin meter_app --release --monitor
```

### Key Technical Details

**GPIO Interrupt Handler**:
```rust
// ESP-IDF pattern - minimal ISR work
unsafe {
    clock_pin.subscribe(move || {
        // Just notify task - don't do GPIO here
        notifier.notify_and_yield(NonZeroU32::new(1).unwrap());
    })
}

// Task handles actual bit output
loop {
    notification.wait(Duration::MAX);
    if bit_index < response_bits.len() {
        let bit = response_bits[bit_index];
        data_pin.set_level(if bit == 1 { Level::High } else { Level::Low });
        bit_index += 1;
    }
}
```

**State Machine**:
1. Idle - wait for clock pulses, data HIGH
2. Wake-up - after 10 pulses, start transmission
3. Transmitting - one bit per pulse
4. Complete - return to idle

**Pre-computed Frames** (already in MeterHandler):
- Message → ASCII → UART frames (start + 7 data + parity + stop)
- Stored as Vec<u8> of 0s and 1s
- ~800-900 bits for typical message

### Testing Strategy
1. Flash `meter_app` on ESP32 #1
2. Flash `mtu_app` on ESP32 #2
3. Wire: MTU GPIO4 (clock out) → Meter GPIO4 (clock in)
4. Wire: MTU GPIO5 (data in) ← Meter GPIO5 (data out)
5. Common ground
6. MTU: `mtu_start` → reads meter message
7. Meter: `meter_message "TEST\r"` → changes response

## 🔮 Future Enhancements (Post-Meter)
- WiFi connectivity (ESP32 has built-in WiFi)
- MQTT integration:
  - Meter subscribes to topic for message updates
  - MTU publishes readings to broker
- Web configuration interface
- OTA firmware updates

## 📝 Notes
- nRF reference implementation verified and working
- ESP32 uses ESP-IDF (std Rust) not Embassy (no_std)
- Both projects share common meter/MTU modules
- GPIO interrupt pattern proven in nRF version
