use super::config::{MeterConfig, MeterType};
use heapless::String;
use std::sync::Mutex;

pub struct MeterHandler {
    config: Mutex<MeterConfig>,
}

impl MeterHandler {
    pub fn new(config: MeterConfig) -> Self {
        Self {
            config: Mutex::new(config),
        }
    }

    pub fn get_config(&self) -> MeterConfig {
        let config = self.config.lock().unwrap();
        config.clone()
    }

    pub fn set_type(&self, meter_type: MeterType) {
        let mut config = self.config.lock().unwrap();
        config.meter_type = meter_type;
        log::info!("Meter: Type set to {:?}", config.meter_type);
    }

    pub fn set_message(&self, message: String<256>) {
        let mut config = self.config.lock().unwrap();
        config.response_message = message;
        log::info!("Meter: Response message updated");
    }

    pub fn enable(&self) {
        let mut config = self.config.lock().unwrap();
        config.enabled = true;
        log::info!("Meter: Enabled");
    }

    pub fn disable(&self) {
        let mut config = self.config.lock().unwrap();
        config.enabled = false;
        log::info!("Meter: Disabled");
    }

    pub fn is_enabled(&self) -> bool {
        let config = self.config.lock().unwrap();
        config.enabled
    }

    /// Build UART frame with proper framing for meter type
    fn build_uart_frame(&self, byte: u8, meter_type: &MeterType) -> heapless::Vec<u8, 12> {
        let mut frame = heapless::Vec::new();

        // Start bit
        let _ = frame.push(0);

        // Data bits (LSB first) - only 7 bits for 7E1/7E2 framing
        let data_7bit = byte & 0x7F; // Mask to 7 bits
        for i in 0..7 {
            let bit = (data_7bit >> i) & 1;
            let _ = frame.push(bit);
        }

        // Parity and stop bits based on meter type
        match meter_type {
            MeterType::Sensus => {
                // 7E1: 7 data bits + even parity + 1 stop bit
                // Calculate even parity for the 7 data bits
                let parity = (data_7bit.count_ones() % 2) as u8;
                let _ = frame.push(parity);
                let _ = frame.push(1); // stop bit
            }
            MeterType::Neptune => {
                // 7E2: 7 data bits + even parity + 2 stop bits
                let parity = (data_7bit.count_ones() % 2) as u8;
                let _ = frame.push(parity);
                let _ = frame.push(1); // stop bit 1
                let _ = frame.push(1); // stop bit 2
            }
        }

        frame
    }

    /// Build complete response frame buffer for all characters in the message
    pub fn build_response_frames(&self) -> heapless::Vec<u8, 2048> {
        let config = self.config.lock().unwrap();
        let mut frame_buffer = heapless::Vec::new();

        // Build frames for each character in the response message
        for (char_index, ch) in config.response_message.chars().enumerate() {
            let char_frame = self.build_uart_frame(ch as u8, &config.meter_type);
            log::info!(
                "Meter: Building frame for char #{}: '{}' (ASCII {}) -> {} bits",
                char_index + 1,
                ch,
                ch as u8,
                char_frame.len()
            );
            for &bit in &char_frame {
                let _ = frame_buffer.push(bit);
            }
        }

        log::info!(
            "Meter: Complete frame buffer: {} total bits for {} characters",
            frame_buffer.len(),
            config.response_message.len()
        );
        frame_buffer
    }
}
