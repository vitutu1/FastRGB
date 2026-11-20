//! Native Windows startup registration (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).

/// Returns `true` if `FastRGB` is currently registered to start automatically with Windows.
#[must_use]
pub fn is_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        win::is_enabled()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Enables or disables automatic launch of `FastRGB` on Windows sign-in.
///
/// When `minimized` is `true`, launches silently into the System Tray (`--minimized`).
///
/// # Errors
///
/// Returns a human-readable error message if the Windows Registry cannot be updated.
pub fn set_enabled(enabled: bool, minimized: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        win::set_enabled(enabled, minimized)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (enabled, minimized);
        Err("Start with Windows is only supported on Windows".to_owned())
    }
}

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
        RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows::core::{PCWSTR, w};

    const RUN_SUBKEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    const VALUE_NAME: PCWSTR = w!("FastRGB");

    pub fn is_enabled() -> bool {
        let mut hkey = HKEY::default();
        // SAFETY: Valid predefined root key and constant null-terminated subkey string.
        let open_ok = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                RUN_SUBKEY,
                Some(0),
                KEY_QUERY_VALUE,
                &mut hkey,
            )
        }
        .is_ok();
        if !open_ok {
            return false;
        }
        let mut data_len = 0u32;
        // SAFETY: Querying value presence on a valid open registry key handle.
        let exists = unsafe {
            RegQueryValueExW(hkey, VALUE_NAME, None, None, None, Some(&mut data_len)).is_ok()
        };
        // SAFETY: Closing key handle opened above.
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        exists && data_len > 2
    }

    pub fn set_enabled(enabled: bool, minimized: bool) -> Result<(), String> {
        let mut hkey = HKEY::default();
        // SAFETY: Opening HKCU Run key with write permission.
        unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                RUN_SUBKEY,
                Some(0),
                KEY_SET_VALUE,
                &mut hkey,
            )
        }
        .ok()
        .map_err(|err| format!("Registry open failed: {err}"))?;

        let result = if enabled {
            let exe = std::env::current_exe()
                .map_err(|err| format!("Failed to locate executable: {err}"))?;
            let cmd = if minimized {
                format!("\"{}\" --minimized", exe.display())
            } else {
                format!("\"{}\"", exe.display())
            };
            let wide: Vec<u16> = OsStr::new(&cmd)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let bytes =
                // SAFETY: Reinterpreting initialized UTF-16 slice as raw bytes for `RegSetValueExW`.
                unsafe { std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2) };
            // SAFETY: Valid open key handle and null-terminated UTF-16 `REG_SZ` buffer.
            unsafe { RegSetValueExW(hkey, VALUE_NAME, Some(0), REG_SZ, Some(bytes)) }
                .ok()
                .map_err(|err| format!("Registry write failed: {err}"))
        } else {
            // SAFETY: Deleting value on valid open key; ignore if already absent.
            let _ = unsafe { RegDeleteValueW(hkey, VALUE_NAME) };
            Ok(())
        };

        // SAFETY: Closing registry key opened above.
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        result
    }
}
