# Audio Visualizer

A lightweight native audio visualizer for Windows, built with Rust, egui, WASAPI loopback capture, and real-time FFT processing.

## Features

- Captures system audio output through Windows WASAPI loopback.
- Real-time spectrum analysis with logarithmic frequency bands.
- Three visual modes:
  - Bars, including stacked block style and mirror reflection.
  - Wave, with a smooth oscilloscope-style waveform.
  - Ring, with wave ring and radial bar variants.
- Ten built-in color themes: five gradient themes and five flat themes.
- Adjustable sensitivity, decay, band count, gap, radius, opacity, and window pinning.
- Persistent settings saved under `%APPDATA%\AudioVisualizer\config.json`.
- Native Windows window with app icon and DPI-aware manifest.

## Requirements

- Windows 10 or Windows 11.
- Rust stable toolchain.
- MSVC build tools for Rust on Windows.

Install the Windows MSVC target if needed:

```powershell
rustup target add x86_64-pc-windows-msvc
```

## Run

```powershell
cargo run
```

## Build

Debug build:

```powershell
cargo build
```

Release build:

```powershell
cargo build --release
```

The release executable is written to:

```text
target\release\audio_visualizer.exe
```

## Notes

- The audio path uses an output device with `build_input_stream()` for WASAPI loopback capture.
- If loopback capture is unavailable, the app attempts a Stereo Mix fallback when present.
- The audio callback writes to a lock-free ring buffer; FFT and rendering run on the UI thread.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
