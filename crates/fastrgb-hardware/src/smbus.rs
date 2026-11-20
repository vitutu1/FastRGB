//! Motherboard SMBus transport abstraction, AMD PIIX4 controller, and Windows `inpoutx64` loader.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::HardwareError;

/// Default AMD FCH / PIIX4 SMBus controller base I/O port (`PCI\VEN_1022&DEV_790B`).
pub const PIIX4_PRIMARY_BASE: u16 = 0x0B00;
/// Secondary AMD FCH / PIIX4 SMBus base I/O port.
pub const PIIX4_SECONDARY_BASE: u16 = 0x0B20;

const SMBHSTSTS_OFF: u16 = 0x00;
const SMBHSTCNT_OFF: u16 = 0x02;
const SMBHSTCMD_OFF: u16 = 0x03;
const SMBHSTADD_OFF: u16 = 0x04;
const SMBHSTDAT0_OFF: u16 = 0x05;
const SMBBLKDAT_OFF: u16 = 0x07;

const STS_HOST_BUSY: u8 = 0x01;
const STS_INTR: u8 = 0x02;
const STS_DEV_ERR: u8 = 0x04;
const STS_BUS_ERR: u8 = 0x08;
const STS_FAILED: u8 = 0x10;
const STS_ERRORS: u8 = STS_DEV_ERR | STS_BUS_ERR | STS_FAILED;
const STS_CLEAR_MASK: u8 = STS_INTR | STS_ERRORS;

const CNT_BYTE_DATA: u8 = 0x08;
const CNT_BLOCK_DATA: u8 = 0x14;
const CNT_START: u8 = 0x40;

const MAX_POLL_ITERS: usize = 5_000;

/// Abstract x86 I/O port reader/writer decoupled from kernel driver FFI for testability.
pub trait PortIo: Send + Sync {
    /// Reads one byte from x86 I/O port `port`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Smbus`] on kernel driver failure.
    fn read_u8(&self, port: u16) -> Result<u8, HardwareError>;

    /// Writes one byte `value` to x86 I/O port `port`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Smbus`] on kernel driver failure.
    fn write_u8(&self, port: u16, value: u8) -> Result<(), HardwareError>;
}

/// Abstract SMBus/I2C controller interface (e.g. AMD PIIX4 `0x0B00`).
pub trait SmbusTransport: Send + Sync {
    /// Writes a single byte to `reg` on 7-bit SMBus address `addr`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Smbus`] on bus failure.
    fn write_byte(&self, addr: u8, reg: u8, value: u8) -> Result<(), HardwareError>;

    /// Reads a single byte from `reg` on 7-bit SMBus address `addr`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Smbus`] on bus failure.
    fn read_byte(&self, addr: u8, reg: u8) -> Result<u8, HardwareError>;

    /// Writes an SMBus block (`<= 32` bytes) to `reg` on 7-bit SMBus address `addr`.
    ///
    /// # Errors
    ///
    /// Returns [`HardwareError::Smbus`] on bus failure.
    fn write_block(&self, addr: u8, reg: u8, data: &[u8]) -> Result<(), HardwareError>;
}

/// AMD PIIX4 SMBus host controller (`0x0B00` / `0x0B20`) driven via [`PortIo`].
pub struct Piix4SmbusTransport {
    io: Arc<dyn PortIo>,
    base_port: u16,
    lock: Mutex<()>,
}

impl std::fmt::Debug for Piix4SmbusTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Piix4SmbusTransport")
            .field("base_port", &format_args!("0x{:04X}", self.base_port))
            .finish()
    }
}

impl Piix4SmbusTransport {
    /// Creates an AMD PIIX4 SMBus transport bound to `base_port` (typically [`PIIX4_PRIMARY_BASE`]).
    #[must_use]
    pub fn new(io: Arc<dyn PortIo>, base_port: u16) -> Self {
        Self {
            io,
            base_port,
            lock: Mutex::new(()),
        }
    }

    /// Configured PCI SMBus I/O base address.
    #[must_use]
    pub const fn base_port(&self) -> u16 {
        self.base_port
    }

    fn wait_ready_and_clear(&self) -> Result<(), HardwareError> {
        for _ in 0..MAX_POLL_ITERS {
            let sts = self.io.read_u8(self.base_port + SMBHSTSTS_OFF)?;
            if (sts & STS_HOST_BUSY) == 0 {
                if (sts & STS_CLEAR_MASK) != 0 {
                    self.io
                        .write_u8(self.base_port + SMBHSTSTS_OFF, STS_CLEAR_MASK)?;
                }
                return Ok(());
            }
            std::hint::spin_loop();
        }
        Err(HardwareError::Smbus(format!(
            "PIIX4 0x{:04X} timed out waiting for HOST_BUSY clear",
            self.base_port
        )))
    }

    fn wait_completion(&self, addr: u8, reg: u8) -> Result<(), HardwareError> {
        for _ in 0..MAX_POLL_ITERS {
            let sts = self.io.read_u8(self.base_port + SMBHSTSTS_OFF)?;
            if (sts & STS_HOST_BUSY) == 0 {
                let _ = self
                    .io
                    .write_u8(self.base_port + SMBHSTSTS_OFF, STS_CLEAR_MASK);
                if (sts & STS_ERRORS) != 0 {
                    return Err(HardwareError::Smbus(format!(
                        "PIIX4 0x{:04X} error sts=0x{sts:02X} on addr=0x{addr:02X} reg=0x{reg:02X}",
                        self.base_port
                    )));
                }
                return Ok(());
            }
            std::hint::spin_loop();
        }
        Err(HardwareError::Smbus(format!(
            "PIIX4 0x{:04X} transfer timeout on addr=0x{addr:02X} reg=0x{reg:02X}",
            self.base_port
        )))
    }
}

impl SmbusTransport for Piix4SmbusTransport {
    fn write_byte(&self, addr: u8, reg: u8, value: u8) -> Result<(), HardwareError> {
        let _guard = self.lock.lock();
        let _os_guard = OsSmbusMutexGuard::acquire();
        self.wait_ready_and_clear()?;

        self.io
            .write_u8(self.base_port + SMBHSTADD_OFF, addr << 1)?;
        self.io.write_u8(self.base_port + SMBHSTCMD_OFF, reg)?;
        self.io.write_u8(self.base_port + SMBHSTDAT0_OFF, value)?;
        self.io
            .write_u8(self.base_port + SMBHSTCNT_OFF, CNT_BYTE_DATA | CNT_START)?;

        self.wait_completion(addr, reg)
    }

    fn read_byte(&self, addr: u8, reg: u8) -> Result<u8, HardwareError> {
        let _guard = self.lock.lock();
        let _os_guard = OsSmbusMutexGuard::acquire();
        self.wait_ready_and_clear()?;

        self.io
            .write_u8(self.base_port + SMBHSTADD_OFF, (addr << 1) | 0x01)?;
        self.io.write_u8(self.base_port + SMBHSTCMD_OFF, reg)?;
        self.io
            .write_u8(self.base_port + SMBHSTCNT_OFF, CNT_BYTE_DATA | CNT_START)?;

        self.wait_completion(addr, reg)?;
        self.io.read_u8(self.base_port + SMBHSTDAT0_OFF)
    }

    fn write_block(&self, addr: u8, reg: u8, data: &[u8]) -> Result<(), HardwareError> {
        if data.is_empty() || data.len() > 32 {
            return Err(HardwareError::Smbus(format!(
                "invalid SMBus block length {} (expected 1..=32)",
                data.len()
            )));
        }

        let _guard = self.lock.lock();
        let _os_guard = OsSmbusMutexGuard::acquire();
        self.wait_ready_and_clear()?;

        // AMD PIIX4 hardware requires reading SMBHSTCNT once to reset the internal 32-byte FIFO pointer.
        let _ = self.io.read_u8(self.base_port + SMBHSTCNT_OFF)?;
        self.io
            .write_u8(self.base_port + SMBHSTADD_OFF, addr << 1)?;
        self.io.write_u8(self.base_port + SMBHSTCMD_OFF, reg)?;
        self.io
            .write_u8(self.base_port + SMBHSTDAT0_OFF, data.len() as u8)?;
        for &byte in data {
            self.io.write_u8(self.base_port + SMBBLKDAT_OFF, byte)?;
        }
        self.io
            .write_u8(self.base_port + SMBHSTCNT_OFF, CNT_BLOCK_DATA | CNT_START)?;

        self.wait_completion(addr, reg)
    }
}

/// Scans standard Corsair Dominator SMBus addresses (`0x18..=0x1B`, `0x58..=0x5F`)
/// verifying Corsair vendor byte `0x1B` at reg `0x43` and model byte `0x03`/`0x04` at reg `0x44`.
/// Retries transient NACK/contention up to 3 times per candidate so dual-stick setups never miss
/// the second DIMM.
#[must_use]
pub fn probe_corsair_dominator_addrs(bus: &dyn SmbusTransport) -> Vec<u8> {
    const CANDIDATES: [u8; 12] = [
        0x18, 0x19, 0x1A, 0x1B, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F,
    ];
    let mut found = Vec::new();
    for addr in CANDIDATES {
        for attempt in 0..3 {
            let Ok(vendor) = bus.read_byte(addr, 0x43) else {
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_micros(150));
                }
                continue;
            };
            if vendor != 0x1B {
                break;
            }
            if let Ok(model) = bus.read_byte(addr, 0x44)
                && matches!(model, 0x03 | 0x04)
            {
                found.push(addr);
                break;
            }
            if attempt < 2 {
                std::thread::sleep(std::time::Duration::from_micros(150));
            }
        }
    }
    found
}

/// Recorded SMBus block write for dry-run inspection and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedSmbusWrite {
    /// 7-bit slave address.
    pub addr: u8,
    /// Register offset.
    pub reg: u8,
    /// Payload bytes.
    pub data: Vec<u8>,
}

/// In-memory SMBus transport used when no kernel SMBus driver is attached.
#[derive(Debug, Default)]
pub struct DryRunSmbusTransport {
    writes: Arc<Mutex<Vec<RecordedSmbusWrite>>>,
}

impl DryRunSmbusTransport {
    /// Creates a new [`DryRunSmbusTransport`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all recorded SMBus writes.
    #[must_use]
    pub fn writes(&self) -> Vec<RecordedSmbusWrite> {
        self.writes.lock().clone()
    }
}

impl SmbusTransport for DryRunSmbusTransport {
    fn write_byte(&self, addr: u8, reg: u8, value: u8) -> Result<(), HardwareError> {
        self.writes.lock().push(RecordedSmbusWrite {
            addr,
            reg,
            data: vec![value],
        });
        Ok(())
    }

    fn read_byte(&self, _addr: u8, _reg: u8) -> Result<u8, HardwareError> {
        Ok(0)
    }

    fn write_block(&self, addr: u8, reg: u8, data: &[u8]) -> Result<(), HardwareError> {
        self.writes.lock().push(RecordedSmbusWrite {
            addr,
            reg,
            data: data.to_vec(),
        });
        Ok(())
    }
}

/// Opens a live Windows SMBus transport via documented `inpoutx64.dll` if installed and open.
///
/// # Errors
///
/// Returns [`HardwareError::Smbus`] explaining why kernel port I/O is unavailable.
pub fn open_live_smbus_transport() -> Result<(Arc<dyn SmbusTransport>, String), HardwareError> {
    #[cfg(target_os = "windows")]
    {
        let io: Arc<dyn PortIo> = Arc::new(win::InpOutPortIo::load()?);
        let bus = Arc::new(Piix4SmbusTransport::new(io, PIIX4_PRIMARY_BASE));
        let desc = format!("AMD PIIX4 SMBus 0x{:04X} (inpoutx64)", bus.base_port());
        Ok((bus, desc))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(HardwareError::Smbus(
            "Live SMBus transport only implemented on Windows".to_owned(),
        ))
    }
}

struct OsSmbusMutexGuard {
    #[cfg(target_os = "windows")]
    handle: Option<windows::Win32::Foundation::HANDLE>,
}

impl OsSmbusMutexGuard {
    fn acquire() -> Self {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
            use windows::core::w;

            // Coordinate politely with motherboard/RAM monitoring utilities via standard Windows SMBus mutex.
            // SAFETY: Constant wide string literal, handle released in Drop.
            let handle =
                unsafe { CreateMutexW(None, false, w!("Global\\Access_SMBUS.HTP.Method")) }
                    .ok()
                    .filter(|h| !h.is_invalid());
            if let Some(h) = handle {
                // SAFETY: Valid mutex handle with bounded 50ms wait.
                unsafe {
                    let _ = WaitForSingleObject(h, 50);
                }
            }
            Self { handle }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self {}
        }
    }
}

impl Drop for OsSmbusMutexGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(h) = self.handle.take() {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::Threading::ReleaseMutex;

            // SAFETY: Valid mutex handle owned by this guard.
            unsafe {
                let _ = ReleaseMutex(h);
                let _ = CloseHandle(h);
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;

    use windows::Win32::Foundation::{FreeLibrary, HMODULE};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::PCSTR;

    use super::PortIo;
    use crate::error::HardwareError;

    type FnIsInpOutDriverOpen = unsafe extern "system" fn() -> i32;
    type FnDlPortReadPortUchar = unsafe extern "system" fn(u16) -> u8;
    type FnDlPortWritePortUchar = unsafe extern "system" fn(u16, u8);

    pub struct InpOutPortIo {
        module: HMODULE,
        read_u8_fn: FnDlPortReadPortUchar,
        write_u8_fn: FnDlPortWritePortUchar,
    }

    // SAFETY: Exported InpOutx64 function pointers and module handle are thread-safe across calls.
    unsafe impl Send for InpOutPortIo {}
    // SAFETY: Exported InpOutx64 function pointers and module handle are thread-safe across calls.
    unsafe impl Sync for InpOutPortIo {}

    impl Drop for InpOutPortIo {
        fn drop(&mut self) {
            if !self.module.is_invalid() {
                // SAFETY: Module handle loaded via `LoadLibraryW` in `load()`.
                unsafe {
                    let _ = FreeLibrary(self.module);
                }
            }
        }
    }

    impl InpOutPortIo {
        pub fn load() -> Result<Self, HardwareError> {
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Ok(exe) = std::env::current_exe()
                && let Some(dir) = exe.parent()
            {
                candidates.push(dir.join("inpoutx64.dll"));
            }
            candidates.push(PathBuf::from("inpoutx64.dll"));

            let mut module = None;
            for path in candidates {
                let wide: Vec<u16> = OsStr::new(&path)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                // SAFETY: Null-terminated UTF-16 path passed to `LoadLibraryW`.
                if let Ok(h) = unsafe { LoadLibraryW(windows::core::PCWSTR(wide.as_ptr())) }
                    && !h.is_invalid()
                {
                    module = Some(h);
                    break;
                }
            }

            let module = module.ok_or_else(|| {
                HardwareError::Smbus(
                    "inpoutx64.dll not found beside executable or in PATH (AMD PIIX4 0x0B00 needs kernel Port I/O)"
                        .to_owned(),
                )
            })?;

            // SAFETY: Resolving standard InpOutx64 C exports from loaded DLL module.
            let (is_open, read_fn, write_fn) = unsafe {
                let is_open_addr =
                    GetProcAddress(module, PCSTR(c"IsInpOutDriverOpen".as_ptr().cast()));
                let read_addr =
                    GetProcAddress(module, PCSTR(c"DlPortReadPortUchar".as_ptr().cast()));
                let write_addr =
                    GetProcAddress(module, PCSTR(c"DlPortWritePortUchar".as_ptr().cast()));

                let (Some(io_p), Some(r_p), Some(w_p)) = (is_open_addr, read_addr, write_addr)
                else {
                    let _ = FreeLibrary(module);
                    return Err(HardwareError::Smbus(
                        "inpoutx64.dll loaded but missing required Port I/O exports".to_owned(),
                    ));
                };

                let is_open: FnIsInpOutDriverOpen = std::mem::transmute(io_p);
                let read_fn: FnDlPortReadPortUchar = std::mem::transmute(r_p);
                let write_fn: FnDlPortWritePortUchar = std::mem::transmute(w_p);
                (is_open, read_fn, write_fn)
            };

            // SAFETY: Valid exported `IsInpOutDriverOpen` check.
            if unsafe { is_open() } == 0 {
                // SAFETY: Closing unused module before erroring.
                unsafe {
                    let _ = FreeLibrary(module);
                }
                return Err(HardwareError::Smbus(
                    "inpoutx64 driver not open (run once as Administrator so inpoutx64.sys starts)"
                        .to_owned(),
                ));
            }

            Ok(Self {
                module,
                read_u8_fn: read_fn,
                write_u8_fn: write_fn,
            })
        }
    }

    impl PortIo for InpOutPortIo {
        fn read_u8(&self, port: u16) -> Result<u8, HardwareError> {
            // SAFETY: Validated export pointer calling kernel driver port read.
            Ok(unsafe { (self.read_u8_fn)(port) })
        }

        fn write_u8(&self, port: u16, value: u8) -> Result<(), HardwareError> {
            // SAFETY: Validated export pointer calling kernel driver port write.
            unsafe {
                (self.write_u8_fn)(port, value);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    struct MockPiix4PortIo {
        writes: Mutex<Vec<(u16, u8)>>,
        block_fifo: Mutex<Vec<u8>>,
        regs: Mutex<BTreeMap<(u8, u8), u8>>,
        last_addr: Mutex<u8>,
        last_cmd: Mutex<u8>,
        dat0: Mutex<u8>,
        force_dev_err: bool,
    }

    impl MockPiix4PortIo {
        fn new() -> Self {
            Self {
                writes: Mutex::new(Vec::new()),
                block_fifo: Mutex::new(Vec::new()),
                regs: Mutex::new(BTreeMap::new()),
                last_addr: Mutex::new(0),
                last_cmd: Mutex::new(0),
                dat0: Mutex::new(0),
                force_dev_err: false,
            }
        }
    }

    impl PortIo for MockPiix4PortIo {
        fn read_u8(&self, port: u16) -> Result<u8, HardwareError> {
            let off = port.wrapping_sub(PIIX4_PRIMARY_BASE);
            match off {
                SMBHSTSTS_OFF => Ok(if self.force_dev_err {
                    STS_DEV_ERR
                } else {
                    STS_INTR
                }),
                SMBHSTDAT0_OFF => Ok(*self.dat0.lock()),
                SMBHSTCNT_OFF => {
                    self.block_fifo.lock().clear();
                    Ok(0)
                }
                _ => Ok(0),
            }
        }

        fn write_u8(&self, port: u16, value: u8) -> Result<(), HardwareError> {
            self.writes.lock().push((port, value));
            let off = port.wrapping_sub(PIIX4_PRIMARY_BASE);
            match off {
                SMBHSTADD_OFF => *self.last_addr.lock() = value,
                SMBHSTCMD_OFF => *self.last_cmd.lock() = value,
                SMBHSTDAT0_OFF => *self.dat0.lock() = value,
                SMBBLKDAT_OFF => self.block_fifo.lock().push(value),
                SMBHSTCNT_OFF if (value & CNT_START) != 0 => {
                    let addr_byte = *self.last_addr.lock();
                    let addr = addr_byte >> 1;
                    let is_read = (addr_byte & 0x01) != 0;
                    let reg = *self.last_cmd.lock();
                    if is_read {
                        let val = self.regs.lock().get(&(addr, reg)).copied().unwrap_or(0xFF);
                        *self.dat0.lock() = val;
                    }
                }
                _ => {}
            }
            Ok(())
        }
    }

    #[test]
    fn piix4_write_block_resets_fifo_and_pushes_payload() {
        let mock = Arc::new(MockPiix4PortIo::new());
        let bus = Piix4SmbusTransport::new(mock.clone(), PIIX4_PRIMARY_BASE);

        bus.write_block(0x19, 0x31, &[0x0C, 0xAA, 0xBB, 0xCC])
            .unwrap();

        assert_eq!(*mock.last_addr.lock(), 0x19 << 1);
        assert_eq!(*mock.last_cmd.lock(), 0x31);
        assert_eq!(*mock.dat0.lock(), 4);
        assert_eq!(&*mock.block_fifo.lock(), &[0x0C, 0xAA, 0xBB, 0xCC]);

        let writes = mock.writes.lock();
        assert!(writes.contains(&(
            PIIX4_PRIMARY_BASE + SMBHSTCNT_OFF,
            CNT_BLOCK_DATA | CNT_START
        )));
    }

    #[test]
    fn piix4_probes_corsair_dominator_sticks() {
        let mock = Arc::new(MockPiix4PortIo::new());
        {
            let mut r = mock.regs.lock();
            r.insert((0x19, 0x43), 0x1B);
            r.insert((0x19, 0x44), 0x04);
            r.insert((0x1B, 0x43), 0x1B);
            r.insert((0x1B, 0x44), 0x04);
        }
        let bus = Piix4SmbusTransport::new(mock, PIIX4_PRIMARY_BASE);
        let found = probe_corsair_dominator_addrs(&bus);
        assert_eq!(found, vec![0x19, 0x1B]);
    }

    #[test]
    fn piix4_surfaces_dev_err_status() {
        let mut mock = MockPiix4PortIo::new();
        mock.force_dev_err = true;
        let bus = Piix4SmbusTransport::new(Arc::new(mock), PIIX4_PRIMARY_BASE);
        let err = bus.write_byte(0x19, 0x31, 0x01).unwrap_err();
        assert!(err.to_string().contains("sts=0x04"));
    }
}
