# Stasis Architecture & Dependency Flow

## Module Dependency Graph

```
src/main.rs (event loop, terminal setup, CLI args)
  ├── src/app.rs (App state, tab navigation, action dispatch)
  │     ├── src/system/collector.rs (CPU, RAM, Swap, Disk, Network stats)
  │     ├── src/system/processes.rs (process list, kill/pause)
  │     ├── src/system/network.rs (ss parsing, socket map, port kill)
  │     ├── src/system/cleaner.rs (cache scan, deletion)
  │     ├── src/system/services.rs (systemctl wrapper)
  │     ├── src/system/autostart.rs (.desktop file parser)
  │     ├── src/system/applications.rs (APT/Pacman/Flatpak/Snap discovery)
  │     ├── src/system/gpu.rs (NVIDIA nvidia-smi / AMD+Intel DRM sysfs)
  │     ├── src/system/sensors.rs (sysinfo + /sys/class/thermal)
  │     └── src/system/sudo.rs (password validation, elevated commands)
  │
  ├── src/ui/mod.rs (render dispatcher, tab header, scrollable tabs)
  │     ├── src/ui/dashboard.rs (gauges, sparklines, health diagnostics)
  │     ├── src/ui/network.rs (socket table, traffic sparklines, filters)
  │     ├── src/ui/processes.rs (process table, top-3 sidebar)
  │     ├── src/ui/cleaner.rs (category toggles, scan results)
  │     ├── src/ui/services.rs (service table, scope toggle)
  │     ├── src/ui/autostart.rs (autostart table, add/remove)
  │     ├── src/ui/applications.rs (package table, uninstall)
  │     └── src/ui/modals.rs (help, confirm, sudo, toast, new entry)
  │
  ├── src/theme.rs (5 palettes: Cyberpunk, Dracula, Nord, Monokai, Gruvbox)
  └── src/gui.rs (GTK3/VTE native window via dlopen, zero Python)
```

## Crate Dependencies

```
stasis
  ├── ratatui 0.29 ─── TUI rendering (widgets, layout, text)
  ├── crossterm 0.28 ── Terminal backend (raw mode, events, mouse, alternate screen)
  ├── sysinfo 0.33 ──── CPU, RAM, Swap, Disk, Processes, Networks, Sensors
  ├── libc 0.2 ──────── POSIX APIs (kill, geteuid, dlopen, dlsym)
  ├── walkdir 2.5 ───── Recursive directory traversal for cleaner scanning
  └── anyhow 1.0 ────── Error propagation in main.rs

  [removed] chrono, serde, serde_json were unused
```

## Data Flow Per Tick (1s interval)

```
1. App::tick()
   ├── SystemCollector::refresh()
   │     ├── sysinfo::System::refresh_all()
   │     ├── CPU per-core usage → history deque (60 entries)
   │     ├── RAM/Swap usage → history deque (60 entries)
   │     ├── Network ingress/egress totals → history deque
   │     ├── Per-interface stats → interface_stats (unused in UI)
   │     └── Disk I/O totals
   │
   ├── ProcessManager::refresh()      [if Processes tab active]
   │     ├── sysinfo::System::refresh_processes()
   │     └── Build ProcessItem list (name, pid, cpu%, mem, status, disk)
   │
   ├── NetworkManager::refresh()
   │     ├── Run `ss -tulnpa` → parse lines → SocketEntry list
   │     └── Compute per-app traffic breakdown
   │
   ├── ServiceManager::refresh()      [if Services tab active]
   │     ├── Run `systemctl list-units --type=service --all`
   │     └── Parse into ServiceItem list
   │
   ├── ApplicationManager::refresh()  [on first load or manual refresh]
   │     ├── APT: dpkg-query -W -f '...'
   │     ├── Pacman: pacman -Q
   │     ├── Flatpak: flatpak list --columns=...
   │     ├── Snap: snap list
   │     └── Desktop: scan /usr/share/applications + ~/.local/share/applications
   │
   ├── AutostartManager::refresh()    [on first load]
   │     └── Scan /etc/xdg/autostart + ~/.config/autostart for .desktop files
   │
   ├── GpuCollector::refresh()
   │     ├── NVIDIA: run `nvidia-smi --query-gpu=...`
   │     └── AMD/Intel: read /sys/class/drm/card*/device/gpu_busy_percent
   │
   └── SensorCollector::refresh()
         ├── sysinfo::Components::refresh()
         └── Fallback: /sys/class/thermal/zone*/temp

2. Terminal::draw() → ui::render()
   ├── render_tabs_header()    (scrollable tab bar with «/» indicators)
   ├── render_dashboard()      (gauges, sparklines, GPU, disk, thermal, health)
   ├── render_network()        (sparklines, socket table, filters)
   ├── render_processes()      (process table, top-3 sidebar, search)
   ├── render_cleaner()        (category grid, scan results, size totals)
   ├── render_services()       (service table, details sidebar, scope toggle)
   ├── render_autostart()      (autostart table, details sidebar)
   ├── render_applications()   (package table, details sidebar, source filter)
   └── render_modals()         (help, confirm, sudo password, toast, new entry)
```

## Event Flow (Input Handling)

```
crossterm::event::read()
  │
  ├── Event::Key → handle_key_event()
  │     ├── InputMode::Normal → global bindings (Tab, arrows, 1-7, t, r, ?)
  │     ├── InputMode::Normal → tab-specific bindings (j/k, s, f, /, etc.)
  │     ├── InputMode::Search → search input filtering
  │     ├── InputMode::HelpModal → dismiss on Esc/Enter
  │     ├── InputMode::ConfirmModal → Y/N/Enter
  │     ├── InputMode::SudoPasswordModal → password input + validate
  │     └── InputMode::NewAutostartModal → name/exec/comment fields
  │
  └── Event::Mouse → handle_mouse_event()
        ├── Modal click → dismiss/confirm
        ├── Tab header click → switch tab (pixel-accurate)
        ├── Tab header scroll → switch tab (ScrollUp=prev, ScrollDown=next)
        ├── List scroll → navigate within current tab
        └── Row click → select table row
```

## GUI Mode (GTK3/VTE)

```
stasis -g
  └── gui::launch_desktop_window()
        ├── dlopen("libgtk-3.so.0") → GTK3 functions
        ├── dlopen("libvte-2.91.so.0") → VTE terminal functions
        ├── dlopen("libgdk-3.so.0") → Wayland/X11 detection
        ├── Create GtkWindow (1180x750, dark CSS theme)
        ├── Create VteTerminal widget
        ├── Resolve stasis binary full path via env::current_exe()
        ├── Spawn `stasis -i` inside VTE (child process inherits TUI)
        ├── Connect "child-exited" → gtk_main_quit
        └── gtk_main() event loop

  Terminal fallback chain (5 attempts):
    1. Alacritty    → alacritty -e stasis -i
    2. Kitty        → kitty stasis -i
    3. WezTerm      → wezterm start stasis -i
    4. foot         → foot stasis -i
    5. xterm        → xterm -e stasis -i
```
