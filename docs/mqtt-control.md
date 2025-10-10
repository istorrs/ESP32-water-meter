# MQTT Control Messages

The ESP32 water meter MTU interface supports remote control and configuration via MQTT messages on the control topic.

## Topics

- **Control Topic**: `istorrs/mtu/control` (subscribe - receive commands)
- **Data Topic**: `istorrs/mtu/data` (publish - send meter readings)

## Data Payload Format

Each meter reading published to `istorrs/mtu/data` includes device identification:

```json
{
  "chip_id": "24:0a:c4:12:34:56",
  "wifi_mac": "24:0a:c4:12:34:57",
  "wifi_ip": "192.168.1.119",
  "message": "V;RB00000200;IB61564400;A1000;Z3214;XT0746;MT0683;RR00000000;GX000000;GN000000",
  "baud_rate": 1200,
  "cycles": 15,
  "successful": 2,
  "corrupted": 0,
  "count": 5
}
```

**Device Identification Fields**:
- `chip_id` - ESP32 base MAC address from eFuse (unique identifier, persists across reboots)
- `wifi_mac` - WiFi station MAC address (may differ from chip_id)
- `wifi_ip` - Current IP address assigned by DHCP

**Meter Data Fields**:
- `message` - Raw meter response string
- `baud_rate` - Current MTU baud rate setting
- `cycles` - Total clock cycles sent
- `successful` - Number of successful reads
- `corrupted` - Number of corrupted reads (frame errors)
- `count` - Sequential message counter

## Message Formats

### JSON Format (Recommended)

JSON messages provide a structured way to send configuration and commands.

#### Set Baud Rate

Configure the MTU baud rate (persists across ESP32 reconnects when using `retain`):

```bash
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"baud_rate":1200}' -q 1 -r
```

**Supported baud rates**: 1-115200 bps (typical water meters: 300, 600, 1200, 9600)

**Important**:
- Use **QoS 1** (`-q 1`) for reliable delivery
- Use **retain** (`-r`) so the configuration persists and is delivered to ESP32 on every connect
- Baud rate can only be changed when MTU is stopped

#### Start MTU with Duration

Start the MTU for a specific duration:

```bash
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"command":"start","duration":60}' -q 1
```

Duration is in seconds (default: 30s if not specified).

#### Stop MTU

Stop the currently running MTU operation:

```bash
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"command":"stop"}' -q 1
```

### Plain Text Format (Legacy)

For backwards compatibility, plain text commands are still supported:

```bash
# Start with default 30s duration
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m "start" -q 1

# Start with custom duration
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m "start 60" -q 1

# Stop
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m "stop" -q 1
```

## QoS Recommendations

### QoS 0 (At most once)
- Fire-and-forget delivery
- Message may be lost if network issues occur
- ⚠️ **Not recommended for important configuration**

### QoS 1 (At least once) ✅ **Recommended**
- Guaranteed delivery with acknowledgment
- Message will be retried if not acknowledged
- ✅ **Use for all control messages and configuration**

### QoS 2 (Exactly once)
- Highest overhead, rarely needed
- Not necessary for this application

## Retained Messages

Use the `-r` (retain) flag for **configuration messages** like baud rate:

```bash
# With retain - persists on broker
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"baud_rate":1200}' -q 1 -r
```

**Benefits**:
- Broker stores the message and delivers it to new subscribers
- ESP32 receives configuration on every connect (perfect for on-demand mode)
- Configuration survives ESP32 reboots

**Don't use retain for**:
- One-time commands like "start" or "stop"
- These should be delivered only once, not on every connect

## On-Demand Mode Behavior

In on-demand WiFi/MQTT mode, the ESP32:

1. **Disconnected by default** - No WiFi/MQTT connection while idle
2. **After MTU read** - Connects WiFi → Connects MQTT
3. **Subscribes to control topic** - Receives any retained messages
4. **Applies configuration** - e.g., changes baud rate if message received
5. **Publishes data** - Sends MTU reading to data topic
6. **Waits 5 seconds** - Listens for any queued downlink messages
7. **Disconnects** - Drops MQTT → Drops WiFi

This means:
- **Retained messages** are received on every publish cycle
- **Non-retained messages** sent while offline are only delivered if QoS 1+ (queued by broker)
- Configuration changes apply before the **next** MTU read

## Example Workflow

### 1. Set baud rate for your meter

```bash
# Set to 1200 bps (typical for Sensus meters)
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"baud_rate":1200}' -q 1 -r
```

This message is now stored by the broker and will be delivered every time the ESP32 connects.

### 2. Trigger a meter read

You can trigger reads in two ways:

**Option A**: Via MQTT (while ESP32 is connected)
```bash
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"command":"start","duration":30}' -q 1
```

**Option B**: Via serial CLI
```
ESP32 CLI> mtu_start 30
```

### 3. ESP32 publishes data

After completing the read, ESP32:
- Connects to WiFi/MQTT
- Receives baud rate configuration (1200 bps)
- Publishes meter data to `istorrs/mtu/data`
- Disconnects

### 4. Change baud rate

If you need to change the baud rate:

```bash
# Update retained message
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m '{"baud_rate":9600}' -q 1 -r
```

The new rate will be applied on the next connection (next meter read).

## Monitoring Messages

### Subscribe to Control Messages

See all control commands being sent:

```bash
mosquitto_sub -h test.mosquitto.org -t "istorrs/mtu/control" -v
```

### Subscribe to Meter Data

See all meter readings with device identification:

```bash
mosquitto_sub -h test.mosquitto.org -t "istorrs/mtu/data" -v
```

### Filter by Device

If you have multiple ESP32 devices, filter by chip ID using `jq`:

```bash
# Subscribe and filter for specific device
mosquitto_sub -h test.mosquitto.org -t "istorrs/mtu/data" | \
  jq 'select(.chip_id == "24:0a:c4:12:34:56")'

# Extract just the meter message
mosquitto_sub -h test.mosquitto.org -t "istorrs/mtu/data" | \
  jq -r '.message'

# Show device summary
mosquitto_sub -h test.mosquitto.org -t "istorrs/mtu/data" | \
  jq '{chip_id, ip: .wifi_ip, message: .message, baud: .baud_rate}'
```

### Send Control to Specific Device

Use device-specific topics for multi-device setups:

```bash
# Device-specific topic format: istorrs/mtu/{chip_id}/control
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/24:0a:c4:12:34:56/control" \
  -m '{"baud_rate":1200}' -q 1 -r
```

**Note**: Current implementation uses a single shared control topic. For production
multi-device deployments, modify the code to subscribe to device-specific topics
using the chip_id.

## Troubleshooting

**Baud rate not changing?**
- Check that MTU is stopped (can't change baud rate while running)
- Verify message uses QoS 1 and retain flag
- Check ESP32 logs for "MQTT: Setting baud rate to X bps"

**Commands not received?**
- Verify broker is reachable: `ping test.mosquitto.org`
- Check QoS level (use QoS 1)
- For on-demand mode: commands are only received during the 5s window after publishing data
- Use retained messages for persistent configuration

**JSON parsing errors?**
- Verify JSON is valid: `echo '{"baud_rate":1200}' | jq .`
- Check quotes are properly escaped in shell
- ESP32 logs will show "Unknown control command" if JSON is invalid
