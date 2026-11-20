//! NVIDIA NVAPI dynamic loader and GPU I2C bus transport (`nvapi64.dll`).

use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::HardwareError;

/// Metadata for a detected physical NVIDIA GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvidiaGpuInfo {
    /// Zero-based NVAPI GPU index.
    pub index: usize,
    /// Marketing name from `NvAPI_GPU_GetFullName`.
    pub name: String,
    /// PCI Vendor ID (e.g. `0x10DE`).
    pub vendor_id: u16,
    /// PCI Device ID (e.g. `0x2783` for RTX 4070 SUPER).
    pub device_id: u16,
    /// PCI Subsystem Vendor ID (e.g. `0x1462` for MSI).
    pub sub_vendor_id: u16,
    /// PCI Subsystem Device ID (e.g. `0xE134`).
    pub sub_device_id: u16,
}

/// Abstract GPU I2C bus transport used by GPU lighting drivers.
pub trait I2cBusTransport: Send + Sync {
    /// Reads a single byte from register `reg` on 7-bit I2C slave `addr_7bit`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Nvapi`] if the I2C transaction fails.
    fn read_byte(&self, addr_7bit: u8, reg: u8) -> Result<u8, HardwareError>;

    /// Writes a single byte `value` to register `reg` on 7-bit I2C slave `addr_7bit`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Nvapi`] if the I2C transaction fails.
    fn write_byte(&self, addr_7bit: u8, reg: u8, value: u8) -> Result<(), HardwareError>;

    /// Writes a contiguous `data` slice to register `reg` on 7-bit I2C slave `addr_7bit`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Nvapi`] if the I2C transaction fails.
    fn write_block(&self, addr_7bit: u8, reg: u8, data: &[u8]) -> Result<(), HardwareError>;
}

/// Recorded I2C write for dry-run verification and unit tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedI2cWrite {
    /// 7-bit I2C slave address.
    pub addr_7bit: u8,
    /// Register offset.
    pub reg: u8,
    /// Written payload bytes.
    pub data: Vec<u8>,
}

/// Deterministic in-memory I2C transport for unit testing GPU protocols.
#[derive(Debug, Default)]
pub struct DryRunI2cTransport {
    writes: Arc<Mutex<Vec<RecordedI2cWrite>>>,
}

impl DryRunI2cTransport {
    /// Creates a new [`DryRunI2cTransport`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a snapshot of all recorded I2C writes.
    #[must_use]
    pub fn writes(&self) -> Vec<RecordedI2cWrite> {
        self.writes.lock().clone()
    }
}

impl I2cBusTransport for DryRunI2cTransport {
    fn read_byte(&self, _addr_7bit: u8, _reg: u8) -> Result<u8, HardwareError> {
        Ok(0)
    }

    fn write_byte(&self, addr_7bit: u8, reg: u8, value: u8) -> Result<(), HardwareError> {
        self.writes.lock().push(RecordedI2cWrite {
            addr_7bit,
            reg,
            data: vec![value],
        });
        Ok(())
    }

    fn write_block(&self, addr_7bit: u8, reg: u8, data: &[u8]) -> Result<(), HardwareError> {
        self.writes.lock().push(RecordedI2cWrite {
            addr_7bit,
            reg,
            data: data.to_vec(),
        });
        Ok(())
    }
}

/// Dynamically loaded `nvapi64.dll` driver interface (user-mode, no admin required).
pub struct NvapiDriver {
    #[cfg(target_os = "windows")]
    inner: WindowsNvapi,
}

impl NvapiDriver {
    /// Attempts to load `nvapi64.dll` and initialize NVAPI.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Nvapi`] if `nvapi64.dll` is missing or initialization fails.
    pub fn load() -> Result<Self, HardwareError> {
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                inner: WindowsNvapi::load()?,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(HardwareError::Nvapi(
                "NVAPI is only supported on Windows".to_owned(),
            ))
        }
    }

    /// Enumerates physical NVIDIA GPUs and reads their PCI identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Nvapi`] if NVAPI enumeration fails.
    pub fn enumerate_gpus(&self) -> Result<Vec<NvidiaGpuInfo>, HardwareError> {
        #[cfg(target_os = "windows")]
        {
            self.inner.enumerate_gpus()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(Vec::new())
        }
    }

    /// Creates an [`I2cBusTransport`] targeting physical GPU `gpu_index`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Nvapi`] if `gpu_index` is invalid.
    pub fn open_i2c_bus(
        &self,
        gpu_index: usize,
    ) -> Result<Box<dyn I2cBusTransport>, HardwareError> {
        #[cfg(target_os = "windows")]
        {
            self.inner.open_i2c_bus(gpu_index)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = gpu_index;
            Err(HardwareError::Nvapi(
                "NVAPI is only supported on Windows".to_owned(),
            ))
        }
    }

    /// Diagnostic read-only probe across NVAPI `port_id` and `i2c_speed_khz` settings.
    #[must_use]
    pub fn probe_i2c_settings(&self, gpu_index: usize, addr_7bit: u8) -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            self.inner.probe_i2c_settings(gpu_index, addr_7bit)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (gpu_index, addr_7bit);
            Vec::new()
        }
    }
}

#[cfg(target_os = "windows")]
mod win_nvapi {
    use std::ffi::{CStr, c_char, c_void};
    use std::ptr;

    use windows::Win32::Foundation::{FreeLibrary, HMODULE};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::{PCSTR, PCWSTR};

    use super::{I2cBusTransport, NvidiaGpuInfo};
    use crate::error::HardwareError;

    const NVAPI_OK: i32 = 0;
    const NVAPI_MAX_PHYSICAL_GPUS: usize = 64;
    const NVAPI_SHORT_STRING_MAX: usize = 64;
    const NVAPI_I2C_SPEED_DEPRECATED: u32 = 0xFFFF;
    const NVAPI_I2C_SPEED_DEFAULT: u32 = 0;

    const ID_NVAPI_INITIALIZE: u32 = 0x0150_E828;
    const ID_NVAPI_ENUM_PHYSICAL_GPUS: u32 = 0xE5AC_921F;
    const ID_NVAPI_GPU_GET_PCI_IDENTIFIERS: u32 = 0x2DDF_B66E;
    const ID_NVAPI_GPU_GET_FULL_NAME: u32 = 0xCEEE_8E9F;
    const ID_NVAPI_I2C_READ_EX: u32 = 0x4D7B_0709;
    const ID_NVAPI_I2C_WRITE_EX: u32 = 0x283A_C65A;

    type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *const c_void;
    type InitializeFn = unsafe extern "C" fn() -> i32;
    type EnumPhysicalGpusFn = unsafe extern "C" fn(*mut *mut c_void, *mut u32) -> i32;
    type GetPciIdentifiersFn =
        unsafe extern "C" fn(*mut c_void, *mut u32, *mut u32, *mut u32, *mut u32) -> i32;
    type GetFullNameFn = unsafe extern "C" fn(*mut c_void, *mut c_char) -> i32;
    type I2cReadExFn = unsafe extern "C" fn(*mut c_void, *mut NvI2cInfoV3, *mut u32) -> i32;
    type I2cWriteExFn = unsafe extern "C" fn(*mut c_void, *mut NvI2cInfoV3, *mut u32) -> i32;

    #[repr(C)]
    struct NvI2cInfoV3 {
        version: u32,
        display_mask: u32,
        is_ddc_port: u8,
        i2c_dev_address: u8,
        reg_addr_ptr: *mut u8,
        reg_addr_size: u32,
        data_ptr: *mut u8,
        data_size: u32,
        i2c_speed: u32,
        i2c_speed_khz: u32,
        port_id: u8,
        is_port_id_set: u32,
    }

    pub(super) struct WindowsNvapi {
        module: HMODULE,
        enum_gpus: EnumPhysicalGpusFn,
        get_pci_ids: GetPciIdentifiersFn,
        get_full_name: GetFullNameFn,
        i2c_read_ex: I2cReadExFn,
        i2c_write_ex: I2cWriteExFn,
    }

    // SAFETY: Function pointers inside `nvapi64.dll` are process-global and thread-safe.
    unsafe impl Send for WindowsNvapi {}
    // SAFETY: Function pointers inside `nvapi64.dll` are process-global and thread-safe.
    unsafe impl Sync for WindowsNvapi {}

    impl Drop for WindowsNvapi {
        fn drop(&mut self) {
            if !self.module.is_invalid() {
                // SAFETY: `self.module` was returned by `LoadLibraryW`.
                unsafe {
                    let _ = FreeLibrary(self.module);
                }
            }
        }
    }

    impl WindowsNvapi {
        pub(super) fn load() -> Result<Self, HardwareError> {
            let mut dll_name: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
            // SAFETY: Null-terminated UTF-16 DLL name; `module` is freed on `Drop`.
            let module = unsafe { LoadLibraryW(PCWSTR(dll_name.as_mut_ptr())) }
                .map_err(|err| HardwareError::Nvapi(format!("LoadLibraryW(nvapi64.dll): {err}")))?;

            // SAFETY: Querying standard exported `nvapi_QueryInterface` symbol.
            let qi_addr = unsafe {
                GetProcAddress(module, PCSTR(c"nvapi_QueryInterface".as_ptr().cast::<u8>()))
            }
            .ok_or_else(|| {
                HardwareError::Nvapi("nvapi_QueryInterface symbol not found".to_owned())
            })?;

            // SAFETY: `nvapi_QueryInterface` has signature `extern "C" fn(u32) -> *const c_void`.
            let query_interface: QueryInterfaceFn = unsafe { std::mem::transmute(qi_addr) };

            let init: InitializeFn =
                resolve_fn(query_interface, ID_NVAPI_INITIALIZE, "NvAPI_Initialize")?;
            let enum_gpus: EnumPhysicalGpusFn = resolve_fn(
                query_interface,
                ID_NVAPI_ENUM_PHYSICAL_GPUS,
                "NvAPI_EnumPhysicalGPUs",
            )?;
            let get_pci_ids: GetPciIdentifiersFn = resolve_fn(
                query_interface,
                ID_NVAPI_GPU_GET_PCI_IDENTIFIERS,
                "NvAPI_GPU_GetPCIIdentifiers",
            )?;
            let get_full_name: GetFullNameFn = resolve_fn(
                query_interface,
                ID_NVAPI_GPU_GET_FULL_NAME,
                "NvAPI_GPU_GetFullName",
            )?;
            let i2c_read_ex: I2cReadExFn =
                resolve_fn(query_interface, ID_NVAPI_I2C_READ_EX, "NvAPI_I2CReadEx")?;
            let i2c_write_ex: I2cWriteExFn =
                resolve_fn(query_interface, ID_NVAPI_I2C_WRITE_EX, "NvAPI_I2CWriteEx")?;

            // SAFETY: `NvAPI_Initialize` has no arguments and initializes the NVAPI runtime.
            let status = unsafe { init() };
            if status != NVAPI_OK {
                return Err(HardwareError::Nvapi(format!(
                    "NvAPI_Initialize failed with status {status}"
                )));
            }

            Ok(Self {
                module,
                enum_gpus,
                get_pci_ids,
                get_full_name,
                i2c_read_ex,
                i2c_write_ex,
            })
        }

        fn raw_gpu_handles(&self) -> Result<Vec<*mut c_void>, HardwareError> {
            let mut handles = [ptr::null_mut(); NVAPI_MAX_PHYSICAL_GPUS];
            let mut count = 0u32;
            // SAFETY: `handles` has capacity for `NVAPI_MAX_PHYSICAL_GPUS` opaque pointers.
            let status = unsafe { (self.enum_gpus)(handles.as_mut_ptr(), &mut count) };
            if status != NVAPI_OK {
                return Err(HardwareError::Nvapi(format!(
                    "NvAPI_EnumPhysicalGPUs failed with status {status}"
                )));
            }
            let n = (count as usize).min(NVAPI_MAX_PHYSICAL_GPUS);
            Ok(handles[..n].to_vec())
        }

        pub(super) fn enumerate_gpus(&self) -> Result<Vec<NvidiaGpuInfo>, HardwareError> {
            let handles = self.raw_gpu_handles()?;
            let mut out = Vec::with_capacity(handles.len());

            for (index, handle) in handles.into_iter().enumerate() {
                let mut dev_id = 0u32;
                let mut subsys_id = 0u32;
                let mut _rev_id = 0u32;
                let mut _ext_dev_id = 0u32;

                // SAFETY: `handle` came from `NvAPI_EnumPhysicalGPUs` and output pointers are valid.
                let pci_status = unsafe {
                    (self.get_pci_ids)(
                        handle,
                        &mut dev_id,
                        &mut subsys_id,
                        &mut _rev_id,
                        &mut _ext_dev_id,
                    )
                };
                if pci_status != NVAPI_OK {
                    continue;
                }

                let mut name_buf = [0 as c_char; NVAPI_SHORT_STRING_MAX];
                // SAFETY: `name_buf` has `NVAPI_SHORT_STRING_MAX` bytes as required by NVAPI.
                let name_status = unsafe { (self.get_full_name)(handle, name_buf.as_mut_ptr()) };
                let name = if name_status == NVAPI_OK {
                    // SAFETY: NVAPI guarantees null-termination on success inside `name_buf`.
                    unsafe { CStr::from_ptr(name_buf.as_ptr()) }
                        .to_string_lossy()
                        .into_owned()
                } else {
                    format!("NVIDIA GPU #{index}")
                };

                // NVAPI packs `dev_id = (device_id << 16) | vendor_id` and `subsys_id = (sub_device_id << 16) | sub_vendor_id`.
                let vendor_id = (dev_id & 0xFFFF) as u16;
                let device_id = ((dev_id >> 16) & 0xFFFF) as u16;
                let sub_vendor_id = (subsys_id & 0xFFFF) as u16;
                let sub_device_id = ((subsys_id >> 16) & 0xFFFF) as u16;

                out.push(NvidiaGpuInfo {
                    index,
                    name,
                    vendor_id,
                    device_id,
                    sub_vendor_id,
                    sub_device_id,
                });
            }
            Ok(out)
        }

        pub(super) fn open_i2c_bus(
            &self,
            gpu_index: usize,
        ) -> Result<Box<dyn I2cBusTransport>, HardwareError> {
            let handles = self.raw_gpu_handles()?;
            let gpu_handle = *handles
                .get(gpu_index)
                .ok_or_else(|| HardwareError::Nvapi(format!("GPU index {gpu_index} not found")))?;

            // Keep an independent `LoadLibraryW` reference for the bus lifetime.
            let bus_owner = Self::load()?;
            Ok(Box::new(NvapiGpuI2cBus {
                _owner: bus_owner,
                gpu_handle,
                i2c_read_ex: self.i2c_read_ex,
                i2c_write_ex: self.i2c_write_ex,
            }))
        }

        pub(super) fn probe_i2c_settings(&self, gpu_index: usize, addr_7bit: u8) -> Vec<String> {
            let Ok(handles) = self.raw_gpu_handles() else {
                return vec!["raw_gpu_handles failed".to_owned()];
            };
            let Some(&gpu_handle) = handles.get(gpu_index) else {
                return vec![format!("GPU {gpu_index} missing")];
            };
            let mut out = Vec::new();
            let struct_size = std::mem::size_of::<NvI2cInfoV3>() as u32;
            for (port_id, is_port_set) in [(0u8, 1u32), (1, 1), (2, 1), (0, 0)] {
                for speed_khz in [0u32, 4u32] {
                    let mut vals = Vec::new();
                    let mut err_status = None;
                    for reg in [0x00u8, 0x01, 0x22, 0x2D, 0x2E, 0x36, 0x46] {
                        let mut reg_buf = [reg];
                        let mut val = [0u8];
                        let mut info = NvI2cInfoV3 {
                            version: struct_size | (3 << 16),
                            display_mask: 0,
                            is_ddc_port: 0,
                            i2c_dev_address: addr_7bit << 1,
                            reg_addr_ptr: reg_buf.as_mut_ptr(),
                            reg_addr_size: 1,
                            data_ptr: val.as_mut_ptr(),
                            data_size: 1,
                            i2c_speed: NVAPI_I2C_SPEED_DEPRECATED,
                            i2c_speed_khz: speed_khz,
                            port_id,
                            is_port_id_set: is_port_set,
                        };
                        let mut unknown = 0u32;
                        // SAFETY: synchronous read-only NVAPI I2C call with stack buffers.
                        let status =
                            unsafe { (self.i2c_read_ex)(gpu_handle, &mut info, &mut unknown) };
                        if status == NVAPI_OK {
                            vals.push(format!("{reg:02X}={:02X}", val[0]));
                        } else {
                            err_status = Some(status);
                            break;
                        }
                    }
                    match err_status {
                        None => out.push(format!(
                            "port={port_id} set={is_port_set} khz={speed_khz} -> OK [{}]",
                            vals.join(" ")
                        )),
                        Some(st) => out.push(format!(
                            "port={port_id} set={is_port_set} khz={speed_khz} -> ERR status={st}"
                        )),
                    }
                }
            }
            out
        }
    }

    struct NvapiGpuI2cBus {
        _owner: WindowsNvapi,
        gpu_handle: *mut c_void,
        i2c_read_ex: I2cReadExFn,
        i2c_write_ex: I2cWriteExFn,
    }

    // SAFETY: `gpu_handle` is a process-wide physical GPU handle protected via NVAPI driver synchronization.
    unsafe impl Send for NvapiGpuI2cBus {}
    // SAFETY: `gpu_handle` is safe to pass across threads into `NvAPI_I2CWriteEx`.
    unsafe impl Sync for NvapiGpuI2cBus {}

    impl I2cBusTransport for NvapiGpuI2cBus {
        fn read_byte(&self, addr_7bit: u8, reg: u8) -> Result<u8, HardwareError> {
            let mut reg_buf = [reg];
            let mut value = [0u8];
            let struct_size = std::mem::size_of::<NvI2cInfoV3>() as u32;
            let mut info = NvI2cInfoV3 {
                version: struct_size | (3 << 16),
                display_mask: 0,
                is_ddc_port: 0,
                i2c_dev_address: addr_7bit << 1,
                reg_addr_ptr: reg_buf.as_mut_ptr(),
                reg_addr_size: 1,
                data_ptr: value.as_mut_ptr(),
                data_size: 1,
                i2c_speed: NVAPI_I2C_SPEED_DEPRECATED,
                i2c_speed_khz: NVAPI_I2C_SPEED_DEFAULT,
                port_id: 1,
                is_port_id_set: 1,
            };
            let mut unknown = 0u32;
            // SAFETY: `info` pointers (`reg_buf`, `value`) remain valid for the duration of the synchronous call.
            let status = unsafe { (self.i2c_read_ex)(self.gpu_handle, &mut info, &mut unknown) };
            if status != NVAPI_OK {
                return Err(HardwareError::Nvapi(format!(
                    "NvAPI_I2CReadEx failed (addr=0x{addr_7bit:02X}, reg=0x{reg:02X}, status={status})"
                )));
            }
            Ok(value[0])
        }

        fn write_byte(&self, addr_7bit: u8, reg: u8, value: u8) -> Result<(), HardwareError> {
            self.write_block(addr_7bit, reg, &[value])
        }

        fn write_block(&self, addr_7bit: u8, reg: u8, data: &[u8]) -> Result<(), HardwareError> {
            let mut reg_buf = [reg];
            let mut payload = data.to_vec();
            let struct_size = std::mem::size_of::<NvI2cInfoV3>() as u32;
            let mut info = NvI2cInfoV3 {
                version: struct_size | (3 << 16),
                display_mask: 0,
                is_ddc_port: 0,
                i2c_dev_address: addr_7bit << 1,
                reg_addr_ptr: reg_buf.as_mut_ptr(),
                reg_addr_size: 1,
                data_ptr: payload.as_mut_ptr(),
                data_size: payload.len() as u32,
                i2c_speed: NVAPI_I2C_SPEED_DEPRECATED,
                i2c_speed_khz: NVAPI_I2C_SPEED_DEFAULT,
                port_id: 1,
                is_port_id_set: 1,
            };
            let mut unknown = 0u32;
            // SAFETY: `info` pointers (`reg_buf`, `payload`) remain valid for the duration of the synchronous call.
            let status = unsafe { (self.i2c_write_ex)(self.gpu_handle, &mut info, &mut unknown) };
            if status != NVAPI_OK {
                return Err(HardwareError::Nvapi(format!(
                    "NvAPI_I2CWriteEx failed (addr=0x{addr_7bit:02X}, reg=0x{reg:02X}, status={status})"
                )));
            }
            Ok(())
        }
    }

    fn resolve_fn<T: Copy>(qi: QueryInterfaceFn, id: u32, name: &str) -> Result<T, HardwareError> {
        // SAFETY: `qi` is `nvapi_QueryInterface` and `id` is a valid NVAPI function ID.
        let ptr = unsafe { qi(id) };
        if ptr.is_null() {
            return Err(HardwareError::Nvapi(format!(
                "nvapi_QueryInterface returned null for {name} (0x{id:08X})"
            )));
        }
        // SAFETY: Caller specifies matching `extern "C"` function pointer type `T`.
        Ok(unsafe { std::mem::transmute_copy(&ptr) })
    }
}

#[cfg(target_os = "windows")]
use win_nvapi::WindowsNvapi;
