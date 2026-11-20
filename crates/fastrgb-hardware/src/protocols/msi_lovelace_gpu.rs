//! MSI RTX 40-Series Lovelace GPU NVAPI I2C protocol (`MSI_GPU_Lovelace.js`, addr `0x68`).
//!
//! Specifically supports the `MSI RTX 4070 SUPER GAMING X SLIM` (`0x10DE:0x2783`,
//! subsys `0x1462:0xE134`) in per-LED `"GAMING X"` mode.

use std::sync::Arc;
use std::time::{Duration, Instant};

use fastrgb_core::Rgb;
use fastrgb_devices::{DeviceError, DeviceOutputSink, SampledLedFrame};

use crate::error::HardwareError;
use crate::nvapi::I2cBusTransport;

/// 7-bit I2C slave address for MSI Lovelace GPUs.
pub const MSI_GPU_I2C_ADDR: u8 = 0x68;

const REG_PER_LED_COLOR_BLOCK: u8 = 0x49;
const GAMING_X_LED_INDICES: [usize; 3] = [3, 4, 5];
const SINGLE_MODE_INTERVAL: Duration = Duration::from_millis(120);
const MAX_CHECKED_WRITE_ATTEMPTS: usize = 4;

/// Lighting control mode for MSI Lovelace GPUs (`MSI_GPU_Lovelace.js` `singleMode` setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MsiGpuControlMode {
    /// Hardware-timed single-zone static register updates (`0x22=0x13`, `0x30..0x32` RGB).
    ///
    /// Matches FastRGB's `singleMode = true` setting used on `MSI 4070 SUPER GAMING X SLIM MLG`
    /// and older VBIOS revisions where 136-byte `0x49` per-LED blocks are ignored.
    #[default]
    SingleZone,
    /// 136-byte per-LED software control (`0x49` BGR block write).
    PerLed,
}

impl MsiGpuControlMode {
    /// Toggles between `SingleZone` and `PerLed`.
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::SingleZone => Self::PerLed,
            Self::PerLed => Self::SingleZone,
        }
    }

    /// Short UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SingleZone => "SingleZone",
            Self::PerLed => "Per-LED",
        }
    }
}

/// Builds the 136-byte BGR payload for `"GAMING X"` per-LED mode (`MSI_GPU_Lovelace.js`).
#[must_use]
pub fn build_gaming_x_block(colors: &[Rgb]) -> [u8; 136] {
    let mut data = [0u8; 136];
    for (i, &led_slot) in GAMING_X_LED_INDICES.iter().enumerate() {
        let color = colors.get(i).copied().unwrap_or(Rgb::BLACK);
        let idx = led_slot * 4;
        // Primary zone indices (12, 16, 20)
        data[idx] = color.b;
        data[idx + 1] = color.g;
        data[idx + 2] = color.r;
        // Mirrored GAMING X zones (`iLedIdx - 12` -> 0, 4, 8)
        let mirror = idx - 12;
        data[mirror] = color.b;
        data[mirror + 1] = color.g;
        data[mirror + 2] = color.r;
    }
    data
}

/// MSI Lovelace GPU lighting sink writing over [`I2cBusTransport`] (`nvapi64.dll`).
pub struct MsiLovelaceGpuSink {
    bus: Arc<dyn I2cBusTransport>,
    addr_7bit: u8,
    mode: MsiGpuControlMode,
    initialized: bool,
    last_single_write: Option<Instant>,
    last_single_color: Option<Rgb>,
}

impl MsiLovelaceGpuSink {
    /// Creates a new [`MsiLovelaceGpuSink`] at `addr_7bit` (normally `0x68`) with `mode`.
    #[must_use]
    pub fn new(bus: Arc<dyn I2cBusTransport>, addr_7bit: u8, mode: MsiGpuControlMode) -> Self {
        Self {
            bus,
            addr_7bit,
            mode,
            initialized: false,
            last_single_write: None,
            last_single_color: None,
        }
    }

    /// Runs the Lovelace software control initialization sequence for the active [`MsiGpuControlMode`].
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError`] if any NVAPI I2C write fails.
    pub fn initialize(&mut self) -> Result<(), HardwareError> {
        let a = self.addr_7bit;
        match self.mode {
            MsiGpuControlMode::SingleZone => {
                self.checked_write(0x2E, 0x00)?;
                self.checked_write(0x2D, 0x00)?;
                self.checked_write(0x22, 0x13)?;
                self.checked_write(0x36, 0x64)?;
                self.checked_write(0x38, 0x00)?;
            }
            MsiGpuControlMode::PerLed => {
                self.bus.write_byte(a, 0x2D, 0x00)?;
                self.bus.write_byte(a, 0x46, 0x01)?;
                self.bus.write_byte(a, 0x22, 0x1C)?;

                self.bus.write_block(a, 0x29, &[0x00, 0x00, 0xFF])?;
                self.bus.write_block(a, 0x27, &[0x00, 0x00, 0xFF])?;
                self.bus.write_block(a, 0x28, &[0x00, 0x00, 0xFF])?;

                self.bus.write_byte(a, 0x36, 0x14)?;
                self.bus.write_byte(a, 0x38, 0x04)?;
                self.bus.write_byte(a, 0x22, 0x08)?;
                self.bus.write_byte(a, 0x3F, 0x00)?;
                self.bus.write_byte(a, 0x2D, 0x00)?;
                self.bus.write_byte(a, 0x46, 0x01)?;
                self.bus.write_byte(a, 0x22, 0x1C)?;

                self.bus.write_block(a, 0x29, &[0x00, 0x00, 0xFF])?;
                self.bus.write_block(a, 0x27, &[0x00, 0x00, 0xFF])?;
                self.bus.write_block(a, 0x28, &[0x00, 0x00, 0xFF])?;
                self.bus.write_byte(a, 0x2E, 0x01)?;
                self.bus.write_byte(a, 0x38, 0x04)?;
            }
        }

        self.initialized = true;
        Ok(())
    }

    fn checked_write(&self, reg: u8, val: u8) -> Result<(), HardwareError> {
        let mut last_err = None;
        for _ in 0..MAX_CHECKED_WRITE_ATTEMPTS {
            match self.bus.write_byte(self.addr_7bit, reg, val) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            HardwareError::Nvapi(format!("checked_write failed for reg 0x{reg:02X}"))
        }))
    }

    fn write_single_color(&mut self, color: Rgb) -> Result<(), HardwareError> {
        self.checked_write(0x22, 0x13)?;
        self.checked_write(0x30, color.r)?;
        self.checked_write(0x31, color.g)?;
        self.checked_write(0x32, color.b)?;
        self.last_single_write = Some(Instant::now());
        self.last_single_color = Some(color);
        Ok(())
    }
}

impl DeviceOutputSink for MsiLovelaceGpuSink {
    fn name(&self) -> &str {
        "msi-lovelace-nvapi"
    }

    fn flush(&mut self, frame: &SampledLedFrame) -> Result<(), DeviceError> {
        if frame.device_id != "msi_4070_super_gaming_x" {
            return Ok(());
        }
        if !self.initialized {
            self.initialize().map_err(|err| DeviceError::SinkFailure {
                sink: self.name().to_owned(),
                reason: err.to_string(),
            })?;
        }
        match self.mode {
            MsiGpuControlMode::SingleZone => {
                let color = frame.colors.first().copied().unwrap_or(Rgb::BLACK);
                if self.last_single_color == Some(color) {
                    return Ok(());
                }
                if let Some(prev) = self.last_single_write
                    && prev.elapsed() < SINGLE_MODE_INTERVAL
                {
                    return Ok(());
                }
                self.write_single_color(color)
                    .map_err(|err| DeviceError::SinkFailure {
                        sink: self.name().to_owned(),
                        reason: err.to_string(),
                    })
            }
            MsiGpuControlMode::PerLed => {
                let block = build_gaming_x_block(&frame.colors);
                self.bus
                    .write_block(self.addr_7bit, REG_PER_LED_COLOR_BLOCK, &block)
                    .map_err(|err| DeviceError::SinkFailure {
                        sink: self.name().to_owned(),
                        reason: err.to_string(),
                    })
            }
        }
    }

    fn shutdown(&mut self, shutdown_color: Rgb) -> Result<(), DeviceError> {
        match self.mode {
            MsiGpuControlMode::SingleZone => {
                let _ = self.write_single_color(shutdown_color);
            }
            MsiGpuControlMode::PerLed => {
                let block = build_gaming_x_block(&[shutdown_color; 3]);
                let _ = self
                    .bus
                    .write_block(self.addr_7bit, REG_PER_LED_COLOR_BLOCK, &block);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nvapi::DryRunI2cTransport;

    #[test]
    fn gaming_x_block_writes_bgr_and_mirrors_slots() {
        let colors = [
            Rgb::new(10, 20, 30),
            Rgb::new(40, 50, 60),
            Rgb::new(70, 80, 90),
        ];
        let block = build_gaming_x_block(&colors);
        assert_eq!(&block[12..15], &[30, 20, 10]);
        assert_eq!(&block[0..3], &[30, 20, 10]);
        assert_eq!(&block[16..19], &[60, 50, 40]);
        assert_eq!(&block[4..7], &[60, 50, 40]);
    }

    #[test]
    fn gpu_sink_per_led_initializes_and_writes_block_0x49() {
        let bus = Arc::new(DryRunI2cTransport::new());
        let mut sink =
            MsiLovelaceGpuSink::new(bus.clone(), MSI_GPU_I2C_ADDR, MsiGpuControlMode::PerLed);
        let frame = SampledLedFrame {
            device_id: "msi_4070_super_gaming_x".to_owned(),
            colors: vec![Rgb::new(255, 10, 20); 3],
        };
        sink.flush(&frame).unwrap();
        let writes = bus.writes();
        assert_eq!(writes.last().unwrap().reg, 0x49);
        assert_eq!(writes.last().unwrap().data.len(), 136);
    }

    #[test]
    fn gpu_sink_single_zone_writes_static_rgb_registers() {
        let bus = Arc::new(DryRunI2cTransport::new());
        let mut sink =
            MsiLovelaceGpuSink::new(bus.clone(), MSI_GPU_I2C_ADDR, MsiGpuControlMode::SingleZone);
        let frame = SampledLedFrame {
            device_id: "msi_4070_super_gaming_x".to_owned(),
            colors: vec![Rgb::new(0xAA, 0xBB, 0xCC); 3],
        };
        sink.flush(&frame).unwrap();
        let writes = bus.writes();
        // 5 init writes + 4 color update writes (`0x22`, `0x30`, `0x31`, `0x32`).
        assert_eq!(writes.len(), 9);
        assert_eq!(writes[0].reg, 0x2E);
        assert_eq!(writes[0].data, vec![0x00]);
        assert_eq!(writes[5].reg, 0x22);
        assert_eq!(writes[5].data, vec![0x13]);
        assert_eq!(writes[6].reg, 0x30);
        assert_eq!(writes[6].data, vec![0xAA]);
        assert_eq!(writes[7].reg, 0x31);
        assert_eq!(writes[7].data, vec![0xBB]);
        assert_eq!(writes[8].reg, 0x32);
        assert_eq!(writes[8].data, vec![0xCC]);
    }
}
