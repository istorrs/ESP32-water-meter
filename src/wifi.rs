use anyhow::Result;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::info;
use std::net::Ipv4Addr;

// SAFETY: WifiManager wraps ESP-IDF WiFi which is thread-safe
unsafe impl Send for WifiManager {}
unsafe impl Sync for WifiManager {}

pub struct WifiManager {
    wifi: Box<BlockingWifi<EspWifi<'static>>>,
    default_ssid: heapless::String<32>,
    default_password: heapless::String<64>,
}

impl WifiManager {
    pub fn new(
        modem: Modem,
        sysloop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
        ssid: &str,
        password: &str,
    ) -> Result<Self> {
        info!("🌐 WiFi: Creating EspWifi instance...");
        let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
        info!("✅ WiFi: EspWifi created");

        let mut ssid_str = heapless::String::<32>::new();
        ssid_str
            .push_str(ssid)
            .map_err(|_| anyhow::anyhow!("SSID too long (max 32 chars)"))?;

        let mut password_str = heapless::String::<64>::new();
        password_str
            .push_str(password)
            .map_err(|_| anyhow::anyhow!("Password too long (max 64 chars)"))?;

        info!("🌐 WiFi: Configuring for SSID '{}'...", ssid);
        let wifi_configuration = Configuration::Client(ClientConfiguration {
            ssid: ssid_str.clone(),
            auth_method: AuthMethod::WPA2Personal,
            password: password_str.clone(),
            ..Default::default()
        });

        esp_wifi.set_configuration(&wifi_configuration)?;
        info!("✅ WiFi: Configuration set");

        info!("🌐 WiFi: Wrapping in BlockingWifi...");
        let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;
        info!("✅ WiFi: Wrapped");

        info!("🌐 WiFi: Starting...");
        wifi.start()?;
        info!("✅ WiFi: Started");

        info!("🌐 WiFi: Connecting to '{}'...", ssid);
        wifi.connect()?;
        info!("✅ WiFi: Connected");

        info!("🌐 WiFi: Waiting for network interface...");
        wifi.wait_netif_up()?;
        info!("✅ WiFi: Network interface up");

        let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
        info!("📡 WiFi: DHCP info: {:?}", ip_info);
        info!("🌐 WiFi: IP address: {}", ip_info.ip);

        Ok(Self {
            wifi: Box::new(wifi),
            default_ssid: ssid_str,
            default_password: password_str,
        })
    }

    pub fn reconnect(&mut self, ssid: Option<&str>, password: Option<&str>) -> Result<()> {
        info!("WiFi reconnect requested");

        // Use provided credentials or default
        let use_ssid = ssid.unwrap_or(self.default_ssid.as_str());
        let use_password = password.unwrap_or(self.default_password.as_str());

        let mut ssid_str = heapless::String::<32>::new();
        ssid_str
            .push_str(use_ssid)
            .map_err(|_| anyhow::anyhow!("SSID too long"))?;

        let mut password_str = heapless::String::<64>::new();
        password_str
            .push_str(use_password)
            .map_err(|_| anyhow::anyhow!("Password too long"))?;

        let wifi_configuration = Configuration::Client(ClientConfiguration {
            ssid: ssid_str,
            auth_method: AuthMethod::WPA2Personal,
            password: password_str,
            ..Default::default()
        });

        // Disconnect if currently connected
        if self.wifi.is_connected().unwrap_or(false) {
            info!("Disconnecting from current network...");
            let _ = self.wifi.disconnect();
        }

        self.wifi.set_configuration(&wifi_configuration)?;

        info!("Connecting to WiFi: {}", use_ssid);
        self.wifi.connect()?;
        info!("WiFi connected");

        self.wifi.wait_netif_up()?;

        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        info!("WiFi DHCP info: {:?}", ip_info);
        info!("WiFi IP: {}", ip_info.ip);

        Ok(())
    }

    pub fn is_connected(&self) -> Result<bool> {
        Ok(self.wifi.is_connected()?)
    }

    pub fn get_ip(&self) -> Result<Ipv4Addr> {
        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        Ok(ip_info.ip)
    }

    pub fn get_ssid(&self) -> Result<heapless::String<32>> {
        if let Configuration::Client(config) = self.wifi.get_configuration()? {
            Ok(config.ssid)
        } else {
            Ok(heapless::String::new())
        }
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if self.wifi.is_connected().unwrap_or(false) {
            info!("🔌 WiFi: Disconnecting...");
            self.wifi.disconnect()?;
            info!("✅ WiFi: Disconnected");
        }
        Ok(())
    }
}
