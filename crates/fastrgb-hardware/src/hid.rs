//! USB HID enumeration and Win32 transport layer.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::HardwareError;

/// Metadata describing an attached USB/HID interface on the host system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidDeviceInfo {
    /// OS device interface path used to open the handle.
    pub path: String,
    /// USB Vendor ID.
    pub vendor_id: u16,
    /// USB Product ID.
    pub product_id: u16,
    /// USB device release number.
    pub version_number: u16,
    /// HID Usage Page.
    pub usage_page: u16,
    /// HID Usage ID.
    pub usage: u16,
    /// Parsed USB interface index (`MI_xx`), if present in the path.
    pub interface: Option<u8>,
    /// Manufacturer string reported by the device.
    pub manufacturer: String,
    /// Product string reported by the device.
    pub product: String,
    /// Serial number string reported by the device.
    pub serial: String,
    /// Input report byte length (0 if none).
    pub input_report_len: u16,
    /// Output report byte length (0 if none).
    pub output_report_len: u16,
}

/// Abstract HID report transport used by HID protocol drivers.
pub trait HidTransport: Send + Sync {
    /// Writes a raw HID output report (first byte = Report ID, e.g. `0xEC`).
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Hid`] if the write fails.
    fn write_report(&self, packet: &[u8]) -> Result<(), HardwareError>;

    /// Reads a raw HID input report into `buf` with `timeout_ms` timeout.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Hid`] if the read fails or times out.
    fn read_report(&self, buf: &mut [u8], timeout_ms: u32) -> Result<usize, HardwareError>;
}

/// Deterministic in-memory HID transport for unit tests and dry-run inspection.
#[derive(Debug, Default)]
pub struct DryRunHidTransport {
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl DryRunHidTransport {
    /// Creates an empty [`DryRunHidTransport`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues a canned response to be returned on the next `read_report` call.
    pub fn push_response(&self, packet: Vec<u8>) {
        self.responses.lock().push(packet);
    }

    /// Returns a snapshot of all packets sent via `write_report`.
    #[must_use]
    pub fn sent_packets(&self) -> Vec<Vec<u8>> {
        self.sent.lock().clone()
    }
}

impl HidTransport for DryRunHidTransport {
    fn write_report(&self, packet: &[u8]) -> Result<(), HardwareError> {
        self.sent.lock().push(packet.to_vec());
        Ok(())
    }

    fn read_report(&self, buf: &mut [u8], _timeout_ms: u32) -> Result<usize, HardwareError> {
        let mut queue = self.responses.lock();
        if queue.is_empty() {
            return Ok(0);
        }
        let next = queue.remove(0);
        let len = next.len().min(buf.len());
        buf[..len].copy_from_slice(&next[..len]);
        Ok(len)
    }
}

/// Parses `mi_XX` USB interface number from a Windows HID device path.
#[must_use]
pub fn parse_mi_interface(path: &str) -> Option<u8> {
    let lower = path.to_ascii_lowercase();
    let idx = lower.find("mi_")?;
    let digits = lower.get(idx + 3..idx + 5)?;
    u8::from_str_radix(digits, 16).ok()
}

/// Enumerates all currently attached HID interfaces on the system.
///
/// # Errors
///
/// Returns [`HardwareError::Hid`] if Win32 `SetupDi` enumeration fails.
#[cfg(target_os = "windows")]
pub fn enumerate_hid_devices() -> Result<Vec<HidDeviceInfo>, HardwareError> {
    use std::ffi::c_void;
    use std::ptr;

    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList,
        SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    };
    use windows::Win32::Devices::HumanInterfaceDevice::{
        HIDD_ATTRIBUTES, HIDP_CAPS, HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetHidGuid,
        HidD_GetManufacturerString, HidD_GetPreparsedData, HidD_GetProductString,
        HidD_GetSerialNumberString, HidP_GetCaps, PHIDP_PREPARSED_DATA,
    };
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::core::PCWSTR;

    // SAFETY: `HidD_GetHidGuid` has no preconditions and returns the Windows HID interface GUID.
    let hid_guid = unsafe { HidD_GetHidGuid() };

    // SAFETY: `SetupDiGetClassDevsW` returns a device info set released below via
    // `SetupDiDestroyDeviceInfoList`.
    let dev_info = unsafe {
        SetupDiGetClassDevsW(
            Some(&hid_guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }
    .map_err(|err| HardwareError::Hid(format!("SetupDiGetClassDevsW failed: {err}")))?;

    let mut result = Vec::new();
    let mut index = 0u32;

    loop {
        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: `dev_info` is a valid HDEVINFO and `iface_data.cbSize` is initialized.
        let ok = unsafe {
            SetupDiEnumDeviceInterfaces(dev_info, None, &hid_guid, index, &mut iface_data)
        };
        if ok.is_err() {
            break;
        }
        index += 1;

        let mut required_size = 0u32;
        // SAFETY: First call queries required buffer byte length into `required_size`.
        let _ = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &iface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            )
        };
        if required_size == 0 {
            continue;
        }

        let mut detail_buf = vec![0u8; required_size as usize];
        let detail_ptr = detail_buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        // SAFETY: `detail_buf` has `required_size` bytes (>= size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>()).
        unsafe {
            (*detail_ptr).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }

        // SAFETY: `detail_ptr` points to a sufficiently sized buffer for `SetupDiGetDeviceInterfaceDetailW`.
        let got_detail = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &iface_data,
                Some(detail_ptr),
                required_size,
                None,
                None,
            )
        };
        if got_detail.is_err() {
            continue;
        }

        // SAFETY: `DevicePath` is a null-terminated UTF-16 string inside `detail_buf`.
        let path_wide_ptr = unsafe { ptr::addr_of!((*detail_ptr).DevicePath).cast::<u16>() };
        let path = read_wide_cstr(path_wide_ptr);
        let mut path_utf16: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

        // Open with `desired_access = 0` (metadata query only) so even devices exclusively opened
        // by FastRGB or Windows input drivers can still be enumerated safely.
        // SAFETY: `path_utf16` is null-terminated and handle is closed via `CloseHandle`.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(path_utf16.as_mut_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        let Ok(handle) = handle else {
            continue;
        };
        if handle.is_invalid() {
            continue;
        }

        let info = query_hid_metadata(handle, path);
        // SAFETY: `handle` is valid and non-invalid.
        unsafe {
            let _ = CloseHandle(handle);
        }
        if let Some(info) = info {
            result.push(info);
        }
    }

    // SAFETY: `dev_info` was created by `SetupDiGetClassDevsW`.
    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(dev_info);
    }

    fn read_wide_cstr(ptr: *const u16) -> String {
        let mut len = 0usize;
        // SAFETY: Null-terminated string within `detail_buf` (capped at 1024 wchars for safety).
        while len < 1024 && unsafe { *ptr.add(len) } != 0 {
            len += 1;
        }
        // SAFETY: `len` elements are valid UTF-16 units before the null terminator.
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        String::from_utf16_lossy(slice)
    }

    fn query_hid_string(
        handle: HANDLE,
        func: unsafe fn(HANDLE, *mut c_void, u32) -> bool,
    ) -> String {
        let mut buf = [0u16; 128];
        // SAFETY: `buf` is 256 bytes of valid writable memory.
        let ok = unsafe { func(handle, buf.as_mut_ptr().cast(), (buf.len() * 2) as u32) };
        if !ok {
            return String::new();
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end]).trim().to_owned()
    }

    fn query_hid_metadata(handle: HANDLE, path: String) -> Option<HidDeviceInfo> {
        let mut attrs = HIDD_ATTRIBUTES {
            Size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        // SAFETY: `handle` is an open HID device and `attrs.Size` is initialized.
        if !unsafe { HidD_GetAttributes(handle, &mut attrs) } {
            return None;
        }

        let mut caps = HIDP_CAPS::default();
        let mut preparsed = PHIDP_PREPARSED_DATA::default();
        // SAFETY: `HidD_GetPreparsedData` allocates preparsed data freed via `HidD_FreePreparsedData`.
        if unsafe { HidD_GetPreparsedData(handle, &mut preparsed) } {
            // SAFETY: `preparsed` was populated by `HidD_GetPreparsedData`.
            let _ = unsafe { HidP_GetCaps(preparsed, &mut caps) };
            // SAFETY: `preparsed` is freed exactly once.
            unsafe {
                let _ = HidD_FreePreparsedData(preparsed);
            }
        }

        let manufacturer = query_hid_string(handle, HidD_GetManufacturerString);
        let product = query_hid_string(handle, HidD_GetProductString);
        let serial = query_hid_string(handle, HidD_GetSerialNumberString);
        let interface = parse_mi_interface(&path);

        Some(HidDeviceInfo {
            path,
            vendor_id: attrs.VendorID,
            product_id: attrs.ProductID,
            version_number: attrs.VersionNumber,
            usage_page: caps.UsagePage,
            usage: caps.Usage,
            interface,
            manufacturer,
            product,
            serial,
            input_report_len: caps.InputReportByteLength,
            output_report_len: caps.OutputReportByteLength,
        })
    }

    Ok(result)
}

/// Non-Windows stub returning an empty HID list.
#[cfg(not(target_os = "windows"))]
pub fn enumerate_hid_devices() -> Result<Vec<HidDeviceInfo>, HardwareError> {
    Ok(Vec::new())
}

/// Opens a live Win32 HID read/write handle for `device_path`.
///
/// # Errors
///
/// Returns [`HardwareError::Hid`] if opening the device handle fails.
#[cfg(target_os = "windows")]
pub fn open_hid_transport(device_path: &str) -> Result<Box<dyn HidTransport>, HardwareError> {
    Ok(Box::new(Win32HidTransport::open(device_path)?))
}

/// Non-Windows stub for [`open_hid_transport`].
#[cfg(not(target_os = "windows"))]
pub fn open_hid_transport(_device_path: &str) -> Result<Box<dyn HidTransport>, HardwareError> {
    Err(HardwareError::Hid(
        "Win32 HID transport is only supported on Windows".to_owned(),
    ))
}

#[cfg(target_os = "windows")]
struct Win32HidTransport {
    handle: Mutex<windows::Win32::Foundation::HANDLE>,
}

#[cfg(target_os = "windows")]
// SAFETY: Win32 `HANDLE` for overlapped file I/O is protected by a `Mutex` and safe to send across threads.
unsafe impl Send for Win32HidTransport {}
#[cfg(target_os = "windows")]
// SAFETY: Win32 `HANDLE` access is serialized by `Mutex`.
unsafe impl Sync for Win32HidTransport {}

#[cfg(target_os = "windows")]
impl Win32HidTransport {
    fn open(path: &str) -> Result<Self, HardwareError> {
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows::core::PCWSTR;

        const GENERIC_READ_FLAG: u32 = 0x8000_0000;
        const GENERIC_WRITE_FLAG: u32 = 0x4000_0000;

        let mut wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is null-terminated; returned handle is owned by `Self` and closed in `Drop`.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_mut_ptr()),
                GENERIC_READ_FLAG | GENERIC_WRITE_FLAG,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .map_err(|err| HardwareError::Hid(format!("CreateFileW failed for '{path}': {err}")))?;

        if handle.is_invalid() {
            return Err(HardwareError::Hid(format!(
                "CreateFileW returned invalid handle for '{path}'"
            )));
        }

        Ok(Self {
            handle: Mutex::new(handle),
        })
    }
}

#[cfg(target_os = "windows")]
impl Drop for Win32HidTransport {
    fn drop(&mut self) {
        let handle = *self.handle.lock();
        if !handle.is_invalid() {
            // SAFETY: `handle` is owned by `Win32HidTransport` and closed once on drop.
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
        }
    }
}

#[cfg(target_os = "windows")]
impl HidTransport for Win32HidTransport {
    fn write_report(&self, packet: &[u8]) -> Result<(), HardwareError> {
        use windows::Win32::Foundation::{CloseHandle, ERROR_IO_PENDING, WAIT_OBJECT_0};
        use windows::Win32::Storage::FileSystem::WriteFile;
        use windows::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};
        use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

        let handle = *self.handle.lock();
        // SAFETY: Creating an auto-reset event closed below before return.
        let event = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|err| HardwareError::Hid(format!("CreateEventW failed: {err}")))?;

        let mut overlapped = OVERLAPPED {
            hEvent: event,
            ..Default::default()
        };

        // SAFETY: `packet` and `overlapped` remain alive until `GetOverlappedResult` completes or `CancelIo` runs.
        let res = unsafe { WriteFile(handle, Some(packet), None, Some(&mut overlapped)) };
        let result = match res {
            Ok(()) => Ok(()),
            Err(err) if err.code() == ERROR_IO_PENDING.into() => {
                // SAFETY: Waiting up to 200ms on our overlapped event.
                let wait = unsafe { WaitForSingleObject(event, 200) };
                if wait != WAIT_OBJECT_0 {
                    // SAFETY: Cancel pending I/O before `overlapped` goes out of scope.
                    unsafe {
                        let _ = CancelIo(handle);
                    }
                    Err(HardwareError::Hid("HID write timed out".to_owned()))
                } else {
                    let mut written = 0u32;
                    // SAFETY: Overlapped operation signaled completion.
                    unsafe { GetOverlappedResult(handle, &overlapped, &mut written, false) }
                        .map_err(|e| HardwareError::Hid(format!("GetOverlappedResult failed: {e}")))
                }
            }
            Err(err) => Err(HardwareError::Hid(format!("WriteFile failed: {err}"))),
        };

        // SAFETY: Event handle is closed after I/O completion or cancellation.
        unsafe {
            let _ = CloseHandle(event);
        }
        result
    }

    fn read_report(&self, buf: &mut [u8], timeout_ms: u32) -> Result<usize, HardwareError> {
        use windows::Win32::Foundation::{CloseHandle, ERROR_IO_PENDING, WAIT_OBJECT_0};
        use windows::Win32::Storage::FileSystem::ReadFile;
        use windows::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};
        use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

        let handle = *self.handle.lock();
        // SAFETY: Event handle closed before function returns.
        let event = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|err| HardwareError::Hid(format!("CreateEventW failed: {err}")))?;

        let mut overlapped = OVERLAPPED {
            hEvent: event,
            ..Default::default()
        };

        let mut read_bytes = 0u32;
        // SAFETY: `buf` and `overlapped` remain alive until completion or `CancelIo`.
        let res = unsafe {
            ReadFile(
                handle,
                Some(buf),
                Some(&mut read_bytes),
                Some(&mut overlapped),
            )
        };
        let result = match res {
            Ok(()) => Ok(read_bytes as usize),
            Err(err) if err.code() == ERROR_IO_PENDING.into() => {
                // SAFETY: Waiting up to `timeout_ms` for device input report.
                let wait = unsafe { WaitForSingleObject(event, timeout_ms) };
                if wait != WAIT_OBJECT_0 {
                    // SAFETY: Cancel pending read before stack buffers are dropped.
                    unsafe {
                        let _ = CancelIo(handle);
                        let mut dummy = 0u32;
                        let _ = GetOverlappedResult(handle, &overlapped, &mut dummy, true);
                    }
                    Ok(0)
                } else {
                    let mut transferred = 0u32;
                    // SAFETY: Signaled event guarantees completion.
                    unsafe { GetOverlappedResult(handle, &overlapped, &mut transferred, false) }
                        .map(|()| transferred as usize)
                        .map_err(|e| HardwareError::Hid(format!("GetOverlappedResult failed: {e}")))
                }
            }
            Err(err) => Err(HardwareError::Hid(format!("ReadFile failed: {err}"))),
        };

        // SAFETY: Event handle closed unconditionally.
        unsafe {
            let _ = CloseHandle(event);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mi_interface_from_windows_hid_path() {
        let p = r"\\?\hid#vid_0b05&pid_19af&mi_02#a&d29d7f0&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert_eq!(parse_mi_interface(p), Some(2));
    }
}
