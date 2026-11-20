//! Error types for `fastrgb-hardware`.

use thiserror::Error;

/// Errors returned by hardware discovery, HID/I2C/SMBus transports, or device protocols.
#[derive(Debug, Error)]
pub enum HardwareError {
    /// Failed to enumerate or open a USB HID device.
    #[error("HID error: {0}")]
    Hid(String),
    /// NVAPI DLL loading or I2C transaction failed.
    #[error("NVAPI I2C error: {0}")]
    Nvapi(String),
    /// Motherboard SMBus transaction failed or is unavailable in user mode.
    #[error("SMBus error: {0}")]
    Smbus(String),
    /// Device returned an unexpected protocol response.
    #[error("protocol error: {0}")]
    Protocol(String),
}
