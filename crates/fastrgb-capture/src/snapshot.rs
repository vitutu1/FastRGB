//! Native non-intrusive PNG snapshot utilities for RGB canvases and OS windows.
//!
//! Capturing the application window via Win32 `PrintWindow` allows inspecting rendered GPUI frames
//! without controlling the mouse, keyboard, or stealing focus.

use std::fs;
use std::path::Path;

use fastrgb_core::CanvasFrame;
use image::{ImageBuffer, RgbaImage};

use crate::error::CaptureError;

/// Saves a [`CanvasFrame`] (`320x200`) directly to a PNG file.
///
/// # Errors
///
/// Returns [`CaptureError::ImageIo`] if directory creation or PNG encoding fails.
pub fn save_canvas_to_png(canvas: &CanvasFrame, path: &Path) -> Result<(), CaptureError> {
    let bytes = canvas.to_rgba_bytes();
    save_rgba_to_png(canvas.width(), canvas.height(), &bytes, path)
}

/// Saves a tightly packed RGBA8 buffer to a PNG file, creating parent directories if needed.
///
/// # Errors
///
/// Returns [`CaptureError::ImageIo`] if the buffer length mismatches or saving fails.
pub fn save_rgba_to_png(
    width: u32,
    height: u32,
    rgba_bytes: &[u8],
    path: &Path,
) -> Result<(), CaptureError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|err| CaptureError::ImageIo(err.to_string()))?;
    }

    let img: RgbaImage =
        ImageBuffer::from_raw(width, height, rgba_bytes.to_vec()).ok_or_else(|| {
            CaptureError::ImageIo(format!(
                "RGBA buffer size {} does not match {width}x{height}",
                rgba_bytes.len()
            ))
        })?;
    img.save(path)
        .map_err(|err| CaptureError::ImageIo(err.to_string()))
}

/// Captures a specific native OS window (`raw_hwnd`) directly to a PNG file without controlling
/// the screen or moving the cursor.
///
/// # Errors
///
/// Returns [`CaptureError::WindowSnapshot`] if the window handle is invalid or capture fails.
#[cfg(target_os = "windows")]
pub fn capture_window_to_png(raw_hwnd: isize, path: &Path) -> Result<(u32, u32), CaptureError> {
    use std::ffi::c_void;
    use std::ptr;

    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    };
    use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    if raw_hwnd == 0 {
        return Err(CaptureError::WindowSnapshot(
            "null HWND provided".to_owned(),
        ));
    }

    let hwnd = HWND(raw_hwnd as *mut c_void);
    let mut rect = RECT::default();
    // SAFETY: `rect` is a valid mutable `RECT` passed to Win32 `GetWindowRect`.
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|err| CaptureError::WindowSnapshot(format!("GetWindowRect failed: {err}")))?;

    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    // SAFETY: `GetDC(Some(hwnd))` returns a window DC paired with `ReleaseDC` below.
    let screen_dc = unsafe { GetDC(None) };
    if screen_dc.is_invalid() {
        return Err(CaptureError::WindowSnapshot("GetDC failed".to_owned()));
    }

    // SAFETY: `CreateCompatibleDC` allocates an in-memory DC freed via `DeleteDC`.
    let mem_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    if mem_dc.is_invalid() {
        // SAFETY: `screen_dc` was successfully acquired above.
        unsafe {
            ReleaseDC(None, screen_dc);
        }
        return Err(CaptureError::WindowSnapshot(
            "CreateCompatibleDC failed".to_owned(),
        ));
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // Negative height creates a top-down DIB so rows match PNG scan order.
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits_ptr: *mut c_void = ptr::null_mut();
    // SAFETY: `bmi` is initialized and `bits_ptr` receives the DIB pixel pointer.
    let bitmap =
        unsafe { CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) };

    let bitmap = match bitmap {
        Ok(bmp) if !bmp.is_invalid() && !bits_ptr.is_null() => bmp,
        Ok(_) => {
            // SAFETY: Cleaning up valid DCs on failure path.
            unsafe {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
            }
            return Err(CaptureError::WindowSnapshot(
                "CreateDIBSection returned invalid handle".to_owned(),
            ));
        }
        Err(err) => {
            // SAFETY: Cleaning up valid DCs on failure path.
            unsafe {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
            }
            return Err(CaptureError::WindowSnapshot(format!(
                "CreateDIBSection failed: {err}"
            )));
        }
    };

    // SAFETY: `bitmap` is selected into `mem_dc`, captured via `PrintWindow` with
    // `PW_RENDERFULLCONTENT` (flag = 2) to include DirectComposition/GPUI content, then restored.
    let prev_obj = unsafe { SelectObject(mem_dc, bitmap.into()) };
    // PW_RENDERFULLCONTENT = 2
    let captured_ok = unsafe { PrintWindow(hwnd, mem_dc, PRINT_WINDOW_FLAGS(2)).as_bool() };

    let byte_len = (width as usize) * (height as usize) * 4;
    let mut rgba = Vec::with_capacity(byte_len);
    if captured_ok {
        // SAFETY: `bits_ptr` points to `width * height * 4` bytes of BGRA top-down pixel data
        // owned by `bitmap`, which remains alive until `DeleteObject` below.
        let bgra = unsafe { std::slice::from_raw_parts(bits_ptr as *const u8, byte_len) };
        for &[b, g, r, _] in bgra.as_chunks::<4>().0 {
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(255);
        }
    }

    // SAFETY: Restoring previous GDI object and releasing all GDI handles in reverse order.
    unsafe {
        SelectObject(mem_dc, prev_obj);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);
    }

    if !captured_ok {
        return Err(CaptureError::WindowSnapshot(
            "PrintWindow returned false".to_owned(),
        ));
    }

    save_rgba_to_png(width, height, &rgba, path)?;
    Ok((width, height))
}

/// Non-Windows stub for [`capture_window_to_png`].
#[cfg(not(target_os = "windows"))]
pub fn capture_window_to_png(_raw_hwnd: isize, _path: &Path) -> Result<(u32, u32), CaptureError> {
    Err(CaptureError::WindowSnapshot(
        "native HWND PrintWindow capture is only supported on Windows".to_owned(),
    ))
}
