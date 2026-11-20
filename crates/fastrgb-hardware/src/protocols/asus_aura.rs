//! ASUS Aura USB HID Motherboard & ARGB Channel Controller (`0x0B05:0x19AF`, `MI_02`).
//!
//! Drives both motherboard onboard zones and 5V ARGB headers (including fans wired as LED
//! strips) using 65-byte `0xEC` HID output reports.

use std::sync::Arc;

use fastrgb_core::Rgb;
use fastrgb_devices::{DeviceError, DeviceOutputSink, SampledLedFrame};

use crate::error::HardwareError;
use crate::hid::HidTransport;

/// ASUS USB Vendor ID.
pub const ASUS_VID: u16 = 0x0B05;
/// Supported ASUS Aura USB Motherboard Controller Product IDs.
pub const ASUS_AURA_PIDS: [u16; 3] = [0x18F3, 0x19AF, 0x1939];
/// Required USB HID interface index (`MI_02`).
pub const ASUS_AURA_INTERFACE: u8 = 2;

const REPORT_LEN: usize = 65;
const MAX_LEDS_PER_PACKET: usize = 20;
const MAX_LEDS_PER_CHANNEL: usize = 120;

const CMD_PREFIX: u8 = 0xEC;
const CMD_SET_MODE: u8 = 0x35;
const CMD_DIRECT_CONTROL: u8 = 0x40;
const CMD_DISABLE_GEN2: u8 = 0x52;
const CMD_FIRMWARE: u8 = 0x82;
const CMD_CONFIG: u8 = 0xB0;
const MODE_DIRECT: u8 = 0xFF;

/// Channel byte permutation matching ASUS Aura's `RGBconfig` parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RgbOrder {
    /// Standard Red-Green-Blue.
    #[default]
    Rgb,
    /// Green-Red-Blue (common on WS2812B / ARGB fans).
    Grb,
    /// Blue-Green-Red.
    Bgr,
    /// Red-Blue-Green.
    Rbg,
    /// Blue-Red-Green.
    Brg,
    /// Green-Blue-Red.
    Gbr,
}

impl RgbOrder {
    /// Permutes `color` into wire byte order `[b0, b1, b2]`.
    #[must_use]
    pub const fn encode(self, color: Rgb) -> [u8; 3] {
        match self {
            Self::Rgb => [color.r, color.g, color.b],
            Self::Grb => [color.g, color.r, color.b],
            Self::Bgr => [color.b, color.g, color.r],
            Self::Rbg => [color.r, color.b, color.g],
            Self::Brg => [color.b, color.r, color.g],
            Self::Gbr => [color.g, color.b, color.r],
        }
    }

    /// Cycles through common RGB orders (useful for quick UI fan calibration).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Rgb => Self::Grb,
            Self::Grb => Self::Bgr,
            Self::Bgr => Self::Rbg,
            Self::Rbg => Self::Brg,
            Self::Brg => Self::Gbr,
            Self::Gbr => Self::Rgb,
        }
    }

    /// Short display label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Grb => "GRB",
            Self::Bgr => "BGR",
            Self::Rbg => "RBG",
            Self::Brg => "BRG",
            Self::Gbr => "GBR",
        }
    }
}

/// Builds 65-byte `0xEC 0x40` Direct Control HID packets for an ASUS Aura ARGB channel.
#[must_use]
pub fn build_direct_packets(channel: u8, colors: &[Rgb], order: RgbOrder) -> Vec<[u8; REPORT_LEN]> {
    let total = colors.len().min(MAX_LEDS_PER_CHANNEL);
    if total == 0 {
        return Vec::new();
    }

    let mut packets = Vec::with_capacity(total.div_ceil(MAX_LEDS_PER_PACKET));
    let mut offset = 0usize;

    while offset < total {
        let count = (total - offset).min(MAX_LEDS_PER_PACKET);
        let is_last = offset + count >= total;

        let mut pkt = [0u8; REPORT_LEN];
        pkt[0] = CMD_PREFIX;
        pkt[1] = CMD_DIRECT_CONTROL;
        pkt[2] = if is_last { 0x80 | channel } else { channel };
        pkt[3] = offset as u8;
        pkt[4] = count as u8;

        for (i, &c) in colors[offset..offset + count].iter().enumerate() {
            let encoded = order.encode(c);
            let base = 5 + i * 3;
            pkt[base] = encoded[0];
            pkt[base + 1] = encoded[1];
            pkt[base + 2] = encoded[2];
        }

        packets.push(pkt);
        offset += count;
    }

    packets
}

/// Live or dry-run ASUS Aura Motherboard & ARGB Channel hardware output sink.
pub struct AsusAuraSink {
    transport: Arc<dyn HidTransport>,
    argb_order: RgbOrder,
    argb_channel_count: u8,
    firmware_model: Option<String>,
    initialized: bool,
}

impl AsusAuraSink {
    /// Creates a new [`AsusAuraSink`] wrapping `transport`.
    #[must_use]
    pub fn new(transport: Arc<dyn HidTransport>, argb_order: RgbOrder) -> Self {
        Self {
            transport,
            argb_order,
            argb_channel_count: 3,
            firmware_model: None,
            initialized: false,
        }
    }

    /// Queries firmware version, reads the config table, disables Gen2 mode, and puts all
    /// channels into direct software control mode.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError`] if HID communication fails.
    pub fn initialize(&mut self) -> Result<(), HardwareError> {
        let mut fw_pkt = [0u8; REPORT_LEN];
        fw_pkt[0] = CMD_PREFIX;
        fw_pkt[1] = CMD_FIRMWARE;
        self.transport.write_report(&fw_pkt)?;

        let mut resp = [0u8; REPORT_LEN];
        let n = self.transport.read_report(&mut resp, 50)?;
        if n >= 18 && resp[1] == 0x02 {
            let model = String::from_utf8_lossy(&resp[2..18])
                .trim_end_matches('\0')
                .trim()
                .to_owned();
            if !model.is_empty() {
                self.firmware_model = Some(model);
            }
        }

        let mut cfg_pkt = [0u8; REPORT_LEN];
        cfg_pkt[0] = CMD_PREFIX;
        cfg_pkt[1] = CMD_CONFIG;
        self.transport.write_report(&cfg_pkt)?;

        let mut cfg_resp = [0u8; REPORT_LEN];
        let cfg_n = self.transport.read_report(&mut cfg_resp, 50)?;
        if cfg_n >= 8 && cfg_resp[1] == 0x30 {
            let reported = cfg_resp[6];
            if reported > 0 && reported <= 4 {
                self.argb_channel_count = reported;
            }
        }

        // Disable Gen2 mode so FastRGB-style direct LED streaming applies cleanly.
        let mut gen2_pkt = [0u8; REPORT_LEN];
        gen2_pkt[0] = CMD_PREFIX;
        gen2_pkt[1] = CMD_DISABLE_GEN2;
        gen2_pkt[2] = 0x53;
        gen2_pkt[3] = 0x00;
        gen2_pkt[4] = 0x01;
        self.transport.write_report(&gen2_pkt)?;

        // Put mainboard (0) and each ARGB header channel (1..=N) into Direct mode (0xFF).
        for ch in 0..=self.argb_channel_count {
            let mut mode_pkt = [0u8; REPORT_LEN];
            mode_pkt[0] = CMD_PREFIX;
            mode_pkt[1] = CMD_SET_MODE;
            mode_pkt[2] = ch;
            mode_pkt[3] = 0x00;
            mode_pkt[4] = 0x00;
            mode_pkt[5] = MODE_DIRECT;
            self.transport.write_report(&mode_pkt)?;
        }

        self.initialized = true;
        Ok(())
    }

    /// Updates the RGB channel wire order (e.g. `RGB` vs `GRB` for ARGB fans).
    pub fn set_argb_order(&mut self, order: RgbOrder) {
        self.argb_order = order;
    }

    /// Returns the detected firmware model string, if queried.
    #[must_use]
    pub fn firmware_model(&self) -> Option<&str> {
        self.firmware_model.as_deref()
    }

    fn map_device_channel(device_id: &str) -> Option<u8> {
        match device_id {
            "asus_b850_argb_ch1" => Some(0),
            "asus_b850_argb_ch2" => Some(1),
            "asus_b850_argb_ch3" => Some(2),
            "asus_b850_mainboard" => Some(4),
            _ => None,
        }
    }
}

impl DeviceOutputSink for AsusAuraSink {
    fn name(&self) -> &str {
        "asus-aura-hid"
    }

    fn flush(&mut self, frame: &SampledLedFrame) -> Result<(), DeviceError> {
        let Some(channel) = Self::map_device_channel(&frame.device_id) else {
            return Ok(());
        };
        if !self.initialized {
            self.initialize().map_err(|err| DeviceError::SinkFailure {
                sink: self.name().to_owned(),
                reason: err.to_string(),
            })?;
        }
        let packets = build_direct_packets(channel, &frame.colors, self.argb_order);
        for pkt in packets {
            self.transport
                .write_report(&pkt)
                .map_err(|err| DeviceError::SinkFailure {
                    sink: self.name().to_owned(),
                    reason: err.to_string(),
                })?;
        }
        Ok(())
    }

    fn shutdown(&mut self, shutdown_color: Rgb) -> Result<(), DeviceError> {
        for ch in 0..self.argb_channel_count {
            let dark = vec![shutdown_color; 40];
            for pkt in build_direct_packets(ch, &dark, self.argb_order) {
                self.transport
                    .write_report(&pkt)
                    .map_err(|err| DeviceError::SinkFailure {
                        sink: self.name().to_owned(),
                        reason: err.to_string(),
                    })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::DryRunHidTransport;

    #[test]
    fn direct_packets_chunk_20_leds_and_set_apply_bit_on_last() {
        let colors = vec![Rgb::new(10, 20, 30); 25];
        let packets = build_direct_packets(1, &colors, RgbOrder::Grb);
        assert_eq!(packets.len(), 2);

        // First packet: channel=1 (no apply), start=0, count=20, GRB order
        assert_eq!(packets[0][0], 0xEC);
        assert_eq!(packets[0][1], 0x40);
        assert_eq!(packets[0][2], 0x01);
        assert_eq!(packets[0][3], 0);
        assert_eq!(packets[0][4], 20);
        assert_eq!(&packets[0][5..8], &[20, 10, 30]);

        // Second packet: channel=0x81 (apply bit set), start=20, count=5
        assert_eq!(packets[1][2], 0x81);
        assert_eq!(packets[1][3], 20);
        assert_eq!(packets[1][4], 5);
    }

    #[test]
    fn sink_initializes_and_flushes_channel_frames() {
        let transport = Arc::new(DryRunHidTransport::new());
        let mut fw_resp = vec![0u8; 65];
        fw_resp[1] = 0x02;
        fw_resp[2..18].copy_from_slice(b"AULA3-AR32-0309 ");
        transport.push_response(fw_resp);

        let mut cfg_resp = vec![0u8; 65];
        cfg_resp[1] = 0x30;
        cfg_resp[6] = 3;
        transport.push_response(cfg_resp);

        let mut sink = AsusAuraSink::new(transport.clone(), RgbOrder::Rgb);
        let frame = SampledLedFrame {
            device_id: "asus_b850_argb_ch2".to_owned(),
            colors: vec![Rgb::new(255, 0, 128); 12],
        };
        sink.flush(&frame).unwrap();

        assert_eq!(sink.firmware_model(), Some("AULA3-AR32-0309"));
        let sent = transport.sent_packets();
        // fw + cfg + disable_gen2 + 4x set_mode (0..=3) + 1 direct packet = 8 packets
        assert_eq!(sent.len(), 8);
        assert_eq!(sent.last().unwrap()[2], 0x81);
    }
}
