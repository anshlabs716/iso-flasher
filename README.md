# 💿 iso-flasher

<div align="center">

### ⚡ Fast, Lightweight ISO-to-USB Flasher

A small Rust utility for writing ISO images directly to USB devices on Linux.

**Simple file picking • Live progress • Snake mode • Safety checks • No bloat**

[![Rust](https://img.shields.io/badge/Rust-2021-black?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Linux-supported-1793D1?style=for-the-badge&logo=linux)](https://www.kernel.org/)
[![License](https://img.shields.io/github/license/anshlabs716/iso-flasher?style=for-the-badge)](https://github.com/anshlabs716/iso-flasher/blob/main/LICENSE)

</div>

---

## ⚡ Quick Start

### 1. Install prerequisites

For Debian, Ubuntu, MX Linux, and other APT-based systems:

```bash
sudo apt update
sudo apt install -y git gcc rustc cargo
```

GCC provides the native Linux toolchain used by Rust's build/link process. Rust and Cargo come from the distro packages.

### 2. Build and install

```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
cargo build --release
sudo install -m 755 target/release/iso-flasher /usr/local/bin/iso-flasher
```

Run it with:

```bash
sudo iso-flasher
```

### Uninstall

```bash
sudo rm -f /usr/local/bin/iso-flasher
rm -rf iso-flasher
```

---

## 🖥️ How it works

The interactive flow is intentionally simple:

**Title screen → Select USB → Select ISO → Confirm → Flash**

1. iso-flasher opens your desktop file picker for the USB target.
2. Select the **whole USB device**, such as `/dev/sdb`.
3. The file picker opens again for the ISO image.
4. Confirm the destructive operation by typing `FLASH`.
5. iso-flasher unmounts the USB partitions and starts flashing.
6. Live progress, speed, and ETA are shown while the image is written.
7. Switch to Snake mode while flashing if you want.

On KDE, iso-flasher uses **KDialog**. If KDialog is unavailable, it falls back to **Zenity**.

> Do not select a partition such as `/dev/sdb1`. Select the whole device.

---

## 🦀 Features

- ⚡ Fast raw-device flashing
- 📁 Native desktop file picker for USB and ISO selection
- 📊 Live progress, speed, and ETA
- 🛡️ Removable-device safety gate
- 📏 ISO/device size checks
- 🔒 Exact `FLASH` confirmation
- 💾 Final sync and Linux block-cache flush
- 🐍 Snake mode while flashing
- ⌨️ Keyboard-friendly controls
- 🦀 Written entirely in Rust
- 🚫 No unnecessary dependencies or bloat

---

## 🖥️ Platform Support

| Platform | Status |
|---|---|
| 🐧 Linux | 🟢 Primary target |
| 🍎 macOS | 🔴 Not supported by the current Linux block-device backend |
| 😈 BSD | 🟡 Not tested |
| 🪟 Windows | 🔴 Not currently supported |

---

## 🎮 Controls

| Control | Action |
|---|---|
| WASD | Move Snake |
| Arrow keys | Move Snake |
| Shift+Tab | Switch Snake / flash view |
| Ctrl+C | Cancel |

---

## 🔐 Safety

Before flashing, iso-flasher:

1. Requires root.
2. Verifies the ISO is a regular file.
3. Requires a direct `/dev/<device>` target.
4. Rejects partitions in the interactive USB picker.
5. Checks target capacity.
6. Blocks non-removable targets unless `--force` is supplied.
7. Requires the exact word `FLASH`.
8. Attempts to unmount the target and its partitions.
9. Writes directly to the block device.
10. Syncs the output before success.

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

Bug fixes, Linux testing, USB compatibility testing, safety improvements, and UI improvements are welcome.

---

## 📜 License

iso-flasher is licensed under the MIT License.

<div align="center">

### 🦀 Rust • 🐧 Linux • 💿 USB flashing • 🐍 Snake mode

**No bloat. No fluff. Just ISO flashing.**

</div>
