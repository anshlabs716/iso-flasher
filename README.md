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

The XDG Desktop Portal client is a Rust crate (`zbus`), so there are **no extra system packages** and no desktop-specific dependencies to install.

### 2. Install and run iso-flasher

```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
cargo build --release
sudo install -m 755 target/release/iso-flasher /usr/local/bin/iso-flasher
sudo iso-flasher
```

### 3. (Optional) Build with the graphical window

The GTK4 window is **not built by default** — it adds GTK and system dependencies. To enable it:

```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
sudo apt install -y libgtk-4-dev pkg-config
cargo build --release --features gui
sudo install -m 755 target/release/iso-flasher /usr/local/bin/iso-flasher
```

Run the GUI with:

```bash
iso-flasher --gui
```

> The GUI runs as your desktop user and only escalates to root via `pkexec` when you click **Flash to USB**. It lists USB drives from sysfs, opens your desktop's native file chooser for the ISO, and shows live progress.

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
- 🔌 Removable USB detection from sysfs
- 🪟 Native file chooser via the XDG Desktop Portal
- 🧭 Desktop-independent USB device selection
- 📏 ISO/device size checks
- 🛡️ Removable-device safety gate
- 🔒 Exact `FLASH` confirmation
- 📊 Live progress, speed, and ETA
- 💾 Final sync and Linux block-cache flush
- ⌨️ Ctrl+C cancellation
- 🐍 Snake mode

---

## 🪟 File Selection

iso-flasher uses the **XDG Desktop Portal** (`org.freedesktop.portal.FileChooser`)
to choose the ISO file. This means you get the file picker your desktop is
already configured to use, rather than a custom directory browser:

| Desktop | Portal backend | Status |
|---|---|---|
| KDE Plasma | `xdg-desktop-portal-kde` | 🟢 Works |
| GNOME | `xdg-desktop-portal-gnome` / gtk | 🟢 Works |
| Xfce | `xdg-desktop-portal-gtk` | 🟢 Works |
| Cinnamon | `xdg-desktop-portal-gtk` | 🟢 Works |
| MATE | `xdg-desktop-portal-gtk` | 🟢 Works |
| Budgie | `xdg-desktop-portal-gtk` | 🟢 Works |
| LXQt | `xdg-desktop-portal-lxqt` | 🟢 Works |
| COSMIC | `xdg-desktop-portal-cosmic` | 🟢 Works |

Because the chooser comes from the portal, it works the same on **Wayland and
X11**, and no particular desktop's components are required. Dolphin, Nautilus,
Thunar, Nemo, Caja and PCManFM are never referenced or launched.

Run `iso-flasher --help` to see which chooser is active on your machine.

### Optional fallbacks

If no FileChooser portal is available, iso-flasher falls back to an installed
`zenity`, `kdialog` or `yad`, in that order. These are **not required**; a
missing portal on a normal desktop install is unusual.

If none of those is present either, iso-flasher says exactly what is missing and
still works from the command line:

```bash
sudo iso-flasher --iso ~/images/linux.iso --device /dev/sdb
```

### ISO filters

The chooser is offered `*.iso`, `*.img` and `application/x-cd-image`. These are
only a convenience: the portal may return a file that matches no filter, so the
selection is validated independently afterwards and an unusual filename still
works.

---

## 🔌 USB Device Selection

A USB drive is a raw block device, not a file, so it does **not** appear in a
normal file picker. iso-flasher never looks for it there. Instead it enumerates
whole block devices from `/sys/block` and offers a list showing the device path,
model, capacity, vendor and bus type:

```text
Detected removable devices
  1) /dev/sda  3.6 GiB  Storage Device  (MXT-USB)  [usb]
```

Devices are matched on `removable=1` **or** a USB bus, because USB sticks
plugged into some hubs report `removable=0`.

Safety rules that always apply:

- Partitions such as `/dev/sdb1` are rejected, however they were selected.
- Only validated whole-device targets under `/dev` are accepted.
- The exact word `FLASH` is required before anything is written.
- Non-removable targets require `--force`.


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
| 1..N | Choose a USB device from the detected list |
| WASD | Move Snake |
| Arrow keys | Move Snake |
| Shift+Tab | Switch Snake / flash view |
| Ctrl+C | Cancel |

When run without `--iso`, iso-flasher lists removable block devices for you to
choose from, then opens your desktop file picker for the ISO image.

---

## 🔐 Safety

Before flashing, iso-flasher:

1. Requires root.
2. Verifies the ISO is a non-empty regular file.
3. Rejects partitions and anything that is not a whole device under `/dev`.
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

### GUI (with `--features gui`)

```text
iso-flasher --gui
```

- Runs as your desktop user (not root)
- Lists USB drives from `/sys/block`
- Opens your desktop's native file chooser for the ISO
- Asks for your password via polkit when you click **Flash to USB**
- Shows live progress, speed and ETA
- **Cancel** button stops the write

---

## 📦 Project Structure

```text
iso-flasher/
├── Cargo.toml
├── src/
│   ├── main.rs       # flashing core, CLI, progress UI
│   ├── gui.rs        # optional GTK4 window (--features gui)
│   ├── session.rs    # desktop/session detection, session bus recovery
│   ├── portal.rs     # XDG Desktop Portal FileChooser client
│   ├── helpers.rs    # optional kdialog / zenity / yad fallbacks
│   ├── devices.rs    # removable block device discovery and validation
│   ├── select.rs     # selection flows for the ISO and the USB device
│   └── privileged.rs # polkit escalation for raw-device writes
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