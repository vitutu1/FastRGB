# FastRGB — System Architecture

## Overview

FastRGB is a native Rust + GPUI desktop RGB controller built around a **Canvas-as-Intermediate-Representation** pipeline: procedural and screen-reactive effects render onto a reusable `320x200` framebuffer, devices sample their physical LED footprints via an area box filter, and hardware sinks transmit frames over USB HID, NVAPI I2C, and SMBus on a dedicated worker thread capped at `<= 60 FPS`.

```text
  +--------------------+      ScreenFrame / ZoneGrid (28x20 HSL + 160x100)
  |  fastrgb-capture   | ------------------------------------------------+
  | (DXGI + Snapshot)  |                                                 |
  +--------------------+                                                 v
                                                                 +-----------------+
  +--------------------+      EffectContext (dt, params, screen) | fastrgb-effects |
  |    fastrgb-core    | <-------------------------------------- | (47 built-ins)  |
  | (Rgb, Hsl, Canvas, |                                         +-----------------+
  |  Params, ZoneGrid) |                                                 |
  +--------------------+                                                 | writes CanvasFrame (320x200)
             ^                                                           v
             |                                                 +-------------------+
             +------------------------------------------------ |  fastrgb-devices  |
             |        Area CanvasSampler + DeviceOutputSink    | (Layouts & Modes) |
             |                                                 +-------------------+
             |                                                           |
             +----------------------------+                              v
                                          |                    +-------------------+
                                          |                    | fastrgb-hardware  |
                                          |                    | (HID/NVAPI/SMBus) |
                                          |                    +-------------------+
                                          v                              ^
                                 +-----------------+                     |
                                 |   fastrgb-app   | --------------------+
                                 | (GPUI + Tray +  |   bounded 60 Hz worker
                                 |  JSON Settings) |
                                 +-----------------+
```

## Runtime Efficiency

- **Frameless GPUI UI**: Custom Obsidian titlebar (`appears_transparent = true`) with native drag/snap and close-to-tray behavior.
- **Zero GPU in Tray**: When hidden to the Windows System Tray, GPUI stops rendering and releases preview textures; only a lightweight CPU hardware tick runs when Live Output is enabled.
- **Bus Protection**: RGB simulation and hardware flushes are capped at `60 FPS` (configurable down to `15/30/45/60 FPS`) to avoid saturating SMBus/I2C and preserve gaming 1% lows.
- **On-Demand DXGI**: Screen capture runs only while a screen-reactive effect (`Screen Ambience`) is active and shuts down immediately when switching away.
