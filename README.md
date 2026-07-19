# 💿 usb-maker

**A powerful, no-bloat, no-fluff CLI tool to flash ISO files to USB drives.**

By AnshLabs716 🔥

---

## 📖 **What is iso-flasher?**

iso-flasher is a **lightweight C program** that writes ISO files to USB drives. It auto-detects your package manager, installs dependencies, and gets the job done with zero nonsense.

No GUI. No bloat. Just **pure C power**.

---

## ✨ **Features**

- 🖥️ **CLI-only** — No GUI, just terminal
- 🔍 **Auto-detects** your package manager (apt, dnf, pacman, brew, etc.)
- 📦 **Installs dependencies** automatically (dd, lsblk, pv)
- 🚀 **Fast ISO writing** with progress
- 🛡️ **Safe** — Asks for confirmation before writing
- 📋 **Lists USB drives** before selection
- 🎨 **Colored output** — Easy to read

---

## 📦 **Supported Package Managers**

| OS | Package Manager |
|----|-----------------|
| Debian/Ubuntu | apt |
| Fedora/RHEL | dnf |
| Arch/Manjaro | pacman |
| openSUSE | zypper |
| Void Linux | xbps |
| Alpine Linux | apk |
| macOS | brew |
| FreeBSD | pkg |
| Gentoo | emerge |

---

## ⚡ **Quick Install**

### **1. Clone the repo**
```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd usb-maker

compile it
gcc -o usb-maker usb-maker.c

run it
./usb-maker
