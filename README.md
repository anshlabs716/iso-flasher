# 💿 iso-flasher

<div align="center">

### ⚡ Fast, Lightweight ISO-to-USB Flasher — Rust Rewrite

**Safe raw-device flashing • Live progress • Snake mode • No bloat**

[![Rust](https://img.shields.io/badge/Rust-written%20in%20Rust-black?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Platform-Linux-1793D1?logo=linux&logoColor=white)](https://github.com/anshlabs716/iso-flasher)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/license/mit/)

</div>

---

## ⚡ Quick Start

### 1. Install prerequisites

For Debian, Ubuntu, MX Linux, and other APT-based systems, install everything needed to build iso-flasher:

```bash
sudo apt update
sudo apt install -y git gcc rustc cargo
```

> GCC provides the native Linux toolchain used by Rust's build/link process. Rust and Cargo are installed directly from the distro packages. No CMake or rustup setup is required.

### 2. Install and run iso-flasher

```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
cargo build --release
sudo install -m 755 target/release/iso-flasher /usr/local/bin/iso-flasher
sudo iso-flasher
```

### Uninstall

Remove the installed binary:

```bash
sudo rm -f /usr/local/bin/iso-flasher
```

Remove the cloned project separately:

```bash
rm -rf iso-flasher
```

---

## 🦀 Complete Rust Rewrite

The old C/CMake implementation has been replaced with a Rust/Cargo implementation.

Reworked components:

- 🦀 Rust application core
- 📦 Cargo build system
- 💿 Raw ISO-to-USB writing
- 🔌 Removable USB detection
- 📏 ISO/device size checks
- 🛡️ Removable-device safety gate
- 🔒 Exact `FLASH` confirmation
- 📊 Live progress, speed, and ETA
- 💾 Final sync and Linux block-cache flush
- ⌨️ Ctrl+C cancellation
- 🐍 Snake mode
- 🔄 USB refresh
- 🔎 Directory-based ISO discovery
- 📁 Interactive directory picker
- ⬆️⬇️ Arrow-key ISO browser
- 🔁 Tab-to-choose-another-directory
- 🧹 Direct unmount command without shell interpolation

---

## 🖥️ Platform Support

| Platform | Status |
|---|---|
| 🐧 Linux | 🟢 Primary development target |
| 📱 Termux | 🟡 Coming soon |
| 🍎 macOS | 🔴 Not supported by current Linux block-device backend |
| 😈 BSD | 🟡 Not tested |
| 🪟 Windows | 🔴 Not currently supported AND NEVER WILL BE 🤣 go switch to Linux! |

---

## 🎮 Controls

| Control | Action |
|---|---|
| ↑ / ↓ | Browse directories and ISO files |
| Enter | Select directory or ISO |
| Tab | Choose another directory while browsing ISO results |
| Esc | Cancel ISO selection |
| WASD | Move Snake |
| Arrow keys | Move Snake |
| Shift+Tab | Switch Snake / flash view |
| Ctrl+C | Cancel |
| R | Refresh USB dashboard at the input screen |

When run without `--iso`, iso-flasher first lets you choose which directory to scan. Use **↑ / ↓** to browse directories and **Enter** to scan the selected directory. The scan recursively searches only that directory for `.iso` files and shows the live number of images found. Press **Tab** to choose another directory or **Esc** to cancel. No manual ISO path entry is required.

---

## 🔐 Safety

Before flashing, iso-flasher:

1. Requires root.
2. Verifies the ISO is a regular file.
3. Requires a direct `/dev/<device>` target.
4. Checks target capacity.
5. Blocks non-removable targets unless `--force` is supplied.
6. Requires the exact word `FLASH`.
7. Attempts to unmount the target.
8. Writes directly to the block device.
9. Syncs the output before success.

**Never guess the target device. A wrong device can destroy data.**

---

## 🧰 CLI

```text
sudo iso-flasher
sudo iso-flasher --iso <image.iso> --device <device>
sudo iso-flasher --iso <image.iso> --device <device> --force
sudo iso-flasher --help
```

---

## 📦 Project Structure

```text
iso-flasher/
├── Cargo.toml
├── src/
│   └── main.rs
├── .github/
│   └── workflows/
│       └── rust.yml
├── .gitignore
├── LICENSE
├── README.md
└── SECURITY.md
```

---

## 🛠️ Development

```bash
cargo check
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

---

## 🤝 Contributing

Rust development, Linux testing, USB compatibility testing, safety testing, UI improvements, and future Termux work are welcome.

---

## 📜 License

iso-flasher is licensed under the MIT License.

---

<div align="center">

### 🦀 Rust rewrite • 🐧 Linux USB flashing • 🐍 Snake mode

**No bloat. No fluff. Just ISO flashing.**

</div>
