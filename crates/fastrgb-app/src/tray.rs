//! Native Windows System Tray integration (`Shell_NotifyIconW`) and Win32 window icon helpers.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};

/// Embedded multi-resolution `FastRGB` `.ico` used for the System Tray and window title/taskbar.
pub const FASTRGB_ICO_BYTES: &[u8] = include_bytes!("../../../assets/fastrgb.ico");

/// Commands emitted from the native System Tray icon and context menu back to the GPUI app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Toggle main window visibility (show & focus, or hide to tray).
    ToggleWindow,
    /// Toggle RGB effect playback (`Play / Pause`).
    TogglePlayback,
    /// Toggle between safe Dry-Run and Live Hardware output.
    ToggleLiveHardware,
    /// Toggle Windows startup registration.
    ToggleAutostart,
    /// Cleanly shut down FastRGB.
    Exit,
}

/// Shared state mirrored to the native tray context menu for accurate checkmarks/labels.
#[derive(Debug, Default)]
pub struct TraySharedState {
    /// Whether the main window is currently hidden to the system tray.
    pub window_hidden: AtomicBool,
    /// Whether RGB animation playback is running.
    pub playing: AtomicBool,
    /// Whether live hardware output is enabled.
    pub live_hardware: AtomicBool,
    /// Whether `Start with Windows` is enabled.
    pub autostart: AtomicBool,
}

/// Handle controlling the background Windows System Tray icon thread.
pub struct TrayController {
    rx: Receiver<TrayCommand>,
    shared: Arc<TraySharedState>,
    #[cfg(target_os = "windows")]
    tray_hwnd: isize,
    #[cfg(target_os = "windows")]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl TrayController {
    /// Spawns the native Windows System Tray icon and returns its command handle.
    #[must_use]
    pub fn spawn(initial_hidden: bool, initial_autostart: bool) -> Option<Self> {
        let shared = Arc::new(TraySharedState::default());
        shared
            .window_hidden
            .store(initial_hidden, Ordering::Relaxed);
        shared.playing.store(true, Ordering::Relaxed);
        shared.live_hardware.store(false, Ordering::Relaxed);
        shared.autostart.store(initial_autostart, Ordering::Relaxed);

        #[cfg(target_os = "windows")]
        {
            let (tx, rx) = mpsc::channel();
            let (ready_tx, ready_rx) = mpsc::channel::<isize>();
            let shared_bg = Arc::clone(&shared);
            let thread = std::thread::Builder::new()
                .name("fastrgb-tray".to_owned())
                .spawn(move || {
                    win::run_tray_loop(tx, shared_bg, ready_tx);
                })
                .ok()?;
            let tray_hwnd = ready_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap_or(0);
            Some(Self {
                rx,
                shared,
                tray_hwnd,
                thread: Some(thread),
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let (_tx, rx) = mpsc::channel();
            Some(Self { rx, shared })
        }
    }

    /// Synchronizes app flags so the tray context menu displays up-to-date labels and checkmarks.
    pub fn sync_state(&self, window_hidden: bool, playing: bool, live: bool, autostart: bool) {
        self.shared
            .window_hidden
            .store(window_hidden, Ordering::Relaxed);
        self.shared.playing.store(playing, Ordering::Relaxed);
        self.shared.live_hardware.store(live, Ordering::Relaxed);
        self.shared.autostart.store(autostart, Ordering::Relaxed);
    }

    /// Drains all pending commands emitted by the tray icon without blocking.
    pub fn drain_commands(&self) -> Vec<TrayCommand> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = self.rx.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }
}

impl Drop for TrayController {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            if self.tray_hwnd != 0 {
                win::post_quit_to_tray(self.tray_hwnd);
            }
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }
}

/// Applies the embedded `FastRGB` icon (`ICON_SMALL` + `ICON_BIG`) onto a Win32 window `HWND`.
pub fn apply_window_icon(hwnd: isize) {
    #[cfg(target_os = "windows")]
    win::apply_window_icon(hwnd);
    #[cfg(not(target_os = "windows"))]
    let _ = hwnd;
}

/// Hides the main app window from the desktop and taskbar (leaving the tray icon active).
pub fn hide_window(hwnd: isize) {
    #[cfg(target_os = "windows")]
    win::hide_window(hwnd);
    #[cfg(not(target_os = "windows"))]
    let _ = hwnd;
}

/// Restores and brings the main app window to the foreground.
pub fn show_window(hwnd: isize) {
    #[cfg(target_os = "windows")]
    win::show_window(hwnd);
    #[cfg(not(target_os = "windows"))]
    let _ = hwnd;
}

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::Sender;

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
        DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetCursorPos,
        GetMessageW, GetWindowLongPtrW, HICON, ICON_BIG, ICON_SMALL, LR_DEFAULTCOLOR,
        LookupIconIdFromDirectoryEx, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG,
        PostMessageW, PostQuitMessage, RegisterClassW, SW_HIDE, SW_RESTORE, SW_SHOW, SendMessageW,
        SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
        TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_DESTROY,
        WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WM_SETICON, WNDCLASSW, WS_OVERLAPPED,
    };
    use windows::core::w;

    use super::{FASTRGB_ICO_BYTES, TrayCommand, TraySharedState};

    const WM_TRAY_CALLBACK: u32 = WM_APP + 42;
    const TRAY_ICON_UID: u32 = 1;

    const ID_TOGGLE_WINDOW: usize = 1001;
    const ID_TOGGLE_PLAY: usize = 1002;
    const ID_TOGGLE_LIVE: usize = 1003;
    const ID_TOGGLE_AUTOSTART: usize = 1004;
    const ID_EXIT: usize = 1005;

    struct TrayContext {
        tx: Sender<TrayCommand>,
        shared: Arc<TraySharedState>,
    }

    /// Creates an `HICON` of `desired_px × desired_px` directly from embedded `.ico` bytes.
    fn load_embedded_icon(desired_px: i32) -> Option<HICON> {
        // SAFETY: `FASTRGB_ICO_BYTES` is a valid static ICO file header + directory + image entries.
        unsafe {
            let offset = LookupIconIdFromDirectoryEx(
                FASTRGB_ICO_BYTES.as_ptr(),
                true,
                desired_px,
                desired_px,
                LR_DEFAULTCOLOR,
            );
            if offset <= 0 || (offset as usize) >= FASTRGB_ICO_BYTES.len() {
                return None;
            }
            let slice = &FASTRGB_ICO_BYTES[offset as usize..];
            CreateIconFromResourceEx(
                slice,
                true,
                0x0003_0000,
                desired_px,
                desired_px,
                LR_DEFAULTCOLOR,
            )
            .ok()
        }
    }

    pub fn apply_window_icon(raw_hwnd: isize) {
        let hwnd = HWND(raw_hwnd as *mut c_void);
        if let Some(small) = load_embedded_icon(16) {
            // SAFETY: Valid window handle and freshly created HICON.
            unsafe {
                let _ = SendMessageW(
                    hwnd,
                    WM_SETICON,
                    Some(WPARAM(ICON_SMALL as usize)),
                    Some(LPARAM(small.0 as isize)),
                );
            }
        }
        if let Some(big) = load_embedded_icon(48) {
            // SAFETY: Valid window handle and freshly created HICON.
            unsafe {
                let _ = SendMessageW(
                    hwnd,
                    WM_SETICON,
                    Some(WPARAM(ICON_BIG as usize)),
                    Some(LPARAM(big.0 as isize)),
                );
            }
        }
    }

    pub fn hide_window(raw_hwnd: isize) {
        let hwnd = HWND(raw_hwnd as *mut c_void);
        // SAFETY: Valid window handle from GPUI window.
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }

    pub fn show_window(raw_hwnd: isize) {
        let hwnd = HWND(raw_hwnd as *mut c_void);
        // SAFETY: Valid window handle from GPUI window.
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }

    pub fn post_quit_to_tray(raw_hwnd: isize) {
        let hwnd = HWND(raw_hwnd as *mut c_void);
        // SAFETY: Posting WM_CLOSE to our own hidden tray window to tear down cleanly.
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }

    pub fn run_tray_loop(
        tx: Sender<TrayCommand>,
        shared: Arc<TraySharedState>,
        ready_tx: Sender<isize>,
    ) {
        let class_name = w!("FastRgbTrayMessageWindow");
        // SAFETY: Retrieving current process module handle.
        let hinstance = unsafe { GetModuleHandleW(None) }.unwrap_or_default();

        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        // SAFETY: Registering private window class for tray callbacks.
        unsafe {
            let _ = RegisterClassW(&wc);
        }

        // SAFETY: Creating hidden utility window to own the tray icon and receive mouse events.
        let Ok(hwnd) = (unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                w!("FastRGB Tray"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }) else {
            let _ = ready_tx.send(0);
            return;
        };

        let ctx_box = Box::new(TrayContext { tx, shared });
        let ctx_ptr = Box::into_raw(ctx_box);
        // SAFETY: Storing owned context pointer in GWLP_USERDATA; reclaimed after message loop ends.
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ctx_ptr as isize);
        }

        let tray_icon = load_embedded_icon(32).unwrap_or_default();
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_UID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY_CALLBACK,
            hIcon: tray_icon,
            ..Default::default()
        };
        let tip: Vec<u16> = "FastRGB — Native RGB Studio".encode_utf16().collect();
        let copy_len = tip.len().min(nid.szTip.len().saturating_sub(1));
        nid.szTip[..copy_len].copy_from_slice(&tip[..copy_len]);

        // SAFETY: Adding notification icon bound to our hidden HWND.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }

        let _ = ready_tx.send(hwnd.0 as isize);

        let mut msg = MSG::default();
        // SAFETY: Standard Win32 message loop for the tray thread.
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            if !tray_icon.is_invalid() {
                let _ = DestroyIcon(tray_icon);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(ctx_ptr));
        }
    }

    unsafe extern "system" fn tray_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_TRAY_CALLBACK => {
                let event = (lparam.0 & 0xFFFF) as u32;
                // SAFETY: Reading context pointer previously stored by `run_tray_loop`.
                let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const TrayContext;
                if !ptr.is_null() {
                    // SAFETY: Pointer remains valid for the lifetime of the tray message loop.
                    let ctx = unsafe { &*ptr };
                    if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                        let _ = ctx.tx.send(TrayCommand::ToggleWindow);
                    } else if event == WM_RBUTTONUP {
                        show_context_menu(hwnd, ctx);
                    }
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                // SAFETY: Destroying hidden tray window on shutdown request.
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // SAFETY: Ending the tray thread's `GetMessageW` loop.
                unsafe {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => {
                // SAFETY: Forwarding unhandled messages to default Win32 window procedure.
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
    }

    fn show_context_menu(hwnd: HWND, ctx: &TrayContext) {
        let hidden = ctx.shared.window_hidden.load(Ordering::Relaxed);
        let playing = ctx.shared.playing.load(Ordering::Relaxed);
        let live = ctx.shared.live_hardware.load(Ordering::Relaxed);
        let autostart = ctx.shared.autostart.load(Ordering::Relaxed);

        // SAFETY: Creating and displaying a native Win32 popup menu at the cursor position.
        unsafe {
            let Ok(menu) = CreatePopupMenu() else {
                return;
            };
            let show_label = if hidden {
                w!("Show FastRGB")
            } else {
                w!("Hide to Tray")
            };
            let play_label = if playing {
                w!("Pause Effects")
            } else {
                w!("Resume Effects")
            };
            let live_flags = MF_STRING | if live { MF_CHECKED } else { MF_UNCHECKED };
            let auto_flags = MF_STRING | if autostart { MF_CHECKED } else { MF_UNCHECKED };

            let _ = AppendMenuW(menu, MF_STRING, ID_TOGGLE_WINDOW, show_label);
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, MF_STRING, ID_TOGGLE_PLAY, play_label);
            let _ = AppendMenuW(menu, live_flags, ID_TOGGLE_LIVE, w!("Live Hardware Output"));
            let _ = AppendMenuW(
                menu,
                auto_flags,
                ID_TOGGLE_AUTOSTART,
                w!("Start with Windows"),
            );
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, MF_STRING, ID_EXIT, w!("Exit FastRGB"));

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(hwnd);
            let selected = TrackPopupMenu(
                menu,
                TPM_LEFTALIGN
                    | TPM_BOTTOMALIGN
                    | windows::Win32::UI::WindowsAndMessaging::TPM_RETURNCMD,
                pt.x,
                pt.y,
                Some(0),
                hwnd,
                None,
            );
            let _ = DestroyMenu(menu);

            match selected.0 as usize {
                ID_TOGGLE_WINDOW => {
                    let _ = ctx.tx.send(TrayCommand::ToggleWindow);
                }
                ID_TOGGLE_PLAY => {
                    let _ = ctx.tx.send(TrayCommand::TogglePlayback);
                }
                ID_TOGGLE_LIVE => {
                    let _ = ctx.tx.send(TrayCommand::ToggleLiveHardware);
                }
                ID_TOGGLE_AUTOSTART => {
                    let _ = ctx.tx.send(TrayCommand::ToggleAutostart);
                }
                ID_EXIT => {
                    let _ = ctx.tx.send(TrayCommand::Exit);
                }
                _ => {}
            }
        }
    }
}
