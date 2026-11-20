//! System hardware discovery (HID, NVAPI GPUs, and external RGB bus-contention detection).

use crate::hid::{HidDeviceInfo, enumerate_hid_devices};
use crate::nvapi::{NvapiDriver, NvidiaGpuInfo};
use crate::protocols::asus_aura::{ASUS_AURA_INTERFACE, ASUS_AURA_PIDS, ASUS_VID};

/// Output execution policy for hardware sinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HardwareMode {
    /// Discover devices and build protocol packets in memory only (safe dry-run mode).
    #[default]
    DryRun,
    /// Transmit live packets directly to physical HID / NVAPI / SMBus hardware.
    LiveHardware,
}

/// Known RGB-compatible HID device matched during system enumeration.
#[derive(Debug, Clone)]
pub struct MatchedHidDevice {
    /// Human-readable matched driver name.
    pub driver_name: String,
    /// Underlying HID endpoint info.
    pub info: HidDeviceInfo,
}

/// Full snapshot of system RGB hardware capabilities detected in user mode.
#[derive(Debug, Clone, Default)]
pub struct HardwareDiscoveryReport {
    /// All attached HID interfaces returned by SetupDi/HidD.
    pub all_hid: Vec<HidDeviceInfo>,
    /// Recognized RGB HID controllers (such as ASUS Aura `0x0B05:0x19AF`).
    pub matched_hid: Vec<MatchedHidDevice>,
    /// Physical NVIDIA GPUs detected via `nvapi64.dll`.
    pub nvidia_gpus: Vec<NvidiaGpuInfo>,
    /// True if a live SMBus transport (`inpoutx64.dll` -> AMD PIIX4 `0x0B00`) opened successfully.
    pub smbus_available: bool,
    /// True if another RGB controller process is running (potential HID/SMBus contention).
    pub external_rgb_running: bool,
    /// Diagnostic notes (e.g. NVAPI status or SMBus privilege notes).
    pub notes: Vec<String>,
}

/// Checks whether a conflicting external RGB controller process is currently running on Windows.
#[must_use]
pub fn is_external_rgb_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        };

        const CONFLICTING_PROCESSES: [&str; 4] = [
            "OpenRGB.exe",
            "iCUE.exe",
            "LightingService.exe",
            "ArmouryCrate.exe",
        ];

        // SAFETY: Snapshot of running processes released via `CloseHandle` before return.
        let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return false;
        };
        if snapshot.is_invalid() {
            return false;
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = false;

        // SAFETY: `entry.dwSize` is initialized and `snapshot` is valid.
        if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if CONFLICTING_PROCESSES
                    .iter()
                    .any(|p| exe.eq_ignore_ascii_case(p))
                {
                    found = true;
                    break;
                }
                // SAFETY: Iterating next process entry inside valid snapshot.
                if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                    break;
                }
            }
        }

        // SAFETY: `snapshot` is owned locally and closed once.
        unsafe {
            let _ = CloseHandle(snapshot);
        }
        found
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Scans Windows HID endpoints and `nvapi64.dll` GPUs without modifying any hardware state.
#[must_use]
pub fn discover_hardware() -> HardwareDiscoveryReport {
    let mut report = HardwareDiscoveryReport {
        external_rgb_running: is_external_rgb_running(),
        ..Default::default()
    };

    match enumerate_hid_devices() {
        Ok(list) => {
            for info in &list {
                if info.vendor_id == ASUS_VID
                    && ASUS_AURA_PIDS.contains(&info.product_id)
                    && info.interface == Some(ASUS_AURA_INTERFACE)
                {
                    report.matched_hid.push(MatchedHidDevice {
                        driver_name: "ASUS Aura Motherboard & ARGB Strips (0x0B05:0x19AF)"
                            .to_owned(),
                        info: info.clone(),
                    });
                }
            }
            report.all_hid = list;
        }
        Err(err) => {
            report.notes.push(format!("HID enumeration error: {err}"));
        }
    }

    match NvapiDriver::load().and_then(|drv| drv.enumerate_gpus()) {
        Ok(gpus) => {
            report.nvidia_gpus = gpus;
        }
        Err(err) => {
            report.notes.push(format!("NVAPI unavailable: {err}"));
        }
    }

    match crate::smbus::open_live_smbus_transport() {
        Ok((_, desc)) => {
            report.smbus_available = true;
            report.notes.push(format!(
                "Corsair Dominator Platinum (SMBus 0x19/0x1B): LIVE via {desc}"
            ));
        }
        Err(err) => {
            report.smbus_available = false;
            report.notes.push(format!(
                "Corsair Dominator Platinum (SMBus 0x19/0x1B on AMD PIIX4 0x0B00) in Dry-Run: {err}"
            ));
        }
    }

    report
}
