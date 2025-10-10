# Hardware Setup for Real Water Meter Connection

This guide covers connecting the ESP32 MTU interface to a real water meter using level shifters.

## Overview

The ESP32 operates at **3.3V logic levels**, while many water meters use **5V logic**. Direct connection can damage the ESP32 or result in unreliable communication. A bidirectional level shifter is required.

## Required Components

### For Real Meter Connection

1. **ESP32 Development Board** (with USB-C or Micro USB)
2. **Bidirectional Level Shifter** (3.3V ↔ 5V)
   - Recommended: **Adafruit TXS0108E** - 8-channel, 1.2V-3.6V to 1.65V-5.5V
   - Recommended: **TXS0102 or TXS0104** (Texas Instruments) - 2 or 4 channel
   - Alternative: **SparkFun BOB-12009** - 4-channel BSS138 based
   - Alternative: Generic BSS138 modules - Cost-effective for 2-4 channels
   - NOT recommended: Simple resistor dividers (unreliable at 1200+ baud)
3. **Current Limiting Resistors** (recommended for protection)
   - 270Ω resistors on water meter side prevent overcurrent
   - Protects ESP32 GPIO from voltage spikes and shorts
4. **Pull-up Resistors** (if not included on level shifter board)
   - 4.7kΩ to 10kΩ on the 5V side of the data line
5. **Connecting Wires** (short, <30cm recommended, twisted pair for >30cm)
6. **Meter Interface Cable** (depends on your meter model)

### For ESP32-to-ESP32 Testing (No Level Shifter)

- Two ESP32 boards
- 3 jumper wires (clock, data, GND)
- No additional components required (see README.md)

## Wiring Diagram - ESP32 to Real Meter

```
ESP32 (3.3V)                 Level Shifter              Water Meter (5V)
                             (TXS0102/BSS138)

GPIO4 (Clock) ────> LV1 ─┬─> HV1 ───[270Ω]───> Clock In
                         │
                      [OE to VCC]
                         │
GPIO5 (Data)  <───> LV2 ─┴─> HV2 ───[270Ω]───> Data Out
                                                    │
                                                 [4.7kΩ]
                                                    │
                                                5V Supply

GND ──────────────> GND ─────> GND ──────────> GND

3.3V ─────────────> VCC_A
5V Supply ────────> VCC_B
```

**Protection Notes**:
- 270Ω current limiting resistors protect against shorts and overcurrent
- Pull-up resistor (4.7kΩ) ensures proper idle state (HIGH)
- Level shifter isolates ESP32 from voltage spikes on meter side

## Level Shifter Configuration

### TXS0102/TXS0104 (Recommended)

**Features**:
- Auto-direction sensing (no DIR pin needed)
- Fast switching (suitable for 1200-9600 baud)
- Built-in 10kΩ pull-ups on both sides

**Connections**:
- **VCC_A**: 3.3V from ESP32
- **VCC_B**: 5V from meter power supply
- **OE**: Tie to VCC_A (3.3V) to enable
- **GND**: Common ground
- **A1**: ESP32 GPIO4 (clock)
- **B1**: Meter clock input
- **A2**: ESP32 GPIO5 (data)
- **B2**: Meter data output

### BSS138-Based Level Shifters

**Features**:
- Simple MOSFET-based design
- Bidirectional
- Requires external pull-ups (usually included on module)

**Connections**:
- **LV**: 3.3V from ESP32
- **HV**: 5V from meter power supply
- **GND**: Common ground
- **LV1**: ESP32 GPIO4 (clock)
- **HV1**: Meter clock input
- **LV2**: ESP32 GPIO5 (data)
- **HV2**: Meter data output

**Pull-up Requirements**:
- Most modules include 10kΩ pull-ups
- If not included, add 4.7kΩ - 10kΩ pull-ups to HV1 and HV2 (to 5V)

## GPIO Configuration for Real Meters

### ESP32 MTU Side (3.3V)

**GPIO4 (Clock Output)**:
- Mode: Push-pull output
- Initial state: LOW (no power to meter simulation)
- Function: Generates clock signal (1200 baud default)
- Current: 12mA source/sink (ESP32 spec)

**GPIO5 (Data Input)**:
- Mode: Floating input (no pull-up/down on ESP32 side)
- Function: Reads meter response
- Note: Pull-up is on the 5V side via level shifter or meter

### Meter Side (5V)

**Clock Line**:
- Driven by level shifter HV output
- Meter reads this as clock input
- Typically has internal Schmitt trigger

**Data Line**:
- Driven by meter (open-drain or push-pull output)
- 4.7kΩ - 10kΩ pull-up to 5V required if meter uses open-drain
- Level shifter converts to 3.3V for ESP32

## Power Supply Considerations

### 5V Power for Meter

**Option 1: USB Power Supply**
- Use a separate 5V USB power supply or wall adapter
- Current: Check meter specifications (typically 50-200mA)
- Provide stable 5V to meter and level shifter VCC_B

**Option 2: ESP32 VIN**
- If ESP32 is powered from USB (5V), VIN pin provides 5V
- Check current capacity: USB typically 500mA max
- Suitable for low-power meters only

**Option 3: External Regulated Supply**
- Use LM7805 or switching regulator
- Provides clean, stable 5V
- Best for battery-powered applications

### Ground Connection

**Critical**: All grounds must be connected together:
- ESP32 GND
- Level shifter GND
- Meter GND
- Power supply GND(s)

**Poor ground connections cause**:
- Communication errors
- Corrupted data
- Intermittent failures

## Signal Integrity

### Wire Length

**Recommended**: <30cm (12 inches) for each signal
- Clock and data lines should be similar length
- Shorter is better at 1200 baud and above

**For longer distances** (>30cm):
- Use twisted pair for clock/data with ground return
- Keep baud rate at 1200 or lower
- Consider differential signaling for >2m

### Shielding

**Not required** for typical installations (<1m)

**Use shielded cable if**:
- Near high-power equipment (motors, pumps)
- Outdoor installation
- Distance >1m
- EMI/RFI issues observed

## Meter Interface Pinout

### Common Meter Types

**Sensus Meters** (7E1 format):
- Usually RJ12 or screw terminals
- Check meter documentation for pinout
- Typical: Pin 1 = Clock, Pin 2 = Data, Pin 3 = GND, Pin 4 = +5V

**Neptune Meters** (7E2 format):
- Similar RJ12 or terminal block
- Check meter model datasheet
- Some models require external pull-up on data line

**Generic/Unknown Meters**:
- Probe with multimeter to identify signals:
  - Clock should toggle during read attempts
  - Data should be idle HIGH (pulled to 5V)
  - Measure voltages to confirm 5V logic

## Testing Procedure

### 1. Test Level Shifter Without Meter

```bash
# Connect level shifter between ESP32 and multimeter
# Measure HV1 while running MTU:

ESP32 CLI> mtu_start 5

# Should see:
# - HV1 toggling between 0V and 5V (clock signal)
# - Frequency: ~1200 Hz (1200 baud)
```

### 2. Connect Meter and Test

```bash
# Connect meter through level shifter
# Enable logging and start read:

ESP32 CLI> mtu_start 30

# Check logs for:
# ✅ "Received message: 'V;RB...' "
# ✅ Success rate: 100%

# If seeing errors:
# ❌ "Message received but CORRUPTED" - Check ground and pull-ups
# ❌ "No complete message received" - Check clock signal and baud rate
```

### 3. Verify Message Content

```bash
ESP32 CLI> mtu_status

# Should show:
# - Successful reads > 0
# - Last message matches meter format
# - Success rate close to 100%
```

## Troubleshooting

### No Communication

**Check**:
1. Level shifter VCC_A = 3.3V, VCC_B = 5V
2. Level shifter OE enabled (tied to VCC_A)
3. All grounds connected
4. Clock signal present on meter (measure with oscilloscope)
5. Baud rate matches meter (try 300, 600, 1200, 9600)

### Corrupted Data

**Check**:
1. Pull-up resistor on data line (5V side) - 4.7kΩ to 10kΩ
2. Current limiting resistors (270Ω) on meter side
3. Wire length <30cm (use twisted pair for longer runs)
4. Solid ground connection (measure resistance <1Ω)
5. Clock signal quality with oscilloscope (clean edges, no ringing)
6. Power supply stability (measure voltage under load)

**Try lower baud rates**:
```bash
ESP32 CLI> mtu_baud 600    # Try 600 baud
ESP32 CLI> mtu_start 30

# Or even slower:
ESP32 CLI> mtu_baud 300    # Try 300 baud
ESP32 CLI> mtu_start 30
```

**Improve signal quality**:
- Add 270Ω resistors if not present
- Use shorter wires
- Check for loose connections
- Verify level shifter VCC voltages (3.3V and 5V)

### Intermittent Failures

**Likely causes**:
- Poor ground connection (check with multimeter)
- Loose wires (re-seat connections)
- Power supply voltage drop (measure under load)
- EMI interference (add shielding)

### Wrong Message Format

**Check meter type**:
```bash
ESP32 CLI> mtu_status

# Compare message format to:
# - Sensus: Usually starts with "V;RB..." (7E1)
# - Neptune: May have different format (7E2)
```

**Verify with manufacturer documentation** for your specific meter model.

## Safety Notes

⚠️ **Important Safety Information**:

1. **Never connect 5V directly to ESP32 GPIO** - This will damage the ESP32
2. **Always use a level shifter** for 5V meters
3. **Verify voltage levels with multimeter** before connecting
4. **Disconnect power** before changing wiring
5. **Water meters may be connected to utility systems** - Check local regulations before interfacing

## Multi-Meter Deployment

For monitoring multiple meters with multiple ESP32 devices, see:
- [docs/mqtt-control.md](mqtt-control.md) - Per-device MQTT topics
- README.md - WiFi/MQTT on-demand configuration

Each ESP32 can monitor one meter and publish to the same MQTT broker with unique chip_id identification.

## Additional Hardware Protection (Recommended for Production)

### ESD Protection
- Add TVS diodes or ESD protection chips on exposed GPIO lines
- Protects against electrostatic discharge damage
- Especially important for long cable runs or outdoor installations

### Overcurrent Protection
- 270Ω series resistors on outputs (as shown in wiring diagram)
- Limits current to ~12mA at 3.3V (within ESP32 GPIO spec of 40mA max)
- Prevents damage from accidental shorts

### Power Supply Quality
- Use regulated power supplies with low ripple
- Add 100μF bulk capacitor near ESP32 VIN
- Add 0.1μF ceramic capacitor near 3.3V pin
- Prevents voltage fluctuations that cause communication errors

### EMI Shielding (for Noisy Environments)
- Use shielded twisted pair cable for meter connections
- Connect shield to GND at one end only
- Keep data and clock lines away from high-power wiring

## Related Documentation

This project is compatible with water meter protocols documented in:
- [rust_water_meter_mtu](https://github.com/istorrs/rust_water_meter_mtu) - Raspberry Pi MTU emulator
  - See their [HARDWARE_SETUP.md](/home/rtp-lab/work/github/rust_water_meter_mtu/docs/HARDWARE_SETUP.md) for Raspberry Pi 3.3V GPIO setup
  - Compatible protocol implementation for cross-platform testing

## References

- [ESP32 Datasheet](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf) - GPIO specifications (40mA max per pin)
- [TXS0102 Datasheet](https://www.ti.com/lit/ds/symlink/txs0102.pdf) - Texas Instruments level shifter
- [Adafruit TXS0108E](https://www.adafruit.com/product/395) - 8-channel level shifter
- [SparkFun BOB-12009](https://www.sparkfun.com/products/12009) - BSS138 level shifter
- README.md - GPIO pin assignments and testing
- rust_water_meter_mtu/docs/HARDWARE_SETUP.md - Raspberry Pi equivalent hardware guide
