# ESP32 Water Meter MTU Interface

Water meter MTU (Meter Transmission Unit) interface for ESP32 using Rust and ESP-IDF with serial CLI control.

## Features

- **Serial CLI**: Interactive command-line interface over UART0 (115200 baud, USB-C connection)
  - Command history, line editing, TAB autocompletion
  - Real-time MTU control and status monitoring
- **MTU Communication**: 1200 baud serial communication with water meter
  - GPIO bit-banging with hardware timer ISR for precise timing
  - Automatic idle line synchronization for reliable message capture
  - Early exit on complete message reception
- **Background Thread Architecture**: Non-blocking MTU operations
  - Main thread handles CLI interaction
  - MTU thread manages GPIO/timer operations via message passing

## Hardware

- **MCU**: ESP32 (Xtensa dual-core)
- **Framework**: ESP-IDF (std Rust)
- **HAL**: esp-idf-hal

## GPIO Pin Assignments

- **UART0 (USB-C)**: GPIO1 (TX), GPIO3 (RX) - 115200 baud CLI
- **MTU Clock**: GPIO4 (output)
- **MTU Data**: GPIO5 (input)

## Prerequisites

1. **Rust ESP toolchain**:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install ESP Rust toolchain
   cargo install espup
   espup install
   source ~/export-esp.sh  # Add to your shell profile
   ```

2. **espflash**:
   ```bash
   cargo install espflash
   ```

## Building and Flashing

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Flash and monitor
espflash flash target/xtensa-esp32-espidf/release/esp32-water-meter --monitor

# Or use cargo-espflash
cargo espflash flash --release --monitor
```

## CLI Commands

Once flashed, connect via USB-C and use a serial terminal (115200 baud):

```
ESP32 CLI> help

Available commands:
  help             - Show this help
  version          - Show firmware version
  status           - Show system status
  uptime           - Show system uptime
  clear            - Clear terminal
  reset            - Reset system

  mtu_start [dur]  - Start MTU operation (default 30s)
  mtu_stop         - Stop MTU operation
  mtu_status       - Show MTU status and statistics
  mtu_baud <rate>  - Set MTU baud rate (default 1200)
  mtu_reset        - Reset MTU statistics
```

### Example Usage

```bash
# Start MTU operation for 30 seconds
ESP32 CLI> mtu_start

# Check status
ESP32 CLI> mtu_status
MTU Status:
  State: Stopped
  Baud rate: 1200 bps
  Pins: GPIO4 (clock), GPIO5 (data)
  Total cycles: 5091
  Statistics:
    Successful reads: 1
    Corrupted reads: 0
    Success rate: 100.0%
  Last message: V;RB00000200;IB61564400;A1000;Z3214;...

# Set different baud rate
ESP32 CLI> mtu_baud 2400

# Start 60 second operation
ESP32 CLI> mtu_start 60
```

## Architecture

### Thread Model
- **Main Thread**: UART CLI loop processing user input
- **MTU Thread**: Background thread owning GPIO pins and timer
  - Receives commands via `mpsc::channel`
  - Spawns per-operation UART framing task
  - Reusable timer ISR for unlimited operations

### MTU Operation Flow
1. Power-up sequence (clock HIGH, 10ms delay)
2. Hardware timer generates 4-phase clock signal (4800 Hz for 1200 baud)
3. ISR increments cycle counter and notifies GPIO task
4. GPIO task toggles clock pin and samples data line
5. UART framing task processes bit stream into 7E1 frames
6. Validates parity and stop bit, extracts ASCII characters
7. Exits early on carriage return (`\r`) or timeout
8. Clock pin set LOW to simulate no power to meter

### Message Format
- **Protocol**: 7E1 UART (7 data bits, even parity, 1 stop bit)
- **Baud Rate**: 1200 bps (configurable)
- **Message**: ASCII text ending with `\r`
- **Example**: `V;RB00000200;IB61564400;A1000;Z3214;XT0746;MT0683;...`

## Technical Details

- **Timer ISR → Task Pattern**: Hardware timer ISR for precise timing, FreeRTOS task for GPIO operations
- **Idle Line Sync**: Waits for 10 consecutive 1-bits before frame detection to prevent mid-transmission startup
- **Efficiency**: ~83-84% ISR notification → task handling efficiency
- **Early Exit**: Operation completes immediately upon receiving complete message
- **Power Simulation**: Clock pin LOW at bootup and after operations to simulate no power to meter

## License

MIT OR Apache-2.0
