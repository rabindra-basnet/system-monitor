# Stasis Known Issues

## Critical

### Process Disk I/O Columns Are Misleading
**File:** `src/ui/processes.rs:393-395`
The columns labeled "Disk R/s" and "Disk W/s" display cumulative bytes since process start (from `sysinfo::Process::disk_usage()`), not per-second rates. Users expect rate values.

**Fix:** Store previous tick's disk_usage values in `ProcessItem`, compute delta per refresh interval.

---

### RPM Support Claimed But Not Implemented
**File:** `src/system/applications.rs:673-700`
README and `is_system_essential_package()` list RPM as a supported package format, but `ApplicationManager::refresh()` never calls `rpm -qa` to discover packages.

**Fix:** Add `Command::new("rpm").args(["-qa", "--queryformat", ...])` to `refresh()`.

---

## Medium

### Stale "sysmon-tui" References
**File:** `src/ui/cleaner.rs:260`
Help text says "Run sysmon-tui with 'sudo'" — should say "Run stasis". The old project name also appears in `install.sh:103` and `src/system/applications.rs:470,545` (filtering sysmon-tui from app list, which is correct but vestigial).

---

### AMD/Intel GPU Data Incomplete
**File:** `src/system/gpu.rs:141-167`
The sysfs fallback path for non-NVIDIA GPUs only reads `gpu_busy_percent`. VRAM usage, temperature, and power are hardcoded to 0.

**Fix:** Read from `/sys/class/drm/card*/device/mem_info_vram_used`, `/sys/class/hwmon/*/temp1_input`, `/sys/class/hwmon/*/power1_average`.

---

### AMD/Intel GPU History Not Updated
**File:** `src/system/gpu.rs:57-62`
GPU history deque is created once on first refresh but never pushed to on subsequent calls. Only NVIDIA path updates history.

---

### Unused Computed Data
- `SystemCollector::interface_stats` — computed every tick but never rendered in any UI
- `NetworkSummary::top_network_processes` — computed but never displayed
- `Theme::border_active` — defined in all 5 themes but never referenced
- `SensorItem::max` and `SensorItem::critical` — collected but never shown

---

### Missing Test Coverage
No tests for: `autostart.rs`, `services.rs`, `sudo.rs`, `network.rs` parsing, `applications.rs` uninstall logic, clipboard functionality, GUI launch path.

---

## Low

### Hardcoded Values
| File | Value | Impact |
|------|-------|--------|
| `gui.rs:249` | CSS colors `#0d1117`, `#c9d1d9` | GUI doesn't adapt to TUI theme |
| `gui.rs:202` | Window size `1180x750` | Not configurable |
| `network.rs:157` | PID <= 1000 = system process | Magic number |
| `processes.rs:58` | PID < 1000 = critical | Same magic number |

---

### install.sh Gaps
- No check for `ss` command (network tab depends on it)
- No check for `systemctl` (services tab depends on it)
- No check for `fuser` (port kill fallback depends on it)
- No MSRV validation (claims 1.75+ but doesn't verify)
- Doesn't verify install dir is in `$PATH`

---

### README Inaccuracies
- "12 integration tests" — these are unit tests, not integration tests
- "Binary Size: 1.2MB" — unverified, depends on build
- "RAM Footprint: <15MB" — unverified, no measurement tooling

---

### CI/CD Gaps
- No `cargo audit` for dependency vulnerabilities
- No MSRV testing (only tests on latest stable)
- Release workflow doesn't run tests before packaging
- No ARM64 testing on real hardware
- No cross-platform testing (Fedora, Arch, non-systemd)
