//! Corsair Dominator Platinum RGB RAM SMBus protocol (`CMT`, 12 LEDs per DIMM).
//!
//! Ported from `Corsair_Dominator_Ram.js` (`0x0C` DirectRGB command + PEC CRC-8 + `0x31`/`0x32`
//! 32+6 byte block writes across detected SMBus addresses such as `0x19` and `0x1B`).

use std::sync::Arc;

use fastrgb_core::Rgb;
use fastrgb_devices::{DeviceError, DeviceOutputSink, SampledLedFrame};

use crate::smbus::SmbusTransport;

/// Default SMBus slave addresses observed on dual-stick Corsair Dominator Platinum systems.
pub const CORSAIR_DOMINATOR_DEFAULT_ADDRS: [u8; 2] = [0x19, 0x1B];

/// Number of addressable LEDs per Corsair Dominator Platinum DIMM (`CMT`).
pub const DOMINATOR_LEDS_PER_STICK: usize = 12;

const REG_DIRECT_RGB: u8 = 0x31;
const CMD_DIRECT_RGB: u8 = 0x0C;

/// Official 256-byte SMBus PEC CRC-8 lookup table used by Corsair Dominator RAM.
#[rustfmt::skip]
const PEC_TABLE: [u8; 256] = [
    0, 7, 14, 9, 28, 27, 18, 21, 56, 63, 54, 49, 36, 35, 42, 45, 112,
    119, 126, 121, 108, 107, 98, 101, 72, 79, 70, 65, 84, 83, 90, 93,
    224, 231, 238, 233, 252, 251, 242, 245, 216, 223, 214, 209, 196,
    195, 202, 205, 144, 151, 158, 153, 140, 139, 130, 133, 168, 175,
    166, 161, 180, 179, 186, 189, 199, 192, 201, 206, 219, 220, 213,
    210, 255, 248, 241, 246, 227, 228, 237, 234, 183, 176, 185, 190,
    171, 172, 165, 162, 143, 136, 129, 134, 147, 148, 157, 154, 39, 32,
    41, 46, 59, 60, 53, 50, 31, 24, 17, 22, 3, 4, 13, 10, 87, 80, 89,
    94, 75, 76, 69, 66, 111, 104, 97, 102, 115, 116, 125, 122, 137, 142,
    135, 128, 149, 146, 155, 156, 177, 182, 191, 184, 173, 170, 163,
    164, 249, 254, 247, 240, 229, 226, 235, 236, 193, 198, 207, 200,
    221, 218, 211, 212, 105, 110, 103, 96, 117, 114, 123, 124, 81, 86,
    95, 88, 77, 74, 67, 68, 25, 30, 23, 16, 5, 2, 11, 12, 33, 38, 47,
    40, 61, 58, 51, 52, 78, 73, 64, 71, 82, 85, 92, 91, 118, 113, 120,
    127, 106, 109, 100, 99, 62, 57, 48, 55, 34, 37, 44, 43, 6, 1, 8, 15,
    26, 29, 20, 19, 174, 169, 160, 167, 178, 181, 188, 187, 150, 145,
    152, 159, 138, 141, 132, 131, 222, 217, 208, 215, 194, 197, 204,
    203, 230, 225, 232, 239, 250, 253, 244, 243,
];

/// Computes Corsair's SMBus CRC-8 over `data[0..37]`.
#[must_use]
pub fn calculate_pec_crc(data: &[u8; 37]) -> u8 {
    let mut crc = 0u8;
    for &byte in data {
        crc = PEC_TABLE[usize::from(crc ^ byte)];
    }
    crc
}

/// Builds the 38-byte Corsair Dominator DirectRGB frame split into `(reg_0x31_32b, reg_0x32_6b)`.
#[must_use]
pub fn build_stick_blocks(colors: &[Rgb]) -> ([u8; 32], [u8; 6]) {
    let mut payload = [0u8; 37];
    payload[0] = CMD_DIRECT_RGB;
    for (i, &c) in colors.iter().take(DOMINATOR_LEDS_PER_STICK).enumerate() {
        let off = 1 + i * 3;
        payload[off] = c.r;
        payload[off + 1] = c.g;
        payload[off + 2] = c.b;
    }
    let crc = calculate_pec_crc(&payload);

    let mut block1 = [0u8; 32];
    block1.copy_from_slice(&payload[..32]);

    let mut block2 = [0u8; 6];
    block2[..5].copy_from_slice(&payload[32..37]);
    block2[5] = crc;

    (block1, block2)
}

/// Corsair Dominator Platinum RGB output sink writing over [`SmbusTransport`].
pub struct CorsairDominatorSink {
    bus: Arc<dyn SmbusTransport>,
    addresses: Vec<u8>,
}

impl CorsairDominatorSink {
    /// Creates a new [`CorsairDominatorSink`] targeting SMBus `addresses` (e.g. `[0x19, 0x1B]`).
    #[must_use]
    pub fn new(bus: Arc<dyn SmbusTransport>, addresses: Vec<u8>) -> Self {
        Self { bus, addresses }
    }

    fn flush_single_stick(&self, addr: u8, slice: &[Rgb]) -> Result<(), DeviceError> {
        let (block1, block2) = build_stick_blocks(slice);
        self.write_with_retry(addr, REG_DIRECT_RGB, &block1)?;
        self.write_with_retry(addr, REG_DIRECT_RGB + 1, &block2)?;
        Ok(())
    }

    fn write_with_retry(&self, addr: u8, reg: u8, data: &[u8]) -> Result<(), DeviceError> {
        let mut last_err = None;
        for attempt in 0..2 {
            match self.bus.write_block(addr, reg, data) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    last_err = Some(err);
                    if attempt == 0 {
                        std::thread::sleep(std::time::Duration::from_micros(200));
                    }
                }
            }
        }
        Err(DeviceError::SinkFailure {
            sink: self.name().to_owned(),
            reason: last_err.map_or_else(|| "SMBus write failed".to_owned(), |e| e.to_string()),
        })
    }
}

impl DeviceOutputSink for CorsairDominatorSink {
    fn name(&self) -> &str {
        "corsair-dominator-smbus"
    }

    fn flush(&mut self, frame: &SampledLedFrame) -> Result<(), DeviceError> {
        // Individual 12-LED DIMM devices (`corsair_dominator_dimm1`, `corsair_dominator_dimm2`, ...)
        if let Some(suffix) = frame.device_id.strip_prefix("corsair_dominator_dimm")
            && let Ok(dimm_one_based) = suffix.parse::<usize>()
            && dimm_one_based >= 1
        {
            let stick_idx = dimm_one_based - 1;
            if let Some(&addr) = self.addresses.get(stick_idx) {
                self.flush_single_stick(addr, &frame.colors)?;
            }
            return Ok(());
        }

        // Backward-compatible combined 24-LED device (`corsair_dominator_platinum`)
        if frame.device_id != "corsair_dominator_platinum" {
            return Ok(());
        }

        let mut last_err = None;
        for (stick_idx, &addr) in self.addresses.iter().enumerate() {
            let start = stick_idx * DOMINATOR_LEDS_PER_STICK;
            let slice = if start < frame.colors.len() {
                let end = (start + DOMINATOR_LEDS_PER_STICK).min(frame.colors.len());
                &frame.colors[start..end]
            } else {
                &[]
            };
            if let Err(err) = self.flush_single_stick(addr, slice) {
                last_err = Some(err);
            }
        }
        if let Some(err) = last_err {
            return Err(err);
        }
        Ok(())
    }

    fn shutdown(&mut self, shutdown_color: Rgb) -> Result<(), DeviceError> {
        let colors = [shutdown_color; DOMINATOR_LEDS_PER_STICK];
        let (block1, block2) = build_stick_blocks(&colors);
        for &addr in &self.addresses {
            let _ = self.bus.write_block(addr, REG_DIRECT_RGB, &block1);
            let _ = self.bus.write_block(addr, REG_DIRECT_RGB + 1, &block2);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smbus::DryRunSmbusTransport;

    #[test]
    fn dominator_packets_include_direct_cmd_and_pec_crc() {
        let colors = vec![Rgb::new(0x11, 0x22, 0x33); 12];
        let (b1, b2) = build_stick_blocks(&colors);
        assert_eq!(b1[0], 0x0C);
        assert_eq!(&b1[1..4], &[0x11, 0x22, 0x33]);

        let mut check = [0u8; 37];
        check[..32].copy_from_slice(&b1);
        check[32..37].copy_from_slice(&b2[..5]);
        assert_eq!(b2[5], calculate_pec_crc(&check));
    }

    #[test]
    fn dominator_sink_splits_two_dimms_across_0x19_and_0x1b() {
        let bus = Arc::new(DryRunSmbusTransport::new());
        let mut sink = CorsairDominatorSink::new(bus.clone(), vec![0x19, 0x1B]);

        let mut colors = vec![Rgb::new(255, 0, 0); 12];
        colors.extend(vec![Rgb::new(0, 0, 255); 12]);

        let frame = SampledLedFrame {
            device_id: "corsair_dominator_platinum".to_owned(),
            colors,
        };
        sink.flush(&frame).unwrap();

        let writes = bus.writes();
        assert_eq!(writes.len(), 4);
        assert_eq!(writes[0].addr, 0x19);
        assert_eq!(writes[0].reg, 0x31);
        assert_eq!(writes[0].data.len(), 32);
        assert_eq!(writes[1].reg, 0x32);
        assert_eq!(writes[1].data.len(), 6);
        assert_eq!(writes[2].addr, 0x1B);
    }

    #[test]
    fn dominator_sink_routes_individual_12_led_dimm_frames() {
        let bus = Arc::new(DryRunSmbusTransport::new());
        let mut sink = CorsairDominatorSink::new(bus.clone(), vec![0x19, 0x1B]);

        let dimm1 = SampledLedFrame {
            device_id: "corsair_dominator_dimm1".to_owned(),
            colors: vec![Rgb::new(0, 255, 65); 12],
        };
        let dimm2 = SampledLedFrame {
            device_id: "corsair_dominator_dimm2".to_owned(),
            colors: vec![Rgb::new(0, 180, 255); 12],
        };
        sink.flush(&dimm1).unwrap();
        sink.flush(&dimm2).unwrap();

        let writes = bus.writes();
        assert_eq!(writes.len(), 4);
        assert_eq!(writes[0].addr, 0x19);
        assert_eq!(writes[2].addr, 0x1B);
    }
}
