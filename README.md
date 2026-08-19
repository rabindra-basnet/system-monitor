# Stasis 󱐋

> **A blazing-fast, lightweight Linux System Monitor, Optimizer & App Uninstaller in Rust & GTK3 (Stacer Alternative).**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux-green.svg)](https://www.kernel.org/)
[![Binary Size](https://img.shields.io/badge/Binary%20Size-1.2MB-blueviolet.svg)](#features)
[![Memory](https://img.shields.io/badge/RAM%20Footprint-%3C15MB-brightgreen.svg)](#features)

---

## ⚡ Purpose & Philosophy

**Stasis** (Greek: *στάσις* — *standing firm, state of perfect equilibrium & stability*) is designed to actively monitor, stabilize, and restore peak performance to your Linux workstation or server. It provides a lightweight, instant-launching alternative to GUI tools like **Stacer**, packing resource graphs, process management, multi-category disk cleaning, systemd service control, autostart management, package uninstallation, and hardware telemetry into an ultra-lean **1.2 MB** binary with pure desktop single-window GUI and terminal modes.

---

## 🚀 Key Modules (Stacer Suite)

1. **󰨇 Dashboard & Hardware Resources**: Real-time CPU, RAM, Swap, Disk gauges, per-core breakdown, network traffic sparklines, hardware thermal sensors, and 60-second time-series history graphs.
2. ** Process Manager**: Interactive process inspection, multi-column sorting, live search filter, protected system process dimming, and safe signals (`SIGKILL`, `SIGTERM`, `SIGSTOP`, `SIGCONT`).
3. **󰃢 System Cleaner**: Multi-category cache cleaner (Package Caches, `~/.cache`, Thumbnails, Crash dumps, System Logs, Trash) with safe user cache defaults and elevated root wiping.
4. ** systemd Services**: Manage system-wide & user (`--user`) service units (Start, Stop, Restart, Enable, Disable) with state filters.
5. **󱑞 Startup Applications**: Toggle enabled/disabled status of desktop autostart apps, create new `.desktop` entries, or remove entries.
6. **󰏖 Applications & Package Uninstaller**: Discover installed packages across APT, Pacman, RPM, Flatpak, Snap, and Desktop apps with size inspection, safety protection locks, and removal.
7. **🔒 System Safety Locks & Sudo Authentication**: Hard locks prevent accidental uninstallation of critical OS packages or termination of PID 1, with in-app masked sudo password elevation.
8. **🎨 Theme Engine**: 5 built-in palettes (**Cyberpunk**, **Dracula**, **Nord**, **Monokai**, **Gruvbox**).
9. **🖱️ Full Mouse Support**: Seamless tab switching, scroll wheel list navigation, search focus, cleaner toggles, and modal button clicks.

---

## ⌨️ Keyboard & Mouse Controls

| Key / Mouse | Scope | Action |
| :--- | :--- | :--- |
| `1` - `6` / `F1` - `F6` | Global | Switch directly to tabs 1 through 6 |
| `Left Click` (Header) | Global | Switch tabs directly by clicking tab title |
| `Tab` / `Shift+Tab` | Global | Cycle forwards / backwards through tabs |
| `Scroll Wheel` | Lists | Scroll through processes, services, apps, and autostart entries |
| `t` | Global | Cycle color theme (Cyberpunk → Dracula → Nord → Monokai → Gruvbox) |
| `r` | Global | Force immediate telemetry refresh |
| `?` | Global | Toggle Help & Shortcuts modal |
| `q` / `Ctrl+C` | Global | Quit application cleanly |
| `j` / `k` or `↓` / `↑` | Navigation | Move cursor / selection down and up |
| `/` | Search | Open live substring search (Processes, Services, Autostart, Apps) |
| `s` / `d` | Processes | Cycle sort column (`s`) / Invert sort order (`d`) |
| `K` / `x` / `Del` | Processes | Kill process (`SIGKILL` with confirmation) |
| `t` | Processes | Terminate process (`SIGTERM` with confirmation) |
| `p` / `c` | Processes | Pause (`SIGSTOP`) / Continue (`SIGCONT`) |
| `Space` / `Click` | Cleaner | Toggle selection checkbox for active cache category |
| `a` | Cleaner | Select All / Deselect All categories |
| `s` | Cleaner | Scan filesystem paths for reclaimable sizes |
| `c` / `Enter` | Cleaner | Clean selected categories (prompts sudo if needed) |
| `u` | Services | Toggle between System and User service units |
| `f` | Services | Cycle filter state (`All` → `Active` → `Inactive` → `Failed`) |
| `s` / `x` / `r` | Services | Start / Stop / Restart service unit |
| `e` / `d` | Services | Enable / Disable service unit at boot |
| `n` | Autostart | Add new startup application |
| `d` / `Delete` | Autostart | Delete selected user startup entry |
| `u` / `Delete` | Applications | Uninstall / remove selected package |
| `f` | Applications | Cycle package source filter (All → System → Desktop → Flatpak → Snap) |
| `s` / `d` | Applications | Cycle sort (Size → Name → Source) / Invert direction |

---

## 🛠️ Build & Installation

### Quick Install Script
```bash
git clone https://github.com/rabindra-basnet/system-monitor.git
cd system-monitor
./install.sh
```

---

## 🚀 Running Stasis

```bash
# Launch directly in your current terminal:
stasis

# Launch in pure single-window Desktop GUI mode:
stasis -g

# Open directly into a specific tab:
stasis -p        # Process Manager
stasis -c        # System Cleaner
stasis -a        # Uninstaller / Applications
stasis -s        # systemd Services

# Run non-interactive telemetry diagnostic check:
stasis --test
```

---

## 📄 License

MIT License — Copyright (c) 2026 Rabindra Basnet.
