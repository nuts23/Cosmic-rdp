# Cosmic RDP 🚀

<p align="center">
  <b>A modern, streamlined, first-party RDP client tailored for the COSMIC Desktop Environment.</b><br>
  Built with Rust, <code>libcosmic</code>, and <code>FreeRDP 3</code>.
</p>

---

## ✨ Features

- **Designed for COSMIC**:
  - Native Look & Feel: Built from the ground up using `libcosmic` / `iced`.
  - Auto Theme Sync: Seamlessly synchronizes with COSMIC light and dark modes.
  - Wayland Native: First-class Wayland support and display scaling.

- **Anti-Remmina Philosophy**:
  - **Single-Window Hub**: Fast, uncluttered connection grid with instant search and filtering.
  - **Streamlined Settings Drawer**: Clean sidebar drawer for connection configuration—no nested modal tabs.

- **Microsoft Teams Remote Optimization**:
  - **Low-Latency Audio**: 20ms PipeWire voice buffer optimization for crystal-clear remote calls.
  - **Webcam Passthrough**: Dynamic Virtual Channel (`/dvc:rdpecam`) redirection with automatic local V4L2 webcam discovery (`/dev/v4l/by-id`).
  - **Real-Time Microphone Controls**: Quick in-session toolbar toggle to mute/unmute local microphone.

- **Dynamic Display Scaling & EGFX**:
  - Automatically resizes remote Windows desktop resolution on the fly as the COSMIC window changes size.
  - Scaling modes: *Dynamic Server Resize*, *Fit to Window*, and *Original 1:1 Pixel Native*.
  - Resolution presets for 1080p, 2K QHD, 4K UHD, and Ultrawide.

- **Security & Compatibility**:
  - Passwords securely persisted in Freedesktop Secret Service (Keyring via `oo7`).
  - Trust-On-First-Use (TOFU) SHA-256 certificate validation.
  - Bi-directional Microsoft `.rdp` file import and export.

- **In-Session Host Keyboard Shortcuts**:
  - `Ctrl + Alt + Shift + F`: Toggle Fullscreen
  - `Ctrl + Alt + Shift + M`: Toggle Microphone Mute
  - `Ctrl + Alt + Shift + D`: Disconnect Session
  - `Ctrl + Alt + Shift + End`: Send `Ctrl + Alt + Del`

---

## 🛠️ Architecture

```
Cosmic-rdp/
├── crates/
│   ├── cosmic-rdp/                 # Main GUI binary application (libcosmic + iced)
│   ├── cosmic-rdp-core/            # FreeRDP 3 backend driver, EGFX, scancodes & mock runner
│   └── cosmic-rdp-models/          # Profile models, V4L2 device discovery, .rdp parser, oo7 keyring
├── roadmap.md                      # Architectural roadmap & phase breakdown
├── walkthrough.md                  # Implementation summary
└── justfile                        # Task runner recipes (build, run, test, release, install)
```

---

## 📦 Prerequisites

- **Rust toolchain** (1.80+ recommended)
- **FreeRDP 3** client binaries (`sdl-freerdp`, `wlfreerdp`, or `xfreerdp`)
- **libcosmic** build dependencies (`libxkbcommon`, `fontconfig`, `wayland-client`, `pipewire` / `alsa`)

On Fedora:
```bash
sudo dnf install cargo rust freerdp
```

On Pop!_OS / Ubuntu:
```bash
sudo apt install cargo rustc freerdp3-x11 freerdp3-sdl
```

---

## 🚀 Building & Running

Using [`just`](https://github.com/casey/just):

```bash
# Run the application
just run

# Run all workspace automated tests
just test

# Build optimized release binary
just build-release
```

Using standard `cargo`:

```bash
# Run development binary
cargo run -p cosmic-rdp

# Run tests
cargo test --workspace

# Build release
cargo build --workspace --release
```

---

## 📜 License

Licensed under the [GPL-3.0-or-later](LICENSE) license.
