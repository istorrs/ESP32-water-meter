# Makefile for ESP32 Water Meter MTU Interface

.PHONY: all build flash release flash-release monitor clean help

# Default target
all: build

# Build (debug)
build:
	@echo "🔧 Building ESP32 MTU app (debug)..."
	@bash -c "source ~/export-esp.sh && cargo +esp build -Zbuild-std=core,alloc"

# Build (release)
release:
	@echo "🔧 Building ESP32 MTU app (release)..."
	@bash -c "source ~/export-esp.sh && cargo +esp build --release -Zbuild-std=core,alloc"

# Flash (debug)
flash: build
	@echo "📱 Flashing ESP32 MTU app (debug)..."
	@bash -c "source ~/export-esp.sh && espflash flash --monitor target/xtensa-esp32-none-elf/debug/esp32-water-meter"

# Flash (release)
flash-release: release
	@echo "📱 Flashing ESP32 MTU app (release)..."
	@bash -c "source ~/export-esp.sh && espflash flash --monitor target/xtensa-esp32-none-elf/release/esp32-water-meter"

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
	@echo "ESP32 Water Meter MTU Interface - Available Commands:"
	@echo ""
	@echo "  make build         - Build debug version"
	@echo "  make release       - Build release version"
	@echo "  make flash         - Build and flash debug version"
	@echo "  make flash-release - Build and flash release version"
	@echo "  make monitor       - Open serial monitor"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make help          - Show this help"
	@echo ""
	@echo "Prerequisites:"
	@echo "  source ~/export-esp.sh  # Must be run before building"
