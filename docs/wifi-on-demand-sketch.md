# WiFi/MQTT On-Demand Implementation Sketch

## Workflow

```
1. Boot → Don't connect WiFi/MQTT
2. User runs mtu_start → Read completes
3. If read successful:
   a. Connect WiFi (~2-5s)
   b. Connect MQTT (~1-2s)
   c. Publish message
   d. Check for queued downlink messages (5s timeout)
   e. Disconnect MQTT
   f. Disconnect WiFi
4. Return to idle
```

## Code Changes

### main.rs
```rust
// At startup: DON'T connect WiFi/MQTT
let wifi = None; // Deferred
let mqtt = None; // Deferred

// Store credentials for later
const WIFI_SSID: &str = "...";
const WIFI_PASSWORD: &str = "...";
const MQTT_BROKER: &str = "...";
```

### After MTU read completion
```rust
fn publish_via_mqtt(message: &str) -> Result<()> {
    log::info!("🌐 Connecting WiFi for publish...");

    // Connect WiFi
    let wifi = WifiManager::new(...)?;
    log::info!("✅ WiFi connected");

    // Connect MQTT with clean_session=false to get queued messages
    let mqtt_config = MqttClientConfiguration {
        clean_session: false, // Important! Get queued messages
        ...
    };
    let mqtt = MqttClient::new(...)?;

    // Wait for MQTT connection
    while !mqtt.is_connected() && timeout < 10s {
        sleep(100ms);
    }

    // Publish message
    mqtt.publish(topic, message)?;
    log::info!("📤 Published message");

    // Wait for any queued downlink messages (5s timeout)
    log::info!("⏳ Waiting for queued messages...");
    sleep(5s);

    // Disconnect MQTT
    drop(mqtt);
    log::info("❌ MQTT disconnected");

    // Disconnect WiFi
    drop(wifi);
    log::info("❌ WiFi disconnected");

    Ok(())
}
```

## MQTT Broker Message Queuing

### What gets queued:
- **QoS 1 & 2 messages** published while you were offline
- **Retained messages** on subscribed topics
- Works if `clean_session = false` and same `client_id`

### What doesn't get queued:
- **QoS 0 messages** (fire and forget)
- Messages if `clean_session = true`

### Test it:
```bash
# While ESP32 is offline, publish a command:
mosquitto_pub -h test.mosquitto.org -t "istorrs/mtu/control" \
  -m "start" -q 1 -r

# When ESP32 connects, it will receive this message
```

## Timing Estimates

| Operation | Time |
|-----------|------|
| WiFi connect | 2-5s |
| MQTT connect | 0.5-2s |
| Publish | 100-500ms |
| Wait for downlink | 5s (configurable) |
| MQTT disconnect | 100ms |
| WiFi disconnect | 100ms |
| **Total** | **8-13s** |

## Power Savings

Assuming 1 read per minute:

### Always-on WiFi/MQTT:
- WiFi idle: ~80mA continuous
- MQTT idle: ~5mA continuous
- **Average: ~85mA**

### On-demand (10s connection per minute):
- Connected: 150mA for 10s
- Idle: 20mA for 50s
- **Average: ~42mA** (50% savings!)

### On-demand (1 read per hour):
- Connected: 150mA for 10s
- Idle: 20mA for 3590s
- **Average: ~20.4mA** (76% savings!)
