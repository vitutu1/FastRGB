# AGENTS.md — FastRGB

Engineering standards for the native Rust + GPUI desktop RGB controller.

## 1. Style & Formatting (Google Rust Style Guide)

- **Formatting**: Always pass `cargo fmt --all -- --check` with [`rustfmt.toml`](rustfmt.toml) (`max_width = 100`, `edition = "2024"`).
- **Lints**: Zero warnings under `cargo clippy --workspace --all-targets -- -D warnings`.
- **No Panics in Production Code**: Never use `.unwrap()`, `.expect()`, or `panic!()` outside tests. Propagate errors with typed `thiserror` enums or `Result`/`Option`.
- **Documentation**: Every public crate, module, struct, enum, trait, function, and method must carry a concise `///` doc comment (`missing_docs` enabled).
- **Unsafe Code**: Isolate `unsafe` strictly to OS-boundary modules (`fastrgb-capture`, `fastrgb-hardware`, `tray.rs`, `autostart.rs`) with `// SAFETY:` invariants.

## 2. Layered Workspace Architecture (DAG)

1. `fastrgb-core`: Pure domain types (`Rgb`, `Hsl`, `CanvasFrame`, `ParameterDescriptor`, `ParameterValue`, `ZoneGrid`). Zero OS/UI/hardware dependencies.
2. `fastrgb-effects`: `Effect` trait, `EffectRegistry`, and 47 procedural/reactive effects. Depends only on `fastrgb-core`.
3. `fastrgb-capture`: DXGI Desktop Duplication screen capture, `28x20` HSL + `160x100` HD zone extractor, synthetic test backend, and native Win32 `PrintWindow` snapshot tool.
4. `fastrgb-devices`: Virtual device layouts, area-filtered `CanvasSampler`, and `DeviceOutputSink` boundary.
5. `fastrgb-hardware`: Native Windows USB HID (ASUS Aura), NVAPI I2C (MSI Lovelace GPU), and AMD PIIX4 SMBus (Corsair Dominator RAM) drivers.
6. `fastrgb-app`: Native GPUI desktop studio, System Tray, Windows startup integration, and persistent JSON settings.

## 3. Verification Gate

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
