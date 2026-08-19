# Stasis Roadmap

## Current Status: v0.2.0

7 tabs fully functional, 5-theme engine, full mouse support, native GTK3/VTE GUI, APT repository with GPG signing.

---

## Phase 1 — Quality & Correctness (v0.3.0)

Fix existing issues before adding new features.

- [ ] Remove unused dependencies (`chrono`, `serde`, `serde_json`)
- [ ] Fix misleading process disk I/O columns — label shows "R/s" but value is cumulative bytes
- [ ] Add RPM package discovery in `ApplicationManager::refresh()`
- [ ] Fix stale "sysmon-tui" references in cleaner help text
- [ ] Display `SensorItem::max` and `SensorItem::critical` in thermal sensors UI
- [ ] Display `NetworkSummary::top_network_processes` in Network tab sidebar
- [ ] Remove unused `Theme::border_active` field or use it for active tab highlight
- [ ] Remove unused `SystemCollector::interface_stats` or render it in Dashboard
- [ ] Delete empty `bin/` directory from repo
- [ ] Fix README: "12 integration tests" should be "12 unit tests"
- [ ] Fix README: remove RPM claim or implement it

## Phase 2 — Test Coverage (v0.3.0)

- [ ] Unit tests for `autostart.rs` (toggle, add, remove, parse_desktop_file)
- [ ] Unit tests for `services.rs` (filtered_services, systemctl parsing)
- [ ] Unit tests for `network.rs` (kill_port, ss parsing, filter modes)
- [ ] Unit tests for `applications.rs` (uninstall_app, parse_size_string, is_system_essential)
- [ ] Unit tests for `sudo.rs` (validate_sudo_password)
- [ ] Integration tests for event loop and modal flows
- [ ] CI: add `cargo audit` for dependency vulnerability scanning
- [ ] CI: test against MSRV (Rust 1.75)
- [ ] Release workflow: run tests before packaging

## Phase 3 — New Features (v0.4.0)

- [ ] **Disk Analyzer** — tree visualization of disk usage by directory (like ncdu)
- [ ] **GPU History** — extend AMD/Intel sysfs path to read VRAM, temperature, power
- [ ] **GPU History for AMD/Intel** — push GPU history on each refresh cycle
- [ ] **Per-Process Disk I/O** — compute actual R/s and W/s deltas between ticks
- [ ] **System Info Tab** — dedicated hardware info (CPU model, RAM slots, disk models, BIOS)
- [ ] **Keyboard shortcut help per tab** — show relevant shortcuts in each tab's empty state

## Phase 4 — Advanced Features (v0.5.0)

- [ ] **APT Repository Manager** — UI to add/remove PPAs and third-party repos
- [ ] **Firewall Management** — UFW/nftables rule viewing and toggling
- [ ] **System Tweaks** — sysctl parameter tuning (swap, vm, net)
- [ ] **Kernel Module Management** — lsmod/modprobe/rmmod UI
- [ ] **User/Group Management** — list users, modify groups
- [ ] **Scheduled Tasks** — crontab and systemd timer viewer/editor

## Phase 5 — Polish & Distribution (v1.0.0)

- [ ] Config file support (`~/.config/stasis/config.toml`)
- [ ] Persist user preferences (theme, last tab, sort order)
- [ ] Flatpak package
- [ ] AUR package
- [ ] Snap package
- [ ] Homebrew formula
- [ ] man page
- [ ] Shell completions (bash, zsh, fish)
- [ ] Localized UI strings (i18n)
- [ ] Wayland-native clipboard (wl-copy detection in install.sh)
