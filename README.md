# 💿 iso-flasher

<div align="center">

### ⚡ Fast, Lightweight ISO-to-USB Flasher — Rust Rewrite

**Safe raw-device flashing • Live progress • Snake mode • No bloat**

[![Rust](https://img.shields.io/badge/Rust-100%25-000000?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Linux-primary-1793D1?style=for-the-badge&logo=linux)](https://github.com/anshlabs716/iso-flasher)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](https://opensource.org/license/mit/)

</div>

---

## ⚡ Quick Start

### Prerequisites

For Debian, Ubuntu, MX Linux, and other APT-based systems:

```bash
sudo apt update
sudo apt install -y git gcc cmake
```

> GCC is required for the legacy/native build tooling and project prerequisites. The application itself is now written in Rust.

### Build

```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher

# Install Rust if it is not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

cargo build --release
sudo install -m 755 target/release/iso-flasher /usr/local/bin/iso-flasher
```

### Run

```bash
sudo iso-flasher
```

Or:

```bash
sudo iso-flasher --iso image.iso --device /dev/sdX
```

### Uninstall

```bash
sudo rm /usr/local/bin/iso-flasher
rm -rf target
```

> ⚠️ **Destructive:** the selected block device is overwritten. Verify it before typing `FLASH`.

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
| WASD | Move Snake |
| Arrow keys | Move Snake |
| Shift+Tab | Switch Snake / flash view |
| Ctrl+C | Cancel |
| R | Refresh USB dashboard at the input screen |

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
