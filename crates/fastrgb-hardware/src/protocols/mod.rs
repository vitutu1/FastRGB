//! Hardware device protocol implementations built directly from FastRGB specifications.

pub mod asus_aura;
pub mod corsair_dominator;
pub mod msi_lovelace_gpu;

pub use asus_aura::{ASUS_AURA_PIDS, ASUS_VID, AsusAuraSink, RgbOrder};
pub use corsair_dominator::{CORSAIR_DOMINATOR_DEFAULT_ADDRS, CorsairDominatorSink};
pub use msi_lovelace_gpu::{MSI_GPU_I2C_ADDR, MsiGpuControlMode, MsiLovelaceGpuSink};
