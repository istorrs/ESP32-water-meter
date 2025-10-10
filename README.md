# ESP32 Water Meter MTU/Meter System

Complete water meter testing system with MTU (Meter Transmission Unit) reader and Meter simulator for ESP32 using Rust and ESP-IDF.

## Overview

This project provides **two separate applications**:

1. **MTU App** (`mtu_app`) - Reads water meter data by generating clock signals and capturing serial responses
2. **Meter App** (`meter_app`) - Simulates a water meter responding to MTU clock signals with configurable messages

Both apps feature interactive serial CLI control over UART0 (115200 baud, USB-C connection).

## Features

### Common Features
- **Serial CLI**: Interactive command-line interface with history, line editing, TAB autocompletion
- **Background Thread Architecture**: Non-blocking operations with main CLI thread
- **GPIO Communication**: 1200 baud serial over GPIO4 (clock) and GPIO5 (data)

### MTU App Features
- Hardware timer ISR for precise 1200 baud clock generation
- Automatic idle line synchronization for reliable message capture
- Early exit on complete message reception
- Message validation with parity checking and statistics

### Meter App Features
- GPIO interrupt-based clock detection (rising edge)
- Pre-computed UART frame generation (7E1/7E2)
- Wake-up threshold (10 pulses) before transmission
- Configurable meter types (Sensus 7E1, Neptune 7E2)
- Customizable response messages via CLI

## Hardware

- **MCU**: ESP32 (Xtensa dual-core)
- **Framework**: ESP-IDF (std Rust)
- **HAL**: esp-idf-hal

## GPIO Pin Assignments

### Both Apps (Common)
- **UART0 (USB-C)**: GPIO1 (TX), GPIO3 (RX) - 115200 baud CLI

### MTU App
- **Clock**: GPIO4 (output) - Generates 1200 baud clock
- **Data**: GPIO5 (input) - Reads meter response

### Meter App
- **Clock**: GPIO4 (input with interrupt) - Detects MTU clock
- **Data**: GPIO5 (output, idle HIGH) - Sends response to MTU

### Testing Configuration
Connect two ESP32 devices:
```
MTU GPIO4 (clock out) ──→ Meter GPIO4 (clock in)
MTU GPIO5 (data in)  ←── Meter GPIO5 (data out)
MTU GND              ──── Meter GND
```

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

### Using Makefile (Recommended)

```bash
# MTU App
make build              # Build MTU (debug)
make flash              # Flash MTU (debug)
make flash-release      # Flash MTU (release)

# Meter App
make build-meter        # Build Meter (debug)
make flash-meter        # Flash Meter (debug)
make flash-meter-release # Flash Meter (release)

# Utilities
make monitor            # Serial monitor
make clean              # Clean build
make help               # Show all commands
```

### Using Cargo Directly

```bash
# MTU App
cargo build --bin mtu_app --release
cargo run --bin mtu_app --release

# Meter App
cargo build --bin meter_app --release
cargo run --bin meter_app --release
```

## CLI Commands

Once flashed, connect via USB-C and use a serial terminal (115200 baud).

### MTU App Commands

```
ESP32 CLI> help

Available commands:
  help             - Show this help
  version          - Show firmware version
  status           - Show system status
  uptime           - Show system uptime
  clear            - Clear terminal
  reset            - Reset system
  echo <text>      - Echo text back

  mtu_start [dur]  - Start MTU operation (default 30s)
  mtu_stop         - Stop MTU operation
  mtu_status       - Show MTU status and statistics
  mtu_baud <rate>  - Set MTU baud rate (1-115200, default 1200)
  mtu_reset        - Reset MTU statistics
```

### Meter App Commands

```
ESP32 CLI> help

Available commands:
  help             - Show this help
  version          - Show firmware version
  status           - Show meter status and statistics
  uptime           - Show system uptime
  clear            - Clear terminal
  reset            - Reset system

  enable           - Enable meter response to clock signals
  disable          - Disable meter response
  type <sensus|neptune> - Set meter type (7E1 or 7E2)
  message <text>   - Set response message (\r added automatically)
```

### Example Usage

#### MTU App
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

#### Meter App
```bash
# Check meter status
ESP32 CLI> status
Meter Status:
  State: Enabled
  Type: Sensus
  Pins: GPIO4 (clock in), GPIO5 (data out)
  Message: 'V;RB00000200;IB61564400;...' (70 chars)
  Statistics:
    Clock pulses: 5091
    Bits transmitted: 560
    Messages sent: 1
    Currently transmitting: No

# Set custom message
ESP32 CLI> message TEST123

# Change meter type to Neptune (7E2)
ESP32 CLI> type neptune

# Disable response
ESP32 CLI> disable
```

## Architecture

### MTU App Architecture

#### Thread Model
- **Main Thread**: UART CLI loop processing user input
- **MTU Thread**: Background thread owning GPIO pins and hardware timer
  - Receives commands via `mpsc::channel`
  - Spawns per-operation UART framing task
  - Reusable timer ISR for unlimited operations

#### Operation Flow
1. Power-up sequence (clock HIGH, 10ms delay)
2. Hardware timer generates 4-phase clock signal (4800 Hz for 1200 baud)
3. Timer ISR increments cycle counter and notifies GPIO task
4. GPIO task toggles clock pin and samples data line
5. UART framing task processes bit stream into 7E1 frames
6. Validates parity and stop bit, extracts ASCII characters
7. Exits early on carriage return (`\r`) or timeout
8. Clock pin set LOW to simulate no power to meter

#### Technical Details
- **Timer ISR → Task Pattern**: Hardware timer ISR for precise timing, FreeRTOS task for GPIO
- **Idle Line Sync**: Waits for 10 consecutive 1-bits before frame detection
- **Efficiency**: ~83-84% ISR notification → task handling efficiency
- **Early Exit**: Completes immediately upon receiving `\r`
- **Power Simulation**: Clock LOW at bootup and after operations

### Meter App Architecture

#### Thread Model
- **Main Thread**: UART CLI loop processing user input
- **Meter Thread**: Background thread with GPIO interrupt handler
  - Clock pin interrupt (rising edge) triggers ISR
  - ISR notifies task via FreeRTOS notification
  - Task outputs pre-computed bits on data line

#### Operation Flow
1. Clock pin idle (LOW from MTU)
2. Clock rising edge triggers GPIO interrupt (ISR)
3. ISR sends notification to meter task (minimal work)
4. Task increments pulse counter
5. After 10 pulses (wake-up threshold), builds response frames
6. On each subsequent clock pulse, outputs next bit on data line
7. Returns to idle after complete message transmitted

#### Technical Details
- **ISR → Notification Pattern**: Minimal ISR work, heavy lifting in task
- **Wake-up Threshold**: 10 pulses before transmission starts
- **Pre-computed Frames**: Message → ASCII → UART frames (7E1/7E2) → bit array
- **State Machine**: Idle → Wake-up → Transmitting → Complete
- **Frame Format**: Configurable 7E1 (Sensus) or 7E2 (Neptune)

### Message Format
- **Protocol**: 7E1 or 7E2 UART
  - 7E1: 7 data bits, even parity, 1 stop bit (Sensus)
  - 7E2: 7 data bits, even parity, 2 stop bits (Neptune)
- **Baud Rate**: 1200 bps (default)
- **Message**: ASCII text ending with `\r`
- **Example**: `V;RB00000200;IB61564400;A1000;Z3214;XT0746;MT0683;...`

## License

MIT OR Apache-2.0
