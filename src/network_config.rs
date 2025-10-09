use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiConfig {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker_url: heapless::String<128>,
    pub client_id: heapless::String<32>,
    pub username: Option<heapless::String<32>>,
    pub password: Option<heapless::String<64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtuMqttTopics {
    pub readings: heapless::String<64>,
    pub status: heapless::String<64>,
}

impl Default for WifiConfig {
    fn default() -> Self {
        let mut ssid = heapless::String::new();
        let mut password = heapless::String::new();
        let _ = ssid.push_str("YOUR_SSID");
        let _ = password.push_str("YOUR_PASSWORD");

        Self { ssid, password }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        let mut broker_url = heapless::String::new();
        let mut client_id = heapless::String::new();
        let _ = broker_url.push_str("mqtt://broker.hivemq.com:1883");
        let _ = client_id.push_str("esp32-mtu");

        Self {
            broker_url,
            client_id,
            username: None,
            password: None,
        }
    }
}

impl Default for MtuMqttTopics {
    fn default() -> Self {
        let mut readings = heapless::String::new();
        let mut status = heapless::String::new();
        let _ = readings.push_str("watermeter/mtu/readings");
        let _ = status.push_str("watermeter/mtu/status");

        Self { readings, status }
    }
}
