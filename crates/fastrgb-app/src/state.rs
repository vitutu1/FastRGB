//! Application state and frame orchestration connecting effects, capture, devices, hardware,
//! and GPUI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    mpsc::{SyncSender, TrySendError, sync_channel},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use fastrgb_capture::DxgiCaptureBackend;
use fastrgb_capture::{
    CaptureBackend, CaptureError, CaptureService, MonitorInfo, SyntheticCaptureBackend,
    capture_window_to_png, save_canvas_to_png,
};
use fastrgb_core::{CanvasFrame, HD_HEIGHT, HD_WIDTH, ParameterValue, Rgb};
use fastrgb_devices::{DeviceManager, DeviceOutputSink, LightingMode, SampledLedFrame};
use fastrgb_effects::{EffectEngine, EffectMetadata, EffectRegistry};
use fastrgb_hardware::{
    AsusAuraSink, CORSAIR_DOMINATOR_DEFAULT_ADDRS, CorsairDominatorSink, DryRunSmbusTransport,
    HardwareDiscoveryReport, HardwareMode, MSI_GPU_I2C_ADDR, MsiGpuControlMode, MsiLovelaceGpuSink,
    NvapiDriver, RgbOrder, discover_hardware, open_hid_transport, open_live_smbus_transport,
    probe_corsair_dominator_addrs,
};
use gpui::{Bounds, Context, Pixels, Point, RenderImage, Window};
use image::{Frame, RgbaImage};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use smallvec::smallvec;
use tracing::{info, warn};

use crate::autostart;
use crate::cli::AppOptions;
use crate::settings::AppSettings;
use crate::tray::{self, TrayCommand, TrayController};
use crate::ui;
use crate::ui::dot_field::DotFieldState;

/// Top-level navigation sections for the FastRGB studio shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavSection {
    /// Visual effect library grid.
    Library,
    /// Live canvas, device placement overlays, effect parameters, and screen capture.
    #[default]
    Lighting,
    /// Grouped Fan/RGB controllers, Peripherals/Hardware, and Hardware Bus diagnostics.
    Devices,
    /// Application startup, tray, performance, and protocol settings.
    Settings,
}

/// Effect library filter pills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    /// Show all effects.
    #[default]
    All,
    /// Static / minimal effects only.
    Static,
    /// Procedural `Basic/` effects only.
    Basic,
    /// Dynamic / screen-reactive effects only.
    Dynamic,
}

enum HardwareWorkerMsg {
    Frame(Vec<SampledLedFrame>),
    Shutdown,
}

struct HardwareWorker {
    tx: SyncSender<HardwareWorkerMsg>,
    last_error: Arc<Mutex<Option<String>>>,
    last_send: Instant,
    handle: Option<JoinHandle<()>>,
}

impl HardwareWorker {
    fn spawn(mut sinks: Vec<Box<dyn DeviceOutputSink>>) -> Self {
        let (tx, rx) = sync_channel::<HardwareWorkerMsg>(1);
        let last_error = Arc::new(Mutex::new(None));
        let err_slot = Arc::clone(&last_error);

        let handle = thread::Builder::new()
            .name("fastrgb-hw".to_owned())
            .spawn(move || {
                while let Ok(msg) = rx.recv() {
                    match msg {
                        HardwareWorkerMsg::Frame(frames) => {
                            for frame in &frames {
                                for sink in &mut sinks {
                                    if let Err(err) = sink.flush(frame)
                                        && let Ok(mut slot) = err_slot.lock()
                                    {
                                        *slot = Some(err.to_string());
                                    }
                                }
                            }
                        }
                        HardwareWorkerMsg::Shutdown => {
                            for sink in &mut sinks {
                                let _ = sink.shutdown(Rgb::BLACK);
                            }
                            break;
                        }
                    }
                }
            })
            .ok();

        Self {
            tx,
            last_error,
            last_send: Instant::now() - Duration::from_millis(20),
            handle,
        }
    }

    fn send_frame(&mut self, frames: Vec<SampledLedFrame>) {
        // Hard 60 Hz guard so fast UI slider drags or 120+ FPS repaints never flood SMBus/I2C/HID.
        let min_hw_interval = Duration::from_micros(1_000_000 / u64::from(MAX_RGB_FPS));
        let now = Instant::now();
        if now.duration_since(self.last_send) < min_hw_interval {
            return;
        }
        self.last_send = now;
        match self.tx.try_send(HardwareWorkerMsg::Frame(frames)) {
            Ok(()) | Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {}
        }
    }

    fn take_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|mut g| g.take())
    }
}

impl Drop for HardwareWorker {
    fn drop(&mut self) {
        let _ = self.tx.try_send(HardwareWorkerMsg::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Per-effect saved playback preset (speed multiplier, brightness, optional FPS cap).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectPlaybackPreset {
    /// Animation speed multiplier (`0.25..=3.0`, default `1.0`).
    pub speed: f32,
    /// Master brightness applied when this effect is active (`0.05..=1.0`, default `1.0`).
    pub brightness: f32,
    /// Optional target frame rate limit (`None` = unlimited / monitor sync).
    pub fps_cap: Option<u32>,
}

impl Default for EffectPlaybackPreset {
    fn default() -> Self {
        Self {
            speed: 1.0,
            brightness: 1.0,
            fps_cap: None,
        }
    }
}

/// Target property bound to an active pointer drag on a horizontal range slider.
#[derive(Debug, Clone, PartialEq)]
pub enum SliderDragTarget {
    /// Per-effect speed multiplier (`0.02x ..= 3.00x`).
    Speed(&'static str),
    /// Per-effect frame-rate cap (`5 ..= 240 FPS`, or `None` = Max).
    FpsCap(&'static str),
    /// Per-effect master brightness (`0.05 ..= 1.0`).
    Brightness(&'static str),
    /// Dynamic numeric effect parameter.
    Parameter {
        /// Parameter identifier.
        id: String,
        /// Minimum allowed value.
        min: f32,
        /// Maximum allowed value.
        max: f32,
        /// Quantization step.
        step: f32,
    },
}

/// Active range-slider drag session storing the track geometry in window pixels.
#[derive(Debug, Clone)]
pub struct ActiveSliderDrag {
    /// Parameter being dragged.
    pub target: SliderDragTarget,
    /// Track bounds captured during layout prepaint.
    pub bounds: Bounds<Pixels>,
}

/// Minimum selectable effect speed multiplier (`2%` slow-motion).
pub const MIN_EFFECT_SPEED: f32 = 0.02;
/// Maximum selectable effect speed multiplier.
pub const MAX_EFFECT_SPEED: f32 = 3.0;
/// Hard ceiling for RGB simulation and hardware bus updates (`60 FPS`) to protect SMBus/I2C/HID.
pub const MAX_RGB_FPS: u32 = 60;
const SPEED_GAMMA: f32 = 1.8;
const FPS_MIN: f32 = 5.0;
const FPS_MAX: f32 = MAX_RGB_FPS as f32;
const FPS_MAX_THRESHOLD: f32 = 0.96;

/// Maps normalized slider ratio `[0.0, 1.0]` onto `[0.02x, 3.00x]` with expanded slow-motion resolution.
#[must_use]
pub fn speed_from_ratio(ratio: f32) -> f32 {
    let t = ratio.clamp(0.0, 1.0);
    let raw = MIN_EFFECT_SPEED + (MAX_EFFECT_SPEED - MIN_EFFECT_SPEED) * t.powf(SPEED_GAMMA);
    (raw * 100.0).round() / 100.0
}

/// Inverse of [`speed_from_ratio`] for slider knob & fill rendering.
#[must_use]
pub fn ratio_from_speed(speed: f32) -> f32 {
    let norm = ((speed - MIN_EFFECT_SPEED) / (MAX_EFFECT_SPEED - MIN_EFFECT_SPEED)).clamp(0.0, 1.0);
    norm.powf(1.0 / SPEED_GAMMA)
}

/// Maps normalized slider ratio `[0.0, 1.0]` onto `Some(5..=60)` FPS, snapping to `None` (`60 FPS` max) at `>= 0.96`.
#[must_use]
pub fn fps_cap_from_ratio(ratio: f32) -> Option<u32> {
    let t = ratio.clamp(0.0, 1.0);
    if t >= FPS_MAX_THRESHOLD {
        None
    } else {
        let scaled = (t / FPS_MAX_THRESHOLD).clamp(0.0, 1.0);
        Some((FPS_MIN + scaled * (FPS_MAX - FPS_MIN)).round() as u32)
    }
}

/// Inverse of [`fps_cap_from_ratio`] for slider knob & fill rendering.
#[must_use]
pub fn ratio_from_fps_cap(fps_cap: Option<u32>) -> f32 {
    match fps_cap {
        None => 1.0,
        Some(fps) => {
            let norm = ((fps as f32 - FPS_MIN) / (FPS_MAX - FPS_MIN)).clamp(0.0, 1.0);
            norm * (FPS_MAX_THRESHOLD - 0.01)
        }
    }
}

/// Active right-click context menu targeting an effect's speed, brightness, and frame rate.
#[derive(Debug, Clone)]
pub struct EffectContextMenu {
    /// Target effect identifier.
    pub effect_id: &'static str,
    /// Display name shown in the menu header.
    pub effect_name: &'static str,
    /// Window coordinates where the context menu should anchor.
    pub position: Point<Pixels>,
}

/// Root GPUI view entity for the FastRGB desktop application.
pub struct FastRgbApp {
    pub(crate) registry: EffectRegistry,
    pub(crate) effects_list: Vec<EffectMetadata>,
    pub(crate) thumbnail_cache: HashMap<&'static str, PathBuf>,
    pub(crate) engine: EffectEngine,
    pub(crate) devices: DeviceManager,
    pub(crate) capture_backend: Box<dyn CaptureBackend>,
    pub(crate) capture_service: CaptureService,
    pub(crate) monitors: Vec<MonitorInfo>,
    pub(crate) selected_monitor: usize,
    pub(crate) playing: bool,
    pub(crate) show_device_overlay: bool,
    pub(crate) inspector_open: bool,
    pub(crate) show_diagnostics: bool,
    pub(crate) start_with_windows: bool,
    pub(crate) settings: AppSettings,
    pub(crate) window_hidden: bool,
    pub(crate) allow_exit: bool,
    pub(crate) icon_applied: bool,
    tray: Option<TrayController>,
    pub(crate) ambient_rgb: Rgb,
    pub(crate) dot_field: DotFieldState,
    pub(crate) toast: Option<(String, Instant)>,
    pub(crate) last_ui_tick: Instant,
    pub(crate) last_tick: Instant,
    pub(crate) last_preview_upload: Instant,
    pub(crate) fps_estimate: f32,
    pub(crate) target_fps_cap: Option<u32>,
    pub(crate) capped_frame_task_running: bool,
    pub(crate) effect_presets: HashMap<&'static str, EffectPlaybackPreset>,
    pub(crate) context_menu: Option<EffectContextMenu>,
    pub(crate) dragging_slider: Option<ActiveSliderDrag>,
    pub(crate) canvas_image: Option<Arc<RenderImage>>,
    pub(crate) retired_canvas_images: Vec<Arc<RenderImage>>,
    pub(crate) capture_preview_image: Option<Arc<RenderImage>>,
    pub(crate) retired_preview_images: Vec<Arc<RenderImage>>,
    pub(crate) last_capture_seq: u64,
    pub(crate) preview_scratch: Vec<u8>,
    pub(crate) raw_hwnd: Option<isize>,
    pub(crate) snapshot_status: Option<String>,
    pub(crate) snapshot_scheduled: bool,
    pub(crate) nav_section: NavSection,
    pub(crate) library_filter: LibraryFilter,
    pub(crate) library_page: usize,
    pub(crate) library_hw_task_running: bool,
    pub(crate) hardware_report: HardwareDiscoveryReport,
    pub(crate) hardware_mode: HardwareMode,
    pub(crate) asus_argb_order: RgbOrder,
    pub(crate) msi_gpu_mode: MsiGpuControlMode,
    pub(crate) hardware_status: Option<String>,
    hardware_worker: Option<HardwareWorker>,
    pub(crate) options: AppOptions,
}

impl FastRgbApp {
    /// Initializes the decoupled subsystems and constructs the GPUI root state.
    pub fn new(options: AppOptions, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = if options.is_snapshot_mode() {
            AppSettings::default()
        } else {
            AppSettings::load()
        };
        let registry = EffectRegistry::with_builtins();
        let effects_list = registry.list();
        let preferred_effect = if options.initial_effect != "rainbow" {
            options.initial_effect.as_str()
        } else {
            settings.last_effect_id.as_str()
        };
        let initial_id = if registry.create(preferred_effect).is_ok() {
            preferred_effect
        } else {
            "rainbow"
        };
        let engine = match EffectEngine::new(&registry, initial_id) {
            Ok(engine) => engine,
            Err(err) => {
                warn!("falling back to solid_color after effect init error: {err}");
                // `solid_color` is always registered in `with_builtins()`.
                EffectEngine::new(&registry, "solid_color").unwrap_or_else(|_| unreachable!())
            }
        };

        let mut capture_backend: Box<dyn CaptureBackend> = if options.synthetic_capture {
            Box::new(SyntheticCaptureBackend)
        } else {
            #[cfg(target_os = "windows")]
            {
                Box::new(DxgiCaptureBackend)
            }
            #[cfg(not(target_os = "windows"))]
            {
                Box::new(SyntheticCaptureBackend)
            }
        };

        let monitors = match capture_backend.list_monitors() {
            Ok(list) if !list.is_empty() => list,
            Ok(_) | Err(_) => {
                capture_backend = Box::new(SyntheticCaptureBackend);
                capture_backend.list_monitors().unwrap_or_default()
            }
        };

        let nav_section = match options.initial_section.to_ascii_lowercase().as_str() {
            "library" => NavSection::Library,
            "devices" | "hardware" => NavSection::Devices,
            "settings" | "config" => NavSection::Settings,
            _ => NavSection::Lighting,
        };
        let raw_hwnd = extract_win32_hwnd(window);
        let icon_applied = if let Some(hwnd) = raw_hwnd {
            tray::apply_window_icon(hwnd);
            true
        } else {
            false
        };
        let start_with_windows = autostart::is_enabled();
        let window_hidden = options.minimized && !options.is_snapshot_mode();
        let tray = if options.is_snapshot_mode() {
            None
        } else {
            TrayController::spawn(window_hidden, start_with_windows)
        };
        let asus_argb_order = settings.parsed_argb_order();
        let msi_gpu_mode = settings.parsed_msi_gpu_mode();
        let target_fps_cap = settings.rgb_fps_cap;
        let saved_brightness = settings.brightness;
        let mut devices = DeviceManager::with_builtin_presets();
        devices.set_global_brightness(saved_brightness);
        let hardware_report = discover_hardware();
        let thumbnail_cache = ui::library_panel::build_thumbnail_cache(&effects_list);
        let mut app = Self {
            registry,
            effects_list,
            thumbnail_cache,
            engine,
            devices,
            capture_backend,
            capture_service: CaptureService::new(),
            monitors,
            selected_monitor: 0,
            playing: true,
            show_device_overlay: true,
            inspector_open: true,
            show_diagnostics: false,
            start_with_windows,
            settings,
            window_hidden,
            allow_exit: false,
            icon_applied,
            tray,
            ambient_rgb: Rgb::new(108, 210, 255),
            dot_field: DotFieldState::default(),
            toast: None,
            last_ui_tick: Instant::now(),
            last_tick: Instant::now(),
            last_preview_upload: Instant::now(),
            fps_estimate: 60.0,
            target_fps_cap,
            capped_frame_task_running: false,
            effect_presets: HashMap::new(),
            context_menu: None,
            dragging_slider: None,
            canvas_image: None,
            retired_canvas_images: Vec::new(),
            capture_preview_image: None,
            retired_preview_images: Vec::new(),
            last_capture_seq: 0,
            preview_scratch: Vec::new(),
            raw_hwnd,
            snapshot_status: None,
            snapshot_scheduled: false,
            nav_section,
            library_filter: LibraryFilter::All,
            library_page: 0,
            library_hw_task_running: false,
            hardware_report,
            hardware_mode: HardwareMode::DryRun,
            asus_argb_order,
            msi_gpu_mode,
            hardware_status: None,
            hardware_worker: None,
            options,
        };

        if app.settings.live_on_startup
            && !app.hardware_report.external_rgb_running
            && !app.options.is_snapshot_mode()
        {
            app.hardware_mode = HardwareMode::LiveHardware;
            app.attach_live_hardware_sinks();
            app.sync_tray_state();
        }

        if app.engine.active_metadata().uses_screen {
            app.start_capture();
        }
        app.step_simulation(1.0 / 60.0);
        if app.window_hidden
            && let Some(hwnd) = app.raw_hwnd
        {
            tray::hide_window(hwnd);
        }
        if app.tray.is_some() {
            app.spawn_tray_pump(cx);
        }
        app.schedule_automated_snapshot(window, cx);
        app
    }

    /// Returns cached local thumbnail `.png` path for `effect_id`, if present.
    #[must_use]
    pub fn thumbnail_for(&self, effect_id: &str) -> Option<PathBuf> {
        self.thumbnail_cache.get(effect_id).cloned()
    }

    fn schedule_automated_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.snapshot_scheduled || !self.options.is_snapshot_mode() {
            return;
        }
        self.snapshot_scheduled = true;
        let delay = Duration::from_millis(self.options.snapshot_delay_ms);
        let window_path = self.options.snapshot_window.clone();
        let canvas_path = self.options.snapshot_canvas.clone();

        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let mut target_hwnd: Option<isize> = None;
            let _ = this.update_in(cx, |app, window, _cx| {
                app.raw_hwnd = extract_win32_hwnd(window);
                target_hwnd = app.raw_hwnd;
                if let Some(ref path) = canvas_path {
                    app.save_canvas_snapshot_at(path);
                }
            });
            if let Some(ref path) = window_path
                && let Some(hwnd) = target_hwnd
            {
                let path_clone = path.clone();
                let res = cx
                    .background_executor()
                    .spawn(async move { capture_window_to_png(hwnd, &path_clone) })
                    .await;
                let _ = this.update(cx, |app, _cx| {
                    app.apply_window_snapshot_result(path, res);
                });
            }
            let _ = this.update(cx, |_app, cx| {
                cx.quit();
            });
        })
        .detach();
    }

    /// Switches the active top-level navigation section and primes canvas buffers when entering `Lighting`.
    pub fn set_nav_section(&mut self, section: NavSection, cx: &mut Context<Self>) {
        self.nav_section = section;
        if section == NavSection::Lighting {
            self.last_tick = Instant::now();
            self.step_simulation(0.0);
        } else if matches!(section, NavSection::Library | NavSection::Settings)
            && self.playing
            && self.hardware_worker.is_some()
        {
            self.ensure_library_hw_loop(cx);
        }
        cx.notify();
    }

    /// Switches the active effect to `effect_id` and auto-starts screen capture if required.
    pub fn select_effect(&mut self, effect_id: &str, cx: &mut Context<Self>) {
        if let Err(err) = self.engine.switch_effect(&self.registry, effect_id) {
            self.snapshot_status = Some(format!("Effect error: {err}"));
            return;
        }
        let preset = self.effect_preset(effect_id);
        self.engine.set_speed_multiplier(preset.speed);
        self.devices.set_global_brightness(preset.brightness);
        self.target_fps_cap = preset.fps_cap;
        self.settings.last_effect_id = effect_id.to_owned();
        if !self.options.is_snapshot_mode() {
            self.settings.save();
        }
        if self.engine.active_metadata().uses_screen {
            if !self.capture_service.is_running() {
                self.start_capture();
            }
        } else if self.capture_service.is_running() {
            // Stop DXGI Desktop Duplication immediately when not needed to save CPU/GPU.
            self.capture_service.stop();
            if let Some(prev) = self.capture_preview_image.take() {
                self.retired_preview_images.push(prev);
            }
        }
        self.step_simulation(1.0 / 60.0);
        cx.notify();
    }

    /// Returns stored playback preset `(speed, brightness, fps_cap)` for `effect_id`.
    #[must_use]
    pub fn effect_preset(&self, effect_id: &str) -> EffectPlaybackPreset {
        self.effect_presets
            .get(effect_id)
            .copied()
            .unwrap_or_default()
    }

    /// Opens the right-click Speed / Brightness / FPS popover for `effect_id` at `position`.
    pub fn open_effect_context_menu(
        &mut self,
        effect_id: &'static str,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(meta) = self.effects_list.iter().find(|m| m.id == effect_id) else {
            return;
        };
        self.context_menu = Some(EffectContextMenu {
            effect_id: meta.id,
            effect_name: meta.name,
            position,
        });
        cx.notify();
    }

    /// Closes any open effect context popover.
    pub fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.context_menu.take().is_some() {
            cx.notify();
        }
    }

    /// Updates global / per-effect speed multiplier (`0.25..=3.0`).
    /// Updates speed multiplier (`0.02..=3.0`) for `effect_id` and applies immediately if active.
    pub fn set_effect_speed(
        &mut self,
        effect_id: &'static str,
        speed: f32,
        cx: &mut Context<Self>,
    ) {
        let clamped = (speed.clamp(MIN_EFFECT_SPEED, MAX_EFFECT_SPEED) * 100.0).round() / 100.0;
        let mut preset = self.effect_preset(effect_id);
        preset.speed = clamped;
        self.effect_presets.insert(effect_id, preset);

        if self.engine.active_metadata().id == effect_id {
            self.engine.set_speed_multiplier(clamped);
            if !self.playing {
                self.step_simulation(0.0);
            }
        }
        cx.notify();
    }

    /// Displays a transient minimalist status toast in the top-right corner.
    pub fn push_toast(&mut self, msg: impl Into<String>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    /// Returns the active toast text if it has not expired (`2.2s` lifetime).
    #[must_use]
    pub fn active_toast(&self) -> Option<&str> {
        let (msg, at) = self.toast.as_ref()?;
        if at.elapsed() < Duration::from_millis(2200) {
            Some(msg.as_str())
        } else {
            None
        }
    }

    /// Updates cursor coordinates for the `DotField` backdrop and triggers lightweight redraws.
    pub fn update_pointer_position(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.dot_field.set_pointer(pos);
        if matches!(self.nav_section, NavSection::Library | NavSection::Settings) || !self.playing {
            cx.notify();
        }
    }

    /// Spawns a click light ripple on the `DotField` backdrop.
    pub fn trigger_dot_ripple(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.dot_field.spawn_ripple(pos);
        if matches!(self.nav_section, NavSection::Library | NavSection::Settings) || !self.playing {
            cx.notify();
        }
    }

    /// Starts dragging a range slider, capturing its window bounds and applying the initial click position.
    pub fn begin_slider_drag(
        &mut self,
        target: SliderDragTarget,
        bounds: Bounds<Pixels>,
        mouse_x: Pixels,
        cx: &mut Context<Self>,
    ) {
        self.dragging_slider = Some(ActiveSliderDrag {
            target: target.clone(),
            bounds,
        });
        self.apply_slider_drag_x(&target, bounds, mouse_x, cx);
    }

    /// Continues an active range slider drag as the mouse moves across the window.
    pub fn update_slider_drag(&mut self, mouse_x: Pixels, cx: &mut Context<Self>) {
        let Some(drag) = self.dragging_slider.clone() else {
            return;
        };
        self.apply_slider_drag_x(&drag.target, drag.bounds, mouse_x, cx);
    }

    /// Ends the active range slider drag session cleanly.
    pub fn end_slider_drag(&mut self) {
        self.dragging_slider = None;
    }

    fn apply_slider_drag_x(
        &mut self,
        target: &SliderDragTarget,
        bounds: Bounds<Pixels>,
        mouse_x: Pixels,
        cx: &mut Context<Self>,
    ) {
        let width: f32 = bounds.size.width.into();
        if width <= 1.0 {
            return;
        }
        let origin_x: f32 = bounds.origin.x.into();
        let x: f32 = mouse_x.into();
        let ratio = ((x - origin_x) / width).clamp(0.0, 1.0);

        match target {
            SliderDragTarget::Speed(id) => {
                self.set_effect_speed(id, speed_from_ratio(ratio), cx);
            }
            SliderDragTarget::FpsCap(id) => {
                self.set_effect_fps_cap(id, fps_cap_from_ratio(ratio), cx);
            }
            SliderDragTarget::Brightness(id) => {
                let b = ((0.05 + ratio * 0.95) * 100.0).round() / 100.0;
                self.set_effect_brightness(id, b, cx);
            }
            SliderDragTarget::Parameter { id, min, max, step } => {
                let span = (max - min).max(0.001);
                let raw = min + ratio * span;
                let snapped = if *step > 0.0001 {
                    ((raw - min) / step).round() * step + min
                } else {
                    raw
                };
                let clamped = snapped.clamp(*min, *max);
                self.update_parameter(id, ParameterValue::Number(clamped), cx);
            }
        }
    }

    /// Updates global / per-effect brightness (`0.05..=1.0`).
    pub fn set_effect_brightness(
        &mut self,
        effect_id: &'static str,
        brightness: f32,
        cx: &mut Context<Self>,
    ) {
        let clamped = brightness.clamp(0.05, 1.0);
        let mut preset = self.effect_preset(effect_id);
        preset.brightness = clamped;
        self.effect_presets.insert(effect_id, preset);

        if self.engine.active_metadata().id == effect_id {
            self.devices.set_global_brightness(clamped);
            self.devices.update_from_canvas(self.engine.canvas());
        }
        cx.notify();
    }

    /// Updates optional frame rate cap (`None` = `60 FPS` max, or `Some(5..=60)`) for `effect_id`.
    pub fn set_effect_fps_cap(
        &mut self,
        effect_id: &'static str,
        fps_cap: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let clamped_cap = fps_cap.and_then(|f| {
            let clamped = f.clamp(5, MAX_RGB_FPS);
            if clamped >= MAX_RGB_FPS {
                None
            } else {
                Some(clamped)
            }
        });
        let mut preset = self.effect_preset(effect_id);
        preset.fps_cap = clamped_cap;
        self.effect_presets.insert(effect_id, preset);

        if self.engine.active_metadata().id == effect_id {
            self.target_fps_cap = clamped_cap;
            self.last_tick = Instant::now();
        }
        cx.notify();
    }

    /// Advances to the previous or next registered effect (used by bottom player controls).
    pub fn step_effect(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.effects_list.is_empty() {
            return;
        }
        let active_id = self.engine.active_metadata().id;
        let cur_idx = self
            .effects_list
            .iter()
            .position(|m| m.id == active_id)
            .unwrap_or(0) as isize;
        let len = self.effects_list.len() as isize;
        let next_idx = (cur_idx + delta).rem_euclid(len) as usize;
        let next_id = self.effects_list[next_idx].id;
        self.select_effect(next_id, cx);
    }

    /// Toggles `Start with Windows` registration under `HKCU\...\Run\FastRGB`.
    pub fn toggle_autostart(&mut self, cx: &mut Context<Self>) {
        let next = !self.start_with_windows;
        match autostart::set_enabled(next, self.settings.start_minimized) {
            Ok(()) => {
                self.start_with_windows = next;
                self.sync_tray_state();
                self.push_toast(
                    if next {
                        "FastRGB will start with Windows"
                    } else {
                        "Removed from Windows startup"
                    },
                    cx,
                );
            }
            Err(err) => {
                warn!("autostart toggle failed: {err}");
                self.push_toast(format!("Startup error: {err}"), cx);
            }
        }
    }

    /// Toggles launching hidden in the Windows System Tray and updates startup registration if active.
    pub fn toggle_start_minimized(&mut self, cx: &mut Context<Self>) {
        self.settings.start_minimized = !self.settings.start_minimized;
        self.settings.save();
        if self.start_with_windows {
            let _ = autostart::set_enabled(true, self.settings.start_minimized);
        }
        cx.notify();
    }

    /// Toggles automatic attachment of live HID / NVAPI / SMBus output on startup.
    pub fn toggle_live_on_startup(&mut self, cx: &mut Context<Self>) {
        self.settings.live_on_startup = !self.settings.live_on_startup;
        self.settings.save();
        cx.notify();
    }

    /// Toggles the interactive `DotField` canvas backdrop.
    pub fn toggle_dot_field(&mut self, cx: &mut Context<Self>) {
        self.settings.dot_field_enabled = !self.settings.dot_field_enabled;
        self.settings.save();
        cx.notify();
    }

    /// Updates the default global RGB frame cap (`5..=60 FPS`) and persists it.
    pub fn set_default_fps_cap(&mut self, cap: Option<u32>, cx: &mut Context<Self>) {
        let active_id = self.engine.active_metadata().id;
        self.set_effect_fps_cap(active_id, cap, cx);
        self.settings.rgb_fps_cap = self.target_fps_cap;
        self.settings.save();
    }

    /// Directly selects an ASUS Aura ARGB wire byte order and persists it.
    pub fn set_asus_argb_order(&mut self, order: RgbOrder, cx: &mut Context<Self>) {
        self.asus_argb_order = order;
        self.settings.asus_argb_order = order.label().to_owned();
        self.settings.save();
        if self.hardware_mode == HardwareMode::LiveHardware {
            self.attach_live_hardware_sinks();
        }
        self.push_toast(format!("ASUS ARGB wire order: {}", order.label()), cx);
    }

    /// Directly selects the MSI GPU I2C addressing mode and persists it.
    pub fn set_msi_gpu_mode(&mut self, mode: MsiGpuControlMode, cx: &mut Context<Self>) {
        self.msi_gpu_mode = mode;
        self.settings.msi_gpu_mode = mode.label().to_owned();
        self.settings.save();
        if self.hardware_mode == HardwareMode::LiveHardware {
            self.attach_live_hardware_sinks();
        }
        self.push_toast(format!("MSI GPU mode: {}", mode.label()), cx);
    }

    /// Restores default application preferences and persists them.
    pub fn reset_settings_to_default(&mut self, cx: &mut Context<Self>) {
        let defaults = AppSettings::default();
        self.asus_argb_order = defaults.parsed_argb_order();
        self.msi_gpu_mode = defaults.parsed_msi_gpu_mode();
        self.target_fps_cap = defaults.rgb_fps_cap;
        self.devices.set_global_brightness(defaults.brightness);
        self.settings = defaults;
        self.settings.save();
        if self.start_with_windows {
            let _ = autostart::set_enabled(true, self.settings.start_minimized);
        }
        self.push_toast("Settings reset to defaults", cx);
    }

    /// Hides the main window into the Windows System Tray while keeping RGB playback alive
    /// with minimal CPU/GPU usage (zero GPUI rendering and released preview textures).
    pub fn hide_to_tray(&mut self, cx: &mut Context<Self>) {
        if let Some(hwnd) = self.raw_hwnd {
            tray::hide_window(hwnd);
            self.window_hidden = true;
            if let Some(prev) = self.canvas_image.take() {
                self.retired_canvas_images.push(prev);
            }
            if let Some(prev) = self.capture_preview_image.take() {
                self.retired_preview_images.push(prev);
            }
            self.sync_tray_state();
            cx.notify();
        }
    }

    /// Toggles main window visibility between desktop and System Tray.
    pub fn toggle_window_visibility(&mut self, cx: &mut Context<Self>) {
        let Some(hwnd) = self.raw_hwnd else {
            return;
        };
        if self.window_hidden {
            tray::show_window(hwnd);
            self.window_hidden = false;
            self.last_tick = Instant::now();
            self.last_ui_tick = Instant::now();
            self.step_simulation(0.0);
        } else {
            tray::hide_window(hwnd);
            self.window_hidden = true;
            if let Some(prev) = self.canvas_image.take() {
                self.retired_canvas_images.push(prev);
            }
            if let Some(prev) = self.capture_preview_image.take() {
                self.retired_preview_images.push(prev);
            }
        }
        self.sync_tray_state();
        cx.notify();
    }

    /// Pushes current app flags into the native System Tray menu state.
    pub fn sync_tray_state(&self) {
        if let Some(tray) = &self.tray {
            tray.sync_state(
                self.window_hidden,
                self.playing,
                self.hardware_mode == HardwareMode::LiveHardware,
                self.start_with_windows,
            );
        }
    }

    fn spawn_tray_pump(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let sleep_ms = this
                    .read_with(cx, |app, _| {
                        if app.window_hidden && app.playing && app.hardware_worker.is_some() {
                            let fps = app
                                .target_fps_cap
                                .map_or(MAX_RGB_FPS, |f| f.clamp(5, MAX_RGB_FPS));
                            if fps >= 50 {
                                12u64
                            } else {
                                (1000 / u64::from(fps)).saturating_sub(3).max(12)
                            }
                        } else if app.window_hidden {
                            // Deep idle in tray: 0% GPU, ~0% CPU.
                            150u64
                        } else {
                            100u64
                        }
                    })
                    .unwrap_or(150);
                cx.background_executor()
                    .timer(Duration::from_millis(sleep_ms))
                    .await;

                let keep_running = this
                    .update(cx, |app, cx| {
                        let commands = app
                            .tray
                            .as_ref()
                            .map(TrayController::drain_commands)
                            .unwrap_or_default();
                        for cmd in commands {
                            match cmd {
                                TrayCommand::ToggleWindow => app.toggle_window_visibility(cx),
                                TrayCommand::TogglePlayback => app.toggle_playback(cx),
                                TrayCommand::ToggleLiveHardware => app.toggle_hardware_mode(cx),
                                TrayCommand::ToggleAutostart => app.toggle_autostart(cx),
                                TrayCommand::Exit => {
                                    app.allow_exit = true;
                                    app.tray = None;
                                    cx.quit();
                                    return false;
                                }
                            }
                        }

                        // Keep physical RGB LEDs animated at <= 60 FPS with zero GPU work while hidden.
                        if app.window_hidden && app.playing && app.hardware_worker.is_some() {
                            let now = Instant::now();
                            let effective_fps = app
                                .target_fps_cap
                                .map_or(MAX_RGB_FPS, |f| f.clamp(5, MAX_RGB_FPS));
                            let min_dt = 1.0 / (effective_fps as f32);
                            let dt = now.duration_since(app.last_tick).as_secs_f32();
                            if dt >= min_dt * 0.90 {
                                app.last_tick = now;
                                app.step_hardware_only(dt);
                            }
                        }
                        true
                    })
                    .unwrap_or(false);

                if !keep_running {
                    break;
                }
            }
        })
        .detach();
    }

    /// Toggles animation playback without touching window frame state outside `render`.
    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.playing = !self.playing;
        if self.playing {
            self.last_tick = Instant::now();
            if matches!(self.nav_section, NavSection::Library | NavSection::Settings)
                && self.hardware_worker.is_some()
            {
                self.ensure_library_hw_loop(cx);
            }
        }
        self.sync_tray_state();
        cx.notify();
    }

    /// Runs a lightweight hardware-only tick while viewing `Library` or `Settings` so physical LEDs
    /// stay animated without re-rendering static UI cards at 60 FPS.
    pub fn ensure_library_hw_loop(&mut self, cx: &mut Context<Self>) {
        if self.library_hw_task_running {
            return;
        }
        self.library_hw_task_running = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let keep_going = this
                    .update(cx, |app, _cx| {
                        if !matches!(app.nav_section, NavSection::Library | NavSection::Settings)
                            || !app.playing
                            || app.hardware_worker.is_none()
                        {
                            app.library_hw_task_running = false;
                            return false;
                        }
                        let now = Instant::now();
                        let dt = now.duration_since(app.last_tick).as_secs_f32();
                        app.last_tick = now;
                        app.step_hardware_only(dt);
                        true
                    })
                    .unwrap_or(false);
                if !keep_going {
                    break;
                }
            }
        })
        .detach();
    }

    /// Updates a parameter value on the active effect.
    pub fn update_parameter(&mut self, id: &str, value: ParameterValue, cx: &mut Context<Self>) {
        if let Err(err) = self.engine.set_parameter(id, value) {
            self.snapshot_status = Some(format!("Param error: {err}"));
            return;
        }
        if !self.playing {
            self.step_simulation(0.0);
        }
        cx.notify();
    }

    /// Cycles a device's lighting mode (`Canvas` -> `Forced(#00BBFF)` -> `Off`).
    pub fn cycle_device_mode(&mut self, device_id: &str, cx: &mut Context<Self>) {
        let current = self
            .devices
            .devices()
            .iter()
            .find(|d| d.layout.id == device_id)
            .map(|d| d.mode);
        let Some(mode) = current else {
            return;
        };
        let next = match mode {
            LightingMode::Canvas => LightingMode::Forced(Rgb::new(0x00, 0xBB, 0xFF)),
            LightingMode::Forced(_) => LightingMode::Off,
            LightingMode::Off => LightingMode::Canvas,
        };
        let _ = self.devices.set_device_mode(device_id, next);
        self.devices.update_from_canvas(self.engine.canvas());
        cx.notify();
    }

    /// Adjusts the LED count on a 1D strip/fan channel (`1..=120`).
    pub fn adjust_strip_leds(&mut self, device_id: &str, delta: i32, cx: &mut Context<Self>) {
        let current = self
            .devices
            .devices()
            .iter()
            .find(|d| d.layout.id == device_id)
            .map(|d| d.layout.leds.len() as i32);
        let Some(cur) = current else {
            return;
        };
        let next = (cur + delta).clamp(1, 120) as u32;
        let _ = self.devices.resize_strip_leds(device_id, next);
        self.devices.update_from_canvas(self.engine.canvas());
        cx.notify();
    }

    /// Re-runs system HID / NVAPI hardware discovery.
    pub fn refresh_hardware(&mut self, cx: &mut Context<Self>) {
        self.hardware_report = discover_hardware();
        let msg = format!(
            "Scanned {} HID · {} matched · {} GPU",
            self.hardware_report.all_hid.len(),
            self.hardware_report.matched_hid.len(),
            self.hardware_report.nvidia_gpus.len()
        );
        self.hardware_status = Some(msg.clone());
        self.push_toast(msg, cx);
    }

    /// Cycles the ASUS Aura ARGB channel wire byte order (`RGB` -> `GRB` -> `BGR`...).
    pub fn cycle_asus_argb_order(&mut self, cx: &mut Context<Self>) {
        self.set_asus_argb_order(self.asus_argb_order.next(), cx);
    }

    /// Toggles MSI GPU I2C addressing between `SingleZone` (`0x22=0x13`, `0x30..0x32`) and `PerLed` (`0x49`).
    pub fn toggle_msi_gpu_mode(&mut self, cx: &mut Context<Self>) {
        self.set_msi_gpu_mode(self.msi_gpu_mode.toggle(), cx);
    }

    /// Toggles between safe Dry-Run mode and Live Hardware output.
    pub fn toggle_hardware_mode(&mut self, cx: &mut Context<Self>) {
        match self.hardware_mode {
            HardwareMode::DryRun => {
                self.hardware_mode = HardwareMode::LiveHardware;
                self.attach_live_hardware_sinks();
                if self.nav_section == NavSection::Library && self.playing {
                    self.ensure_library_hw_loop(cx);
                }
                self.push_toast("Live hardware output enabled", cx);
            }
            HardwareMode::LiveHardware => {
                self.hardware_worker = None;
                self.devices.clear_sinks();
                self.hardware_mode = HardwareMode::DryRun;
                self.hardware_status = Some("Switched back to Dry-Run (safe mode)".to_owned());
                self.push_toast("Switched to Dry-Run", cx);
            }
        }
        self.sync_tray_state();
    }

    fn attach_live_hardware_sinks(&mut self) {
        self.hardware_worker = None;
        self.devices.clear_sinks();
        self.hardware_report = discover_hardware();

        let mut sinks: Vec<Box<dyn DeviceOutputSink>> = Vec::new();
        let mut attached = Vec::new();
        let mut errors = Vec::new();

        // 1. ASUS Aura HID Motherboard + 3 ARGB Fan/Strip Channels (0x0B05:0x19AF, MI_02)
        if let Some(asus) = self.hardware_report.matched_hid.first() {
            match open_hid_transport(&asus.info.path) {
                Ok(transport) => {
                    let mut sink = AsusAuraSink::new(transport.into(), self.asus_argb_order);
                    match sink.initialize() {
                        Ok(()) => {
                            let model = sink.firmware_model().unwrap_or("ASUS Aura").to_owned();
                            attached.push(format!(
                                "ASUS HID ({model}, {})",
                                self.asus_argb_order.label()
                            ));
                            sinks.push(Box::new(sink));
                        }
                        Err(err) => errors.push(format!("ASUS init: {err}")),
                    }
                }
                Err(err) => errors.push(format!("ASUS HID open: {err}")),
            }
        } else {
            errors.push("ASUS Aura 0x0B05:0x19AF (MI_02) not found".to_owned());
        }

        // 2. MSI RTX 4070 SUPER GAMING X SLIM via user-mode nvapi64.dll I2C (0x68)
        if let Some(msi_gpu) = self
            .hardware_report
            .nvidia_gpus
            .iter()
            .find(|g| g.sub_vendor_id == 0x1462)
        {
            match NvapiDriver::load().and_then(|drv| drv.open_i2c_bus(msi_gpu.index)) {
                Ok(i2c_bus) => {
                    let mut gpu_sink = MsiLovelaceGpuSink::new(
                        i2c_bus.into(),
                        MSI_GPU_I2C_ADDR,
                        self.msi_gpu_mode,
                    );
                    match gpu_sink.initialize() {
                        Ok(()) => {
                            attached.push(format!(
                                "{} (I2C 0x68, {})",
                                msi_gpu.name,
                                self.msi_gpu_mode.label()
                            ));
                            sinks.push(Box::new(gpu_sink));
                        }
                        Err(err) => errors.push(format!("MSI GPU I2C init: {err}")),
                    }
                }
                Err(err) => errors.push(format!("NVAPI open: {err}")),
            }
        }

        // 3. Corsair Dominator Platinum SMBus (AMD PIIX4 0x0B00 via inpoutx64.dll when present)
        match open_live_smbus_transport() {
            Ok((ram_bus, bus_desc)) => {
                let probed = probe_corsair_dominator_addrs(ram_bus.as_ref());
                let addrs = if probed.is_empty() {
                    CORSAIR_DOMINATOR_DEFAULT_ADDRS.to_vec()
                } else {
                    probed
                };
                let addr_labels: Vec<String> = addrs.iter().map(|a| format!("0x{a:02X}")).collect();
                sinks.push(Box::new(CorsairDominatorSink::new(ram_bus, addrs)));
                attached.push(format!(
                    "Corsair Dominator ({bus_desc}, addrs=[{}])",
                    addr_labels.join(",")
                ));
            }
            Err(err) => {
                let ram_bus = Arc::new(DryRunSmbusTransport::new());
                sinks.push(Box::new(CorsairDominatorSink::new(
                    ram_bus,
                    CORSAIR_DOMINATOR_DEFAULT_ADDRS.to_vec(),
                )));
                errors.push(format!("Corsair Dominator RAM (Dry-Run): {err}"));
            }
        }

        if !sinks.is_empty() {
            self.hardware_worker = Some(HardwareWorker::spawn(sinks));
        }

        let summary = if errors.is_empty() {
            format!("LIVE: {}", attached.join(" | "))
        } else {
            format!(
                "Active: [{}] | Warnings: [{}]",
                attached.join(", "),
                errors.join("; ")
            )
        };
        info!("{summary}");
        self.hardware_status = Some(summary);
    }

    /// Starts live screen capture on `self.selected_monitor`, falling back to synthetic if DXGI fails.
    pub fn start_capture(&mut self) {
        let interval = Duration::from_millis(16);
        if let Err(err) = self.capture_service.start(
            self.capture_backend.as_ref(),
            self.selected_monitor,
            interval,
        ) {
            warn!("capture start failed ({err}); falling back to SyntheticCaptureBackend");
            self.capture_backend = Box::new(SyntheticCaptureBackend);
            self.monitors = self.capture_backend.list_monitors().unwrap_or_default();
            self.selected_monitor = 0;
            if let Err(synth_err) =
                self.capture_service
                    .start(self.capture_backend.as_ref(), 0, interval)
            {
                self.snapshot_status = Some(format!("Capture failed: {synth_err}"));
                return;
            }
            self.snapshot_status = Some(format!(
                "DXGI unavailable ({err}); using synthetic display source"
            ));
        }
    }

    /// Toggles live screen capture on/off.
    pub fn toggle_capture(&mut self, cx: &mut Context<Self>) {
        if self.capture_service.is_running() {
            self.capture_service.stop();
        } else {
            self.start_capture();
        }
        cx.notify();
    }

    /// Selects a display monitor index and restarts capture if active.
    pub fn select_monitor(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.monitors.len() {
            return;
        }
        self.selected_monitor = index;
        if self.capture_service.is_running() {
            self.start_capture();
        }
        cx.notify();
    }

    /// Captures the live GPUI app window via Win32 `PrintWindow` into `captures/window_<ts>.png`.
    pub fn capture_app_window_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.raw_hwnd = extract_win32_hwnd(window);
        let Some(hwnd) = self.raw_hwnd else {
            self.snapshot_status = Some("Window HWND unavailable for PrintWindow".to_owned());
            cx.notify();
            return;
        };
        let path = timestamped_capture_path("window");
        cx.spawn(async move |this, cx| {
            let path_clone = path.clone();
            let res = cx
                .background_executor()
                .spawn(async move { capture_window_to_png(hwnd, &path_clone) })
                .await;
            let _ = this.update(cx, |app, cx| {
                app.apply_window_snapshot_result(&path, res);
                cx.notify();
            });
        })
        .detach();
    }

    /// Exports the live `320x200` RGB effect canvas into `captures/canvas_<ts>.png`.
    pub fn capture_canvas_snapshot(&mut self, cx: &mut Context<Self>) {
        let path = timestamped_capture_path("canvas");
        self.save_canvas_snapshot_at(&path);
        cx.notify();
    }

    fn apply_window_snapshot_result(&mut self, path: &Path, res: Result<(u32, u32), CaptureError>) {
        match res {
            Ok((w, h)) => {
                info!(
                    "captured native window snapshot ({w}x{h}) -> {}",
                    path.display()
                );
                self.snapshot_status =
                    Some(format!("Window PNG saved ({w}x{h}): {}", path.display()));
            }
            Err(err) => {
                warn!("window snapshot failed: {err}");
                self.snapshot_status = Some(format!("Window capture error: {err}"));
            }
        }
    }

    fn save_canvas_snapshot_at(&mut self, path: &Path) {
        match save_canvas_to_png(self.engine.canvas(), path) {
            Ok(()) => {
                info!("captured RGB canvas snapshot -> {}", path.display());
                self.snapshot_status = Some(format!("Canvas PNG saved: {}", path.display()));
            }
            Err(err) => {
                warn!("canvas snapshot failed: {err}");
                self.snapshot_status = Some(format!("Canvas capture error: {err}"));
            }
        }
    }

    /// Lightweight headless simulation tick used when hidden in the tray or viewing static tabs.
    /// Updates physical RGB hardware without building GPU `RenderImage` textures.
    pub fn step_hardware_only(&mut self, dt_seconds: f32) {
        let latest_capture = self.capture_service.latest();
        let zone_grid = latest_capture.as_ref().map(|snap| &snap.zone_grid);

        self.engine.step(dt_seconds, zone_grid);
        let sampled_frames = self.devices.update_from_canvas(self.engine.canvas());
        if let Some(worker) = &mut self.hardware_worker {
            worker.send_frame(sampled_frames);
            if let Some(err) = worker.take_error() {
                let msg = format!("Sink error: {err}");
                if self.hardware_status.as_deref() != Some(msg.as_str()) {
                    warn!("{msg}");
                    self.hardware_status = Some(msg);
                }
            }
        }
    }

    /// Advances simulation by `dt_seconds` and updates GPU render images + virtual/hardware LEDs.
    pub fn step_simulation(&mut self, dt_seconds: f32) {
        let latest_capture = self.capture_service.latest();
        let zone_grid = latest_capture.as_ref().map(|snap| &snap.zone_grid);

        self.engine.step(dt_seconds, zone_grid);
        let canvas = self.engine.canvas();
        let sampled_ambient = sample_dominant_ambient(canvas);
        self.ambient_rgb = blend_ambient(self.ambient_rgb, sampled_ambient);
        let sampled_frames = self.devices.update_from_canvas(canvas);
        if let Some(worker) = &mut self.hardware_worker {
            worker.send_frame(sampled_frames);
            if let Some(err) = worker.take_error() {
                let msg = format!("Sink error: {err}");
                if self.hardware_status.as_deref() != Some(msg.as_str()) {
                    warn!("{msg}");
                    self.hardware_status = Some(msg);
                }
            }
        }

        if !self.window_hidden
            && (self.nav_section == NavSection::Lighting || self.options.is_snapshot_mode())
        {
            if let Some(new_img) = build_canvas_render_image(self.engine.canvas())
                && let Some(prev) = self.canvas_image.replace(new_img)
            {
                self.retired_canvas_images.push(prev);
            }

            if let Some(snap) = latest_capture
                && snap.zone_grid.sequence != self.last_capture_seq
                && self.last_preview_upload.elapsed() >= Duration::from_millis(66)
            {
                self.last_capture_seq = snap.zone_grid.sequence;
                self.last_preview_upload = Instant::now();
                if let Some(preview_img) =
                    build_hd_preview_image(&snap.zone_grid.hd_rgba, &mut self.preview_scratch)
                    && let Some(prev) = self.capture_preview_image.replace(preview_img)
                {
                    self.retired_preview_images.push(prev);
                }
            }
        }
    }

    fn tick_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.raw_hwnd.is_none() {
            self.raw_hwnd = extract_win32_hwnd(window);
        }
        if !self.icon_applied
            && let Some(hwnd) = self.raw_hwnd
        {
            tray::apply_window_icon(hwnd);
            self.icon_applied = true;
        }

        // Hidden in System Tray: schedule zero GPUI animation frames so GPU stays at 0%.
        if self.window_hidden {
            return;
        }

        let now = Instant::now();
        let ui_dt = now.duration_since(self.last_ui_tick).as_secs_f32();
        self.last_ui_tick = now;
        if self.settings.dot_field_enabled {
            self.dot_field.step(ui_dt);
        }

        if let Some((_, at)) = &self.toast
            && at.elapsed() >= Duration::from_millis(2200)
        {
            self.toast = None;
        }

        let dot_active = self.settings.dot_field_enabled && self.dot_field.has_active_motion();

        if self.playing {
            let needs_live_ui =
                matches!(self.nav_section, NavSection::Lighting | NavSection::Devices);
            if needs_live_ui {
                let effective_fps = self
                    .target_fps_cap
                    .map_or(MAX_RGB_FPS, |f| f.clamp(5, MAX_RGB_FPS));
                let min_dt = 1.0 / (effective_fps as f32);
                let rgb_dt = now.duration_since(self.last_tick).as_secs_f32();

                if rgb_dt >= min_dt * 0.88 {
                    self.last_tick = now;
                    if rgb_dt > 0.0001 && rgb_dt < 0.25 {
                        let inst_fps = (1.0 / rgb_dt).clamp(1.0, MAX_RGB_FPS as f32);
                        self.fps_estimate = self.fps_estimate * 0.88 + inst_fps * 0.12;
                    }
                    self.step_simulation(rgb_dt);
                }

                if dot_active {
                    // Instant 120+ Hz responsiveness while moving the cursor or playing ripples.
                    window.request_animation_frame();
                } else {
                    // Idle cursor: pace GPUI repaints to the RGB cap to minimize GPU/CPU usage.
                    self.schedule_capped_frame(effective_fps, cx);
                }
            } else {
                self.last_tick = now;
                if self.hardware_worker.is_some() {
                    self.ensure_library_hw_loop(cx);
                }
                if dot_active {
                    window.request_animation_frame();
                }
            }
        } else {
            self.last_tick = now;
            if dot_active {
                window.request_animation_frame();
            }
        }
    }

    fn schedule_capped_frame(&mut self, cap_fps: u32, cx: &mut Context<Self>) {
        if self.capped_frame_task_running {
            return;
        }
        self.capped_frame_task_running = true;
        // Compensate for Windows' default 15.6ms timer granularity so 60 FPS wakes smoothly
        // without spinning at 240 FPS when the cursor is idle.
        let delay = if cap_fps >= 50 {
            Duration::from_millis(12)
        } else {
            let target_ms = (1000 / u64::from(cap_fps.max(1))).saturating_sub(3);
            Duration::from_millis(target_ms.max(12))
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |app, cx| {
                app.capped_frame_task_running = false;
                if !app.window_hidden
                    && app.playing
                    && matches!(app.nav_section, NavSection::Lighting | NavSection::Devices)
                {
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

impl gpui::Render for FastRgbApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.tick_frame(window, cx);
        ui::render_root(self, window, cx)
    }
}

fn extract_win32_hwnd(window: &Window) -> Option<isize> {
    let handle = <Window as HasWindowHandle>::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
        _ => None,
    }
}

fn sample_dominant_ambient(canvas: &CanvasFrame) -> Rgb {
    let w = canvas.width() as usize;
    let h = canvas.height() as usize;
    if w == 0 || h == 0 {
        return Rgb::new(108, 210, 255);
    }
    let step_x = (w / 16).max(1);
    let step_y = (h / 10).max(1);
    let mut sum_r = 0u64;
    let mut sum_g = 0u64;
    let mut sum_b = 0u64;
    let mut count = 0u64;
    let mut y = step_y / 2;
    while y < h {
        let mut x = step_x / 2;
        while x < w {
            if let Some(px) = canvas.get_pixel(x as u32, y as u32) {
                sum_r += u64::from(px.r);
                sum_g += u64::from(px.g);
                sum_b += u64::from(px.b);
                count += 1;
            }
            x += step_x;
        }
        y += step_y;
    }
    if count == 0 {
        return Rgb::new(108, 210, 255);
    }
    let r = ((sum_r as f32 / count as f32) * 1.45).clamp(64.0, 255.0) as u8;
    let g = ((sum_g as f32 / count as f32) * 1.45).clamp(64.0, 255.0) as u8;
    let b = ((sum_b as f32 / count as f32) * 1.45).clamp(72.0, 255.0) as u8;
    Rgb::new(r, g, b)
}

fn blend_ambient(prev: Rgb, next: Rgb) -> Rgb {
    let lerp = |a: u8, b: u8| -> u8 { (f32::from(a) * 0.88 + f32::from(b) * 0.12).round() as u8 };
    Rgb::new(
        lerp(prev.r, next.r),
        lerp(prev.g, next.g),
        lerp(prev.b, next.b),
    )
}

fn build_canvas_render_image(canvas: &CanvasFrame) -> Option<Arc<RenderImage>> {
    let mut raw = Vec::with_capacity((canvas.width() * canvas.height() * 4) as usize);
    canvas.write_bgra_bytes(&mut raw);
    let buf = RgbaImage::from_raw(canvas.width(), canvas.height(), raw)?;
    Some(Arc::new(RenderImage::new(smallvec![Frame::new(buf)])))
}

fn build_hd_preview_image(
    hd_rgba: &[fastrgb_core::Rgba],
    scratch: &mut Vec<u8>,
) -> Option<Arc<RenderImage>> {
    scratch.clear();
    scratch.reserve(hd_rgba.len() * 4);
    for px in hd_rgba {
        scratch.push(px.b);
        scratch.push(px.g);
        scratch.push(px.r);
        scratch.push(px.a);
    }
    let buf = RgbaImage::from_raw(HD_WIDTH, HD_HEIGHT, scratch.clone())?;
    Some(Arc::new(RenderImage::new(smallvec![Frame::new(buf)])))
}

fn timestamped_capture_path(prefix: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    PathBuf::from("captures").join(format!("{prefix}_{millis}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_and_fps_slider_curves_roundtrip() {
        assert!((speed_from_ratio(0.0) - MIN_EFFECT_SPEED).abs() < 1e-4);
        assert!((speed_from_ratio(1.0) - MAX_EFFECT_SPEED).abs() < 1e-4);
        let slow = speed_from_ratio(ratio_from_speed(0.10));
        assert!((slow - 0.10).abs() <= 0.01);

        assert_eq!(fps_cap_from_ratio(0.0), Some(5));
        assert_eq!(fps_cap_from_ratio(1.0), None);
        let thirty = fps_cap_from_ratio(ratio_from_fps_cap(Some(30)));
        assert!(matches!(thirty, Some(v) if (28..=32).contains(&v)));
    }
}
