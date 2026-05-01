# Debian Porting Guide

Panduan ini menjelaskan cara membawa Audio Visualizer ke Debian/Linux dengan menu dan UI yang tetap sama seperti versi Windows. Target utama adalah menjaga layer UI tetap memakai `egui/eframe`, lalu mengganti bagian yang memang tergantung Windows: audio capture, resource/icon build, window behavior, dan packaging.

## Target Hasil

- Binary Linux native untuk Debian 12/13.
- UI, menu, ikon, mode visual, settings panel, dan preset warna tetap memakai kode `egui` yang sama.
- Audio system output terbaca dari monitor source PulseAudio/PipeWire.
- Distribusi awal berupa AppImage atau `.deb`; portable tarball juga bisa disediakan.

## Tech Stack Yang Disarankan

Tetap gunakan:

- Rust stable
- `eframe 0.29`
- `egui 0.29`
- `egui_extras 0.29` dengan fitur `svg`
- `rustfft 6.2`
- `ringbuf 0.4`
- `serde` + `serde_json`

Tambahkan untuk Debian/Linux audio:

- `libpulse-binding`
- `libpulse-simple-binding`

Alasan: mayoritas Debian desktop modern memakai PipeWire dengan kompatibilitas PulseAudio. Dengan membaca monitor source PulseAudio, app bisa menangkap audio output sistem tanpa harus meminta user mengaktifkan input seperti Stereo Mix.

Alternatif jangka panjang:

- `pipewire` crate untuk integrasi PipeWire native.

Namun untuk port pertama, PulseAudio compatibility layer lebih sederhana dan lebih stabil untuk kebutuhan visualizer.

## Dependency Sistem Debian

Install dependency build:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libpulse-dev \
  libasound2-dev \
  libx11-dev \
  libxi-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxinerama-dev \
  libxkbcommon-dev \
  libwayland-dev \
  libgl1-mesa-dev \
  libegl1-mesa-dev
```

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

## Menggunakan Codex Di Debian

Codex bisa dipakai langsung di Debian lewat Codex CLI. Ini berguna saat mengerjakan port Linux karena Codex dapat membaca folder project, mengedit file, menjalankan command build/test, dan membantu review perubahan lokal.

Install dependency dasar:

```bash
sudo apt update
sudo apt install -y nodejs npm git
```

Install Codex CLI:

```bash
npm i -g @openai/codex
```

Jalankan Codex dari folder project:

```bash
cd ~/visualizer
codex
```

Pada run pertama, Codex akan meminta login. Gunakan akun ChatGPT yang memiliki akses Codex, atau gunakan API key jika workflow kamu memakai API.

Contoh prompt yang cocok untuk port Debian:

```text
baca project ini dan jelaskan bagian mana yang masih Windows-specific
```

```text
port aplikasi ini ke Debian, ubah bagian audio backend saja dan pertahankan UI egui yang sama
```

Untuk upgrade Codex CLI:

```bash
npm i -g @openai/codex@latest
```

Jika versi `nodejs` dari repository Debian terlalu tua atau `npm` bermasalah, gunakan NodeSource atau `nvm` untuk memasang Node.js yang lebih baru, lalu ulangi install `@openai/codex`.

Alternatif tanpa install lokal adalah memakai Codex web/cloud:

- Buka `https://chatgpt.com/codex`.
- Connect GitHub account.
- Pilih repository.
- Jalankan task porting dari browser.

Referensi resmi:

- Codex CLI: `https://developers.openai.com/codex/cli`
- Codex web/cloud: `https://developers.openai.com/codex/cloud`
- OpenAI Codex repo: `https://github.com/openai/codex`

## File Yang Perlu Diubah

### 1. `Cargo.toml`

Pindahkan `winres` agar hanya dipakai di Windows:

```toml
[target.'cfg(windows)'.build-dependencies]
winres = "0.1"
```

Tambahkan dependency Linux:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
libpulse-binding = "2"
libpulse-simple-binding = "2"
```

`cpal` bisa tetap dipakai untuk Windows dan fallback input device, tetapi capture system audio di Debian sebaiknya jangan mengandalkan `cpal` saja.

### 2. `build.rs`

`build.rs` sekarang sudah memakai `#[cfg(windows)]`, jadi aman secara logika. Setelah `winres` dipindah ke target Windows, pastikan semua referensi `winres::WindowsResource` tetap berada di dalam blok:

```rust
#[cfg(windows)]
{
    let mut res = winres::WindowsResource::new();
}
```

Untuk Linux tidak perlu embed `.ico` lewat `build.rs`. Icon Linux akan dipasang lewat file `.desktop` dan packaging.

### 3. `src/main.rs`

Bagian ini bisa tetap sama untuk UI:

```rust
eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_inner_size([900.0, 560.0])
        .with_min_inner_size([600.0, 400.0])
        .with_decorations(true)
        .with_transparent(true)
        .with_icon(load_app_icon()),
    ..Default::default()
}
```

Catatan Debian:

- `with_transparent(true)` tidak selalu konsisten di Wayland.
- Always-on-top lebih stabil di X11 daripada Wayland.
- Lock posisi window lewat `ViewportCommand::OuterPosition` bisa dibatasi compositor Wayland.

Untuk hasil paling mirip Windows, uji pertama di sesi X11:

```bash
echo $XDG_SESSION_TYPE
```

Jika hasilnya `wayland`, tetap bisa berjalan, tetapi behavior pin/lock posisi mungkin tidak sama persis.

### 4. `src/audio.rs`

Ini bagian terbesar yang perlu dipisah. Saat ini Windows memakai WASAPI loopback melalui output device:

```rust
#[cfg(windows)]
fn audio_host() -> cpal::Host {
    cpal::host_from_id(cpal::HostId::Wasapi).unwrap_or_else(|_| cpal::default_host())
}
```

Untuk Debian, buat backend terpisah:

```text
src/audio/
  mod.rs
  windows.rs
  linux_pulse.rs
```

Struktur yang disarankan:

```rust
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux_pulse;

#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "linux")]
pub use linux_pulse::*;
```

Backend Linux harus:

- Enumerasi PulseAudio sources.
- Prioritaskan source dengan suffix `.monitor`.
- Pilih default monitor source dari default sink.
- Buka stream recording PulseAudio.
- Convert sample ke `f32`.
- Push sample ke `ringbuf::HeapProd<f32>`.
- Update `AudioState` dengan `sample_rate`, `channels`, `device_name`, `is_active`, dan `last_error`.

Pseudo-flow:

```text
PulseAudio context connect
-> get server info
-> ambil default_sink_name
-> monitor source = "{default_sink_name}.monitor"
-> buka Simple recording stream ke monitor source
-> loop read buffer
-> convert i16/f32 ke f32
-> producer.try_push(sample)
```

Jangan lakukan lock berat di audio read loop. Sama seperti Windows, callback/loop audio harus fokus memindahkan sample ke ring buffer.

### 5. `src/config.rs`

Pastikan lokasi config tidak hardcoded Windows. Idealnya:

- Windows: `%APPDATA%/AudioVisualizer/config.json`
- Linux: `$XDG_CONFIG_HOME/audio-visualizer/config.json`
- Fallback Linux: `~/.config/audio-visualizer/config.json`

Dependency opsional:

```toml
directories = "5"
```

Dengan `directories`, path config bisa dibuat lintas OS tanpa logika manual terlalu banyak.

### 6. `assets/`

Untuk Debian, siapkan icon PNG ukuran umum:

```text
assets/linux/
  audio-visualizer.png
```

Ukuran yang disarankan:

- 256x256 PNG
- 512x512 PNG jika ingin AppImage/release page lebih tajam

SVG tetap bagus untuk ikon internal UI. Logo app Linux sebaiknya PNG untuk kompatibilitas desktop entry.

### 7. `src/ui/*`

Targetnya tidak perlu rewrite UI.

File berikut sebaiknya tetap shared:

- `src/ui/mod.rs`
- `src/ui/settings_panel.rs`
- `src/ui/mode_bars.rs`
- `src/ui/mode_waveform.rs`
- `src/ui/mode_circular.rs`
- `src/ui/renderer.rs`

Yang perlu diberi perhatian:

- Tombol Pin tetap ada, tapi di Linux beri fallback jika compositor tidak mengizinkan lock posisi.
- Opacity/transparency perlu diuji di X11 dan Wayland.
- Jangan ubah layout settings panel kecuali ada perbedaan font/spacing Linux.

## Rencana Implementasi Bertahap

### Tahap 1: Build UI di Debian

Tujuan: app terbuka dengan UI yang sama, meski audio belum aktif.

Langkah:

```bash
cargo check
cargo run
```

Jika gagal karena `winres`, perbaiki target-specific build dependency di `Cargo.toml`.

### Tahap 2: Pisahkan Audio Backend

Refactor `src/audio.rs` menjadi module OS-specific:

```text
src/audio/mod.rs
src/audio/windows.rs
src/audio/linux_pulse.rs
```

Jaga API publik tetap sama:

```rust
AudioEngine::start(selected_device)
AudioEngine::restart(selected_device)
AudioEngine::drain_samples(max)
AudioEngine::state()
AudioEngine::devices()
```

Dengan begitu `src/ui` tidak perlu tahu apakah audio berasal dari WASAPI atau PulseAudio.

### Tahap 3: Implement PulseAudio Monitor Capture

Untuk Debian, `AudioEngine::devices()` harus menampilkan monitor source yang manusiawi, misalnya:

```text
Speakers Monitor
HDMI Monitor
USB Audio Monitor
```

Secara internal simpan nama source PulseAudio, misalnya:

```text
alsa_output.pci-0000_00_1f.3.analog-stereo.monitor
```

Kalau tidak ada monitor source:

- set `is_active = false`
- isi `last_error`
- UI tetap berjalan tanpa crash

### Tahap 4: Window Behavior Linux

Uji:

- resize window
- settings panel scroll
- pin always-on-top
- pin lock position
- opacity
- transparent background

Catatan penting:

- X11 biasanya mendukung always-on-top lebih baik.
- Wayland sering membatasi aplikasi untuk mengatur posisi window sendiri.
- Kalau lock posisi tidak bisa di Wayland, tampilkan behavior graceful: pin hanya menjadi always-on-top.

### Tahap 5: Packaging

Opsi awal yang disarankan:

1. Portable tarball:

```text
AudioVisualizer-linux-x86_64.tar.gz
```

Isi:

```text
audio_visualizer
README.md
LICENSE
assets/linux/audio-visualizer.png
```

2. AppImage:

Mudah dicoba user tanpa install penuh.

3. `.deb`:

Paling terasa native di Debian.

Struktur `.deb`:

```text
usr/bin/audio-visualizer
usr/share/applications/audio-visualizer.desktop
usr/share/icons/hicolor/256x256/apps/audio-visualizer.png
usr/share/doc/audio-visualizer/README.md
usr/share/licenses/audio-visualizer/LICENSE
```

Contoh desktop entry:

```ini
[Desktop Entry]
Type=Application
Name=Audio Visualizer
Comment=Native system audio visualizer
Exec=audio-visualizer
Icon=audio-visualizer
Terminal=false
Categories=AudioVideo;Audio;
StartupWMClass=audio_visualizer
```

## GitHub Actions Untuk Debian Build

Tambahkan workflow Linux terpisah:

```yaml
name: Linux Build

on:
  push:
    tags:
      - "v*"
  workflow_dispatch:

jobs:
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: |
          sudo apt update
          sudo apt install -y \
            pkg-config \
            libpulse-dev \
            libasound2-dev \
            libx11-dev \
            libxi-dev \
            libxcursor-dev \
            libxrandr-dev \
            libxinerama-dev \
            libxkbcommon-dev \
            libwayland-dev \
            libgl1-mesa-dev \
            libegl1-mesa-dev

      - name: Build
        run: cargo build --release

      - name: Package portable tarball
        run: |
          mkdir -p dist/AudioVisualizer-linux-x86_64
          cp target/release/audio_visualizer dist/AudioVisualizer-linux-x86_64/
          cp README.md LICENSE dist/AudioVisualizer-linux-x86_64/
          tar -czf dist/AudioVisualizer-linux-x86_64.tar.gz -C dist AudioVisualizer-linux-x86_64
```

## Checklist Porting

- [ ] `Cargo.toml` memakai target-specific dependency untuk `winres`.
- [ ] Dependency PulseAudio Linux ditambahkan.
- [ ] `src/audio.rs` dipecah menjadi backend Windows dan Linux.
- [ ] Backend Linux bisa membaca default monitor source.
- [ ] UI berjalan tanpa perubahan besar.
- [ ] Config path memakai XDG config directory di Linux.
- [ ] Icon Linux tersedia sebagai PNG.
- [ ] App diuji di X11.
- [ ] App diuji di Wayland.
- [ ] Portable Linux tarball dibuat.
- [ ] `.desktop` file dibuat.
- [ ] `.deb` atau AppImage dibuat.
- [ ] GitHub Actions Linux build ditambahkan.

## Risiko Yang Perlu Diantisipasi

- Wayland membatasi always-on-top dan lock posisi window.
- Beberapa desktop environment mematikan transparency atau blur behavior.
- Nama monitor source PulseAudio/PipeWire bisa berbeda antar mesin.
- User yang memakai ALSA murni tanpa PulseAudio/PipeWire perlu fallback khusus.
- Build Linux membutuhkan library desktop native yang tidak diperlukan di Windows.

## Rekomendasi Final

Untuk versi Debian pertama, jangan rewrite aplikasi ke framework lain. Tetap gunakan Rust + `egui/eframe`, karena menu dan UI sekarang sudah bisa dipertahankan hampir 1:1.

Perubahan utama cukup difokuskan ke:

1. Backend audio Linux berbasis PulseAudio monitor source.
2. Config path XDG.
3. Packaging Linux.
4. Fallback window behavior untuk Wayland.

Setelah versi Debian stabil, baru pertimbangkan backend PipeWire native sebagai peningkatan kualitas audio capture.
