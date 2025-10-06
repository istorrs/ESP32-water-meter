# Makefile for ESP32 Water Meter MTU Interface (ESP-IDF)

.PHONY: all build flash release flash-release monitor clean help

# Default target
all: build

# Build (debug)
build:
	@echo "🔧 Building ESP32 MTU app (debug) with ESP-IDF..."
	cargo build

# Build (release)
release:
	@echo "🔧 Building ESP32 MTU app (release) with ESP-IDF..."
	cargo build --release

# Flash (debug)
flash: build
	@echo "📱 Flashing ESP32 MTU app (debug)..."
	cargo run

# Flash (release)
flash-release: release
	@echo "📱 Flashing ESP32 MTU app (release)..."
	cargo run --release

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
	@echo "ESP32 Water Meter MTU Interface (ESP-IDF) - Available Commands:"
	@echo ""
	@echo "  make build         - Build debug version"
	@echo "  make release       - Build release version"
	@echo "  make flash         - Build and flash debug version"
	@echo "  make flash-release - Build and flash release version"
	@echo "  make monitor       - Open serial monitor"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make help          - Show this help"
	@echo ""
	@echo "Note: ESP-IDF will be automatically downloaded and configured on first build"
