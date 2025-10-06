# ESP32 Water Meter MTU Interface

Water meter MTU (Meter Transmission Unit) interface for ESP32 (Xtensa) using Rust and Embassy async framework.

## Hardware

- **Chip**: ESP32 (Xtensa dual-core)
- **Framework**: Embassy async (no_std)
- **HAL**: esp-hal 1.0.0-rc.0

## Prerequisites

1. **ESP Xtensa toolchain**:
   ```bash
   cargo install espup
   espup install
   source ~/export-esp.sh  # Add to your shell profile
   ```

2. **espflash**:
   ```bash
   cargo install espflash
   ```

## Building

```bash
# Build
cargo +esp build -Zbuild-std=core,alloc

# Flash
cargo +esp run -Zbuild-std=core,alloc

# Or use Makefile
make flash
```

## Project Status

🚧 **In Development** - Porting from ESP32-C3 version

## GPIO Pin Assignments

- **MTU Clock Out**: GPIO6
- **MTU Data In**: GPIO7
- **LEDs**: GPIO2-5
- **Buttons**: GPIO9, 10, 18, 19
- **UART**: USB Serial

## License

MIT OR Apache-2.0
