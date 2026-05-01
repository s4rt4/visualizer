# Chrome OS Porting Guide

Panduan ini menjelaskan opsi terbaik agar Audio Visualizer bisa berjalan di Chrome OS dengan UI dan menu yang tetap semirip mungkin dengan versi desktop. Fokus utama Chrome OS adalah web-based build, karena akses audio sistem di Chrome OS jauh lebih terbatas dibanding Windows WASAPI atau Debian PulseAudio/PipeWire.

## Rekomendasi Utama

Gunakan pendekatan:

```text
PWA + WebAssembly + Web Audio API
```

Lalu tambahkan Chrome Extension jika ingin capture audio dari tab aktif:

```text
Chrome Extension Manifest V3 + chrome.tabCapture
```

Alasan:

- Chrome OS sangat kuat untuk aplikasi web/PWA.
- `egui` bisa dikompilasi ke WebAssembly melalui `eframe`.
- Visual renderer dan FFT bisa dipakai ulang.
- Web Audio API cocok untuk input microphone, file audio, dan stream browser.
- Chrome Extension bisa capture audio tab aktif dengan UX yang lebih natural untuk browser.

## Target Hasil

- Versi PWA yang bisa dibuka dari Chrome dan di-install ke launcher Chrome OS.
- UI/menu visualizer tetap mengikuti versi desktop.
- Mode visual tetap sama: Bars, Wave, Ring.
- Theme, opacity, radius, gap, block style, dan settings panel tetap dipertahankan.
- Audio source minimal:
  - microphone/input device
  - audio file drag/drop
  - tab audio via Chrome Extension
  - screen/system audio via `getDisplayMedia` sebagai fallback dengan permission user

## Pilihan Metode Di Chrome OS

### 1. PWA Web-Based

Ini metode terbaik untuk versi umum.

Kelebihan:

- Jalan langsung di Chrome OS.
- Bisa di-install seperti app.
- Tidak perlu Linux container.
- Update mudah lewat hosting.
- UI `egui` bisa dipakai ulang via WASM.

Kekurangan:

- Tidak bisa diam-diam capture seluruh audio sistem.
- User harus memberi permission untuk microphone/screen capture.
- System audio tergantung dukungan Chrome dan pilihan share audio.

### 2. Chrome Extension Companion

Ini metode terbaik untuk capture audio dari tab browser.

Kelebihan:

- Bisa capture audio tab aktif memakai `chrome.tabCapture`.
- Cocok untuk YouTube, Spotify Web, browser media player, dan tab web lain.
- Bisa mengirim stream audio ke visualizer PWA/offscreen document.

Kekurangan:

- Harus dipasang sebagai extension.
- Capture biasanya butuh user gesture.
- Scope utamanya tab audio, bukan seluruh audio sistem Chrome OS.

### 3. Debian/Linux App Via Crostini

Chrome OS bisa menjalankan Linux apps melalui Crostini, jadi versi Debian mungkin bisa berjalan.

Kelebihan:

- Bisa memakai build Debian yang sama.
- Cocok untuk power user.

Kekurangan:

- Tidak terasa native untuk semua user Chrome OS.
- Audio routing dari Chrome OS host ke Linux container bisa terbatas.
- Capture system audio Chrome OS host belum tentu konsisten.

### 4. Android App

Tidak direkomendasikan untuk tahap awal.

Kelebihan:

- Bisa dipasang dari APK/Play Store.

Kekurangan:

- Perlu rewrite UI atau wrapper besar.
- Capture audio antar aplikasi dibatasi Android.
- Tidak efisien untuk mempertahankan UI `egui` yang sudah ada.

## Tech Stack Yang Disarankan

Core shared:

- Rust stable
- `egui 0.29`
- `eframe 0.29`
- `rustfft 6.2`
- `serde`
- `serde_json`

Build web:

- `wasm32-unknown-unknown`
- `trunk`
- `wasm-bindgen`
- `web-sys`
- `js-sys`

Audio web:

- Web Audio API
- `AudioContext`
- `AnalyserNode`
- `AudioWorklet` untuk tahap lanjut
- `MediaStreamAudioSourceNode`

PWA:

- `manifest.webmanifest`
- service worker
- app icons PNG
- static hosting, misalnya GitHub Pages, Cloudflare Pages, Netlify, atau Vercel

Chrome Extension:

- Manifest V3
- `chrome.tabCapture`
- service worker
- offscreen document jika perlu proses audio jangka panjang

## Dependency Rust Untuk Web

Tambahkan target web dependencies di `Cargo.toml`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
  "AudioContext",
  "AudioNode",
  "AnalyserNode",
  "MediaDevices",
  "MediaStream",
  "MediaStreamAudioSourceNode",
  "Navigator",
  "Window"
] }
```

Install target dan build tool:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

Run web build lokal:

```bash
trunk serve
```

## Struktur File Yang Disarankan

Tambahkan struktur web tanpa merusak desktop build:

```text
src/
  main.rs
  web_main.rs
  audio/
    mod.rs
    windows.rs
    linux_pulse.rs
    web.rs
  ui/
    mod.rs
    mode_bars.rs
    mode_waveform.rs
    mode_circular.rs
    renderer.rs
    settings_panel.rs

web/
  index.html
  manifest.webmanifest
  service-worker.js
  icons/
    icon-192.png
    icon-512.png

extension/
  manifest.json
  service_worker.js
  offscreen.html
  offscreen.js
```

## File Yang Perlu Diubah

### 1. `src/main.rs`

Pisahkan entry point desktop dan web:

```rust
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // desktop native entry point
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // kosong, web entry point memakai wasm_bindgen di web_main.rs
}
```

### 2. `src/web_main.rs`

Buat entry point WASM:

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();

    let options = eframe::WebOptions::default();
    eframe::WebRunner::new()
        .start(
            "visualizer_canvas",
            options,
            Box::new(|cc| Ok(Box::new(crate::ui::VisualizerApp::new(cc)))),
        )
        .await
}
```

Tambahkan dependency jika memakai panic hook:

```toml
console_error_panic_hook = "0.1"
```

### 3. `src/audio/web.rs`

Backend web tidak bisa memakai WASAPI atau PulseAudio. Backend web harus membaca audio dari `MediaStream`.

Sumber audio yang realistis:

- microphone via `navigator.mediaDevices.getUserMedia({ audio: true })`
- screen/tab audio via `navigator.mediaDevices.getDisplayMedia({ audio: true, video: true })`
- Chrome Extension tab stream via `chrome.tabCapture`
- file audio via `<input type="file">` atau drag/drop

Untuk versi awal, gunakan `AnalyserNode`:

```text
MediaStream
-> AudioContext
-> MediaStreamAudioSourceNode
-> AnalyserNode
-> getFloatTimeDomainData()
-> kirim sample ke visualizer
```

Catatan:

- `AnalyserNode` sudah menyediakan time domain dan frequency data.
- Jika ingin tetap memakai `rustfft`, ambil time domain data dari Web Audio lalu proses di Rust.
- Untuk performa lebih tinggi, pindahkan audio read ke `AudioWorklet`.

### 4. `src/config.rs`

Di web, config tidak bisa disimpan ke `%APPDATA%` atau filesystem biasa. Gunakan:

```text
localStorage
```

Strategi:

- Native desktop: file JSON.
- Web/Chrome OS: `window.localStorage`.

Pisahkan dengan `cfg`:

```rust
#[cfg(target_arch = "wasm32")]
fn load_config() {
    // localStorage
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config() {
    // filesystem
}
```

### 5. `src/ui/mod.rs`

Sebagian besar UI bisa dipakai ulang. Yang perlu disesuaikan:

- Pin window tidak relevan di web/PWA.
- Lock posisi window tidak tersedia di browser.
- Opacity window native tidak berlaku sama seperti desktop.

Di Chrome OS web mode:

- Sembunyikan tombol Pin, atau ubah menjadi no-op dengan tooltip.
- Settings tetap sama, tetapi bagian `Window` perlu disederhanakan.
- Background opacity masih bisa diterapkan ke canvas/panel, bukan native OS window.

### 6. `web/index.html`

Contoh minimal:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="theme-color" content="#080a0e" />
    <link rel="manifest" href="/manifest.webmanifest" />
    <title>Audio Visualizer</title>
  </head>
  <body>
    <canvas id="visualizer_canvas"></canvas>
  </body>
</html>
```

### 7. `web/manifest.webmanifest`

Contoh:

```json
{
  "name": "Audio Visualizer",
  "short_name": "Visualizer",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#080a0e",
  "theme_color": "#080a0e",
  "icons": [
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

## Chrome Extension Companion

Gunakan extension jika target utamanya audio dari tab aktif.

### `extension/manifest.json`

Contoh awal:

```json
{
  "manifest_version": 3,
  "name": "Audio Visualizer Capture",
  "version": "0.1.0",
  "permissions": ["tabCapture", "activeTab", "offscreen"],
  "background": {
    "service_worker": "service_worker.js"
  },
  "action": {
    "default_title": "Capture tab audio"
  }
}
```

### Flow Extension

```text
User klik extension icon
-> chrome.tabCapture.capture({ audio: true, video: false })
-> dapat MediaStream audio tab aktif
-> stream dikirim ke visualizer/offscreen document
-> Web Audio API membaca stream
-> visualizer render di PWA/window
```

Catatan:

- Capture harus dipicu user gesture.
- Extension tidak boleh capture semua audio sistem tanpa permission.
- Audio dari tab yang dicapture bisa perlu diteruskan lagi ke output supaya user tetap mendengar audio.

## Audio Capture Strategy

### Microphone/Input Device

Metode:

```js
navigator.mediaDevices.getUserMedia({ audio: true })
```

Cocok untuk:

- visualizer microphone
- testing awal
- perangkat tanpa tab capture

Tidak cocok untuk:

- capture musik yang sedang diputar dari aplikasi lain

### Tab Audio

Metode:

```js
chrome.tabCapture.capture({ audio: true, video: false })
```

Cocok untuk:

- YouTube
- Spotify Web
- media player web
- audio dari tab browser aktif

Ini opsi terbaik untuk Chrome OS jika user memang memutar audio dari browser.

### Screen/System Audio Fallback

Metode:

```js
navigator.mediaDevices.getDisplayMedia({
  video: true,
  audio: true
})
```

Cocok untuk:

- user yang rela memilih source screen/tab/window
- fallback tanpa extension

Kekurangan:

- Browser akan selalu meminta permission.
- User harus memilih opsi share audio jika tersedia.
- Dukungan system audio bisa berbeda tergantung OS/browser.

## Roadmap Implementasi

### Tahap 1: Web Build Tanpa Audio

Tujuan:

- UI `egui` tampil di browser.
- Mode visual bisa berjalan dengan fake/demo samples.
- Settings panel tampil benar di ukuran layar Chromebook.

Checklist:

- [ ] Tambah target `wasm32-unknown-unknown`.
- [ ] Tambah `web/index.html`.
- [ ] Tambah `src/web_main.rs`.
- [ ] `trunk serve` berhasil.
- [ ] Canvas tidak blank.
- [ ] Layout responsif di 1366x768 dan 1280x800.

### Tahap 2: Microphone Audio

Tujuan:

- Visualizer bereaksi terhadap microphone/input device.
- Permission flow jelas.

Checklist:

- [ ] Tambah backend `src/audio/web.rs`.
- [ ] Implement `getUserMedia`.
- [ ] Ambil time domain data via `AnalyserNode`.
- [ ] Feed data ke renderer/FFT.
- [ ] Tampilkan status permission/error.

### Tahap 3: PWA

Tujuan:

- App bisa di-install di Chrome OS launcher.

Checklist:

- [ ] Tambah `manifest.webmanifest`.
- [ ] Tambah service worker.
- [ ] Tambah icon 192 dan 512.
- [ ] Test install PWA di Chrome.

### Tahap 4: Chrome Extension Tab Capture

Tujuan:

- Capture audio dari tab aktif.

Checklist:

- [ ] Tambah folder `extension/`.
- [ ] Manifest V3.
- [ ] Implement `chrome.tabCapture`.
- [ ] Hubungkan stream ke Web Audio.
- [ ] Pastikan audio tetap terdengar oleh user.
- [ ] Test YouTube dan Spotify Web.

### Tahap 5: Polish Chrome OS

Tujuan:

- Rasanya seperti app native Chrome OS.

Checklist:

- [ ] Sembunyikan fitur Pin/lock position di web.
- [ ] Keyboard shortcut opsional.
- [ ] Touchpad/touch friendly spacing.
- [ ] Persist settings via localStorage.
- [ ] Tambahkan hosted release.

## GitHub Actions Untuk Web Build

Contoh workflow:

```yaml
name: Web Build

on:
  push:
    branches:
      - main
  workflow_dispatch:

jobs:
  build-web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install trunk
        run: cargo install trunk

      - name: Build web
        run: trunk build --release --dist dist-web

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: audio-visualizer-web
          path: dist-web
```

Untuk deploy ke GitHub Pages, tambahkan job deploy setelah build.

## Batasan Penting

- Browser tidak boleh capture audio sistem secara diam-diam.
- User gesture dan permission wajib untuk capture tab/screen/audio.
- Pin window, lock posisi, dan always-on-top tidak relevan untuk PWA biasa.
- Extension lebih kuat untuk tab audio, tetapi tetap dibatasi permission Chrome.
- Crostini/Linux app bukan opsi terbaik untuk user umum Chrome OS.

## Rekomendasi Final

Untuk Chrome OS, jalur terbaik adalah:

1. Buat versi PWA/WASM dulu.
2. Pertahankan UI `egui` sebanyak mungkin.
3. Implement audio web via Web Audio API.
4. Tambahkan Chrome Extension companion untuk tab audio.
5. Jadikan Debian/Crostini sebagai opsi sekunder untuk power user.

Dengan pendekatan ini, project tetap punya satu core visual Rust, tetapi audio backend bisa mengikuti platform:

```text
Windows      -> WASAPI loopback
Debian       -> PulseAudio/PipeWire monitor source
Chrome OS    -> Web Audio API + Chrome Extension tabCapture
```
