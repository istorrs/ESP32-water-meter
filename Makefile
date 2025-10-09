# Makefile for ESP32 Water Meter MTU/Meter (ESP-IDF)

.PHONY: all build flash release flash-release build-meter flash-meter flash-meter-release monitor clean help

# Default target
all: build

# === MTU App Targets ===

# Build MTU (debug)
build:
	@echo "🔧 Building ESP32 MTU app (debug) with ESP-IDF..."
	cargo build --bin mtu_app

# Build MTU (release)
release:
	@echo "🔧 Building ESP32 MTU app (release) with ESP-IDF..."
	cargo build --bin mtu_app --release

# Flash MTU (debug)
flash: build
	@echo "📱 Flashing ESP32 MTU app (debug)..."
	cargo run --bin mtu_app

# Flash MTU (release) - let cargo handle bootloader/partition table
flash-release: release
	@echo "📱 Flashing ESP32 MTU app (release)..."
	cargo run --bin mtu_app --release

# === Meter App Targets ===

# Build Meter (debug)
build-meter:
	@echo "🔧 Building ESP32 Meter app (debug) with ESP-IDF..."
	cargo build --bin meter_app

# Build Meter (release)
release-meter:
	@echo "🔧 Building ESP32 Meter app (release) with ESP-IDF..."
	cargo build --bin meter_app --release

# Flash Meter (debug)
flash-meter: build-meter
	@echo "📱 Flashing ESP32 Meter app (debug)..."
	cargo run --bin meter_app

# Flash Meter (release)
flash-meter-release: release-meter
	@echo "📱 Flashing ESP32 Meter app (release)..."
	cargo run --bin meter_app --release

# Monitor
monitor:
	@echo "🖥️  Opening serial monitor..."
	espflash serial-monitor

# Clean
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean

# Help
help:
	@echo "ESP32 Water Meter MTU/Meter (ESP-IDF) - Available Commands:"
	@echo ""
	@echo "MTU App (MTU Reader):"
	@echo "  make build              - Build MTU app (debug)"
	@echo "  make release            - Build MTU app (release)"
	@echo "  make flash              - Flash MTU app (debug)"
	@echo "  make flash-release      - Flash MTU app (release)"
	@echo ""
	@echo "Meter App (Meter Simulator):"
	@echo "  make build-meter        - Build Meter app (debug)"
	@echo "  make release-meter      - Build Meter app (release)"
	@echo "  make flash-meter        - Flash Meter app (debug)"
	@echo "  make flash-meter-release - Flash Meter app (release)"
	@echo ""
	@echo "Utilities:"
	@echo "  make monitor            - Open serial monitor"
	@echo "  make clean              - Clean build artifacts"
	@echo "  make help               - Show this help"
	@echo ""
	@echo "GPIO Configuration:"
	@echo "  MTU:   GPIO4 (clock out) → GPIO5 (data in)"
	@echo "  Meter: GPIO4 (clock in)  ← GPIO5 (data out)"
	@echo ""
	@echo "Testing: Connect MTU GPIO4→Meter GPIO4, MTU GPIO5←Meter GPIO5, GND"
	@echo "Note: ESP-IDF will be automatically downloaded and configured on first build"
