# 💿 iso-flasher

<div align="center">

### ⚡ A Powerful, No-Bloat ISO-to-USB Flasher

**Fast • Lightweight • Written in C**

[![C](https://img.shields.io/badge/C-100%25-A8B9CC?style=for-the-badge&logo=c&logoColor=black)](https://en.wikipedia.org/wiki/C_(programming_language))
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20BSD%20%7C%20Termux-1793D1?style=for-the-badge)](https://github.com/anshlabs716/iso-flasher)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](https://opensource.org/license/mit/)

## 📑 Table of Contents

  - [⚡ A Powerful, No-Bloat ISO-to-USB Flasher](#-a-powerful-no-bloat-iso-to-usb-flasher)
- [🚧 STILL IN DEVELOPMENT 🚧](#-still-in-development-)
- [📖 What is iso-flasher?](#-what-is-iso-flasher)
- [✨ Planned Features](#-planned-features)
  - [💿 ISO Flashing](#-iso-flashing)
  - [🔍 Automatic Detection](#-automatic-detection)
  - [📦 Dependency Handling](#-dependency-handling)
  - [🎨 User Experience](#-user-experience)
- [📦 Supported Package Managers](#-supported-package-managers)
- [🖥️ Platform Support](#-platform-support)
- [🛠️ Requirements](#-requirements)
  - [📱 Termux](#-termux)
- [⚡ Quick Start](#-quick-start)
  - [1. Clone the repository](#1-clone-the-repository)
  - [2. Compile](#2-compile)
  - [3. Run](#3-run)
  - [4. Install](#4-install)
  - [5. Uninstall](#5-uninstall)
- [🖥️ GUI](#-gui)
  - [🚧 GUI COMING SOON](#-gui-coming-soon)
- [📁 Project Structure](#-project-structure)
- [🧩 Development Status](#-development-status)
- [🗺️ Roadmap](#-roadmap)
- [🛡️ Safety](#-safety)
- [🐛 Bug Reports](#-bug-reports)
- [🤝 Contributing](#-contributing)
- [🔐 Security](#-security)
- [📜 License](#-license)
- [👨‍💻 Author](#-author)
- [⭐ Support](#-support)

## 🚧 STILL IN DEVELOPMENT 🚧

**iso-flasher can now write bootable ISO images directly to USB block devices on Linux.**

**GUI coming soon • Termux support coming soon**

> ⚠️ **Platform notice:** Linux is currently the primary development target.  
> macOS and BSD have **not been tested yet**.

</div>

---

## ⚡ Quick Start

### 1. Clone the repository

~~~~bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
~~~~

### 2. Compile

Using CMake:

~~~~bash
cmake -B build
cmake --build build
~~~~

### 3. Run

~~~~bash
sudo ./build/iso-flasher
~~~~

The interactive mode lists removable block devices, asks for an ISO and target device, requires typing `FLASH`, unmounts the target, then writes the ISO directly to the selected device.

> ⚠️ **Destructive operation:** flashing overwrites the selected device. Verify the target path before confirming.

### 4. Install

To install the compiled binary system-wide:

~~~~bash
sudo install -m 755 build/iso-flasher /usr/local/bin/iso-flasher
~~~~

You can then run it from anywhere:

~~~~bash
sudo iso-flasher
~~~~

### 5. Uninstall

If you installed it to `/usr/local/bin`:

~~~~bash
sudo rm /usr/local/bin/iso-flasher
~~~~

If you only built a local copy in the repository, remove that binary with:

~~~~bash
rm -f iso-flasher
~~~~

---

## 📖 What is iso-flasher?

**iso-flasher** is a lightweight C-based ISO-to-USB flashing tool currently under active development.

The goal is simple:

> 💿 Flash an ISO.  
> ⚡ Make it fast.  
> 🧹 Keep it lightweight.  
> 🚫 No unnecessary bloat.

The project is still being built, and the terminal interface is currently being refined. Linux USB flashing is functional.

---

## ✨ Planned Features

### 💿 ISO Flashing

- ✅ ISO-to-USB raw block flashing
- ✅ Removable USB drive detection
- ✅ Drive selection
- ✅ Live flash progress and transfer speed
- ✅ Explicit `FLASH` confirmation before destructive writes
- ✅ ISO/USB size validation
- ✅ Final `fsync()` and block-device cache flush
- ✅ Partial-write handling

### 🔍 Automatic Detection

iso-flasher is designed to detect the package manager available on the host system.

### 📦 Dependency Handling

Planned support for utilities such as:

- `dd`
- `lsblk`
- `pv`

### 🎨 User Experience

- 🌈 Colored output
- 📋 Clear drive listings
- ⚡ Lightweight execution
- 🧹 Minimal dependencies

---

## 📦 Supported Package Managers

| Distribution / OS | Package Manager |
|---|---|
| 🐧 Debian / Ubuntu | `apt` |
| 🎩 Fedora / RHEL | `dnf` |
| 🏔️ Arch / Manjaro | `pacman` |
| 🦎 openSUSE | `zypper` |
| 🔲 Void Linux | `xbps` |
| 🏔️ Alpine Linux | `apk` |
| 🍎 macOS | `brew` |
| 😈 FreeBSD | `pkg` |
| 🧪 Gentoo | `emerge` |
| 📱 Termux | `pkg` **(Coming Soon)** |

> 🚧 **Termux support is coming soon.** Termux's `pkg` package manager has not been fully integrated or tested yet.

---

## 🖥️ Platform Support

| Platform | Status |
|---|---|
| 🐧 Linux | 🟢 Primary development target |
| 📱 Termux | 🟡 Coming soon |
| 🍎 macOS | 🟡 Not tested yet |
| 😈 BSD | 🟡 Not tested yet |
| 🪟 Windows | 🔴 Not currently supported AND NEVER WILL BE 🤣️ go switch to linux! |

> **Note:** macOS and BSD have not been tested yet. Compatibility may change as development continues.

---

## 🛠️ Requirements

Currently, development requires:

- A C compiler such as `gcc` or `clang`
- A supported Unix-like operating system

Check your compiler:

~~~~bash
gcc --version
~~~~

or:

~~~~bash
clang --version
~~~~

### 📱 Termux

**Termux support is coming soon.**

The project has not been fully adapted or tested for Termux yet.

---


---

## 🖥️ GUI

A graphical interface is also planned.

### 🚧 GUI COMING SOON

The future GUI is intended to make selecting ISO files, choosing USB drives, monitoring progress, and managing the flashing process easier.

~~~~text
┌─────────────────────────────┐
│        💿 iso-flasher       │
├─────────────────────────────┤
│                             │
│  ISO File                   │
│  [ Select ISO... ]          │
│                             │
│  USB Drive                  │
│  [ Select Drive... ]        │
│                             │
│  ┌───────────────────────┐  │
│  │     FLASH ISO         │  │
│  └───────────────────────┘  │
│                             │
│  Progress: ███████░░░ 70%   │
└─────────────────────────────┘
~~~~

---

## 📁 Project Structure

~~~~text
iso-flasher/
├── iso-flasher.c
├── .gitignore
├── LICENSE
├── README.md
└── SECURITY.md
~~~~

---

## 🧩 Development Status

| Component | Status |
|---|---|
| C foundation | 🟢 In development |
| CLI interface | 🟡 In development |
| ISO handling | 🟡 In development |
| USB detection | 🟢 Implemented |
| USB flashing | 🟢 Implemented on Linux |
| Progress reporting | 🟢 Implemented |
| GUI | 🟡 Coming soon |
| Termux support | 🟡 Coming soon |
| Termux `pkg` support | 🟡 Coming soon |
| Linux testing | 🟢 Primary target |
| macOS testing | ⚪ Not tested |
| BSD testing | ⚪ Not tested |

---

## 🗺️ Roadmap

- [x] Project foundation
- [ ] Complete core ISO handling
- [x] Implement USB detection
- [x] Implement USB flashing
- [x] Add confirmation system
- [x] Add progress reporting
- [x] Improve error handling
- [ ] Expand Linux support
- [ ] Add Termux support
- [ ] Add Termux `pkg` support
- [ ] Test macOS
- [ ] Test BSD
- [ ] Build GUI
- [ ] Test across supported systems
- [ ] First stable release

---

## 🛡️ Safety

iso-flasher works directly with storage devices.

**Always verify the selected drive before writing an ISO.**

Selecting the wrong drive can result in data loss.

---

## 🐛 Bug Reports

Found a bug or have an idea?

When opening an issue, include:

- Operating system
- Distribution/version
- C compiler and version
- Package manager
- What you were doing
- What happened
- Any terminal output or errors

If you're testing on **macOS, BSD, or Termux**, please mention it since these platforms are not fully tested yet.

---

## 🤝 Contributing

Contributions, ideas, testing, and improvements are welcome.

Especially useful right now:

- 🐧 Linux testing
- 📱 Termux testing once support is available
- 🍎 macOS testing
- 😈 BSD testing
- 💻 C development
- 🖥️ GUI development

To contribute:

1. Fork the repository
2. Create a branch
3. Make your changes
4. Test your changes
5. Commit your work
6. Push your branch
7. Open a Pull Request

---

## 🔐 Security

Please see [`SECURITY.md`](SECURITY.md) for security information and vulnerability reporting.

If you discover a security issue, please report it responsibly.

---

## 📜 License

iso-flasher is licensed under the **MIT License**.

See [`LICENSE`](LICENSE) for the complete license text.

---

## 👨‍💻 Author

**Ansh Bhatia — AnshLabs716**

Built with C and a hatred for unnecessary bloat. 🔥

---

## ⭐ Support

If you find iso-flasher useful:

- ⭐ Star the repository
- 🐛 Report bugs
- 💡 Suggest features
- 🔧 Contribute improvements
- 🖥️ Help test the upcoming GUI
- 📱 Help test future Termux support

---

<div align="center">

# 💿 iso-flasher

### ⚡ No bloat. No fluff. Just ISO flashing.

**⚡ Linux USB flashing is functional • GUI still planned**

**🖥️ GUI coming soon • 📱 Termux support coming soon**

**🍎 macOS & 😈 BSD not tested yet**

</div>
