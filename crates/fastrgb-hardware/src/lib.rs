//! Real hardware discovery, HID/NVAPI/SMBus transports, and device protocol implementations.
//!
//! All hardware drivers implement [`fastrgb_devices::DeviceOutputSink`] and operate behind
//! transport traits so protocols can be unit-tested deterministically in dry-run mode before
//! touching physical hardware.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod discovery;
pub mod error;
pub mod hid;
pub mod nvapi;
pub mod protocols;
pub mod smbus;

pub use discovery::{
    HardwareDiscoveryReport, HardwareMode, MatchedHidDevice, discover_hardware,
    is_external_rgb_running,
};
pub use error::HardwareError;
pub use hid::{
    DryRunHidTransport, HidDeviceInfo, HidTransport, enumerate_hid_devices, open_hid_transport,
};
pub use nvapi::{DryRunI2cTransport, I2cBusTransport, NvapiDriver, NvidiaGpuInfo};
pub use protocols::{
    AsusAuraSink, CORSAIR_DOMINATOR_DEFAULT_ADDRS, CorsairDominatorSink, MSI_GPU_I2C_ADDR,
    MsiGpuControlMode, MsiLovelaceGpuSink, RgbOrder,
};
pub use smbus::{
    DryRunSmbusTransport, PIIX4_PRIMARY_BASE, PIIX4_SECONDARY_BASE, Piix4SmbusTransport, PortIo,
    SmbusTransport, open_live_smbus_transport, probe_corsair_dominator_addrs,
};
