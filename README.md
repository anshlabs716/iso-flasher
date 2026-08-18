# 💿 iso-flasher

<div align="center">

### ⚡ A Powerful, No-Bloat ISO-to-USB Flasher

**Fast • Lightweight • Written in C**

[![C](https://img.shields.io/badge/C-100%25-A8B9CC?style=for-the-badge&logo=c&logoColor=black)](https://en.wikipedia.org/wiki/C_(programming_language))
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20BSD-1793D1?style=for-the-badge)](https://github.com/anshlabs716/iso-flasher)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](https://opensource.org/license/mit/)

## 🚧 STILL IN DEVELOPMENT 🚧

**iso-flasher is currently in development and cannot flash USB drives yet.**

**GUI coming soon!**

</div>

---

## 📖 What is iso-flasher?

**iso-flasher** is a lightweight C-based ISO-to-USB flashing tool currently under active development.

The goal is simple:

> 💿 Flash an ISO.  
> ⚡ Make it fast.  
> 🧹 Keep it lightweight.  
> 🚫 No unnecessary bloat.

The project is still being built, so **USB flashing is not functional yet**. Features and functionality may change as development continues.

---

## ✨ Planned Features

### 💿 ISO Flashing

- 🚧 ISO-to-USB flashing
- 🚧 USB drive detection
- 🚧 Drive selection
- 🚧 Flash progress
- 🚧 Confirmation before writing

### 🔍 Automatic Detection

Planned support for package managers including:

- `apt`
- `dnf`
- `pacman`
- `zypper`
- `xbps`
- `apk`
- `brew`
- `pkg`
- `emerge`

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

> 🚧 These targets are planned. Compatibility may change during development.

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

---

## ⚡ Build

### 1. Clone the repository

~~~~bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
~~~~

### 2. Compile

Using GCC:

~~~~bash
gcc -o iso-flasher iso-flasher.c
~~~~

### 3. Run

~~~~bash
./iso-flasher
~~~~

> ⚠️ **Development notice:** The current build cannot flash USB drives yet. The flashing functionality is still being implemented.

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
| USB detection | 🟡 In development |
| USB flashing | 🔴 Not implemented yet |
| Progress reporting | 🔴 Planned |
| GUI | 🔴 Coming soon |
| Cross-platform support | 🟡 Planned |

---

## 🗺️ Roadmap

- [x] Project foundation
- [ ] Complete core ISO handling
- [ ] Implement USB detection
- [ ] Implement USB flashing
- [ ] Add confirmation system
- [ ] Add progress reporting
- [ ] Improve error handling
- [ ] Expand platform support
- [ ] Build GUI
- [ ] Test across supported systems
- [ ] First stable release

---

## 🛡️ Safety

Once USB flashing is implemented, iso-flasher will work directly with storage devices.

**Always verify the selected drive before writing an ISO.**

Selecting the wrong drive can result in data loss.

---

## 🐛 Bug Reports

Found a bug or have an idea?

When opening an issue, include:

- Operating system
- Distribution/version
- C compiler and version
- What you were doing
- What happened
- Any terminal output or errors

---

## 🤝 Contributing

Contributions, ideas, and improvements are welcome.

To contribute:

1. Fork the repository
2. Create a branch
3. Make your changes
4. Test your changes
5. Commit your work
6. Push the branch
7. Open a Pull Request

---

## 🔐 Security

Please see [`SECURITY.md`](SECURITY.md) for security information and vulnerability reporting.

---

## 📜 License

iso-flasher is licensed under the **MIT License**.

See [`LICENSE`](LICENSE) for the complete license text.

---

## 👨‍💻 Author

**Ansh Bhatia — AnshLabs716**

Built with C and a hatred for unnecessary bloat. 🔥

---

<div align="center">

# 💿 iso-flasher

### ⚡ No bloat. No fluff. Just ISO flashing.

**🚧 In Development • USB flashing not available yet • GUI coming soon**

</div>
