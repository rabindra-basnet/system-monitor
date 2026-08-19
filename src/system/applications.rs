use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq)]
pub enum AppSourceFilter {
    All,
    UserInstalled,
    InitialOS,
    DesktopOnly,
    Flatpak,
    Snap,
}

impl AppSourceFilter {
    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::UserInstalled,
            Self::UserInstalled => Self::InitialOS,
            Self::InitialOS => Self::DesktopOnly,
            Self::DesktopOnly => Self::Flatpak,
            Self::Flatpak => Self::Snap,
            Self::Snap => Self::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All Packages",
            Self::UserInstalled => "User Installed (New)",
            Self::InitialOS => "Initial OS Installs",
            Self::DesktopOnly => "Desktop Apps",
            Self::Flatpak => "Flatpak",
            Self::Snap => "Snap",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppSortBy {
    Size,
    Age,
    Name,
    Source,
}

#[derive(Clone, Debug)]
pub struct ApplicationItem {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub description: String,
    pub source: String,
    pub package_id: String,
    pub is_essential: bool,
    pub is_initial_install: bool,
    pub installed_time: Option<u64>,
}

pub struct ApplicationManager {
    pub items: Vec<ApplicationItem>,
    pub search_query: String,
    pub source_filter: AppSourceFilter,
    pub sort_by: AppSortBy,
    pub sort_descending: bool,
    pub is_loading: bool,
}

pub fn format_installation_age(timestamp: Option<u64>, is_initial: bool) -> String {
    if is_initial {
        return "Initial OS".to_string();
    }
    let Some(ts) = timestamp else {
        return "—".to_string();
    };

    let now = match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => return "—".to_string(),
    };

    if ts > now {
        return "Just now".to_string();
    }

    let diff_secs = now - ts;
    let days = diff_secs / 86400;
    let months = days / 30;
    let years = days / 365;

    if years >= 1 {
        let rem_months = (days % 365) / 30;
        if rem_months > 0 {
            format!("{}y {}mo ago", years, rem_months)
        } else {
            format!("{}y ago", years)
        }
    } else if months >= 1 {
        let rem_days = days % 30;
        if rem_days > 0 {
            format!("{}mo {}d ago", months, rem_days)
        } else {
            format!("{}mo ago", months)
        }
    } else if days >= 1 {
        format!("{}d ago", days)
    } else {
        let hours = diff_secs / 3600;
        if hours >= 1 {
            format!("{}h ago", hours)
        } else {
            let mins = diff_secs / 60;
            if mins >= 1 {
                format!("{}m ago", mins)
            } else {
                "Just now".to_string()
            }
        }
    }
}

pub fn is_system_essential_package(name: &str) -> bool {
    let lower = name.to_lowercase();
    // 1. Core OS & Kernel & Init
    lower == "base-files"
        || lower == "base-passwd"
        || lower.starts_with("systemd")
        || lower.starts_with("libc6")
        || lower.starts_with("glibc")
        || lower.starts_with("coreutils")
        || lower == "dpkg"
        || lower == "apt"
        || lower == "aptitude"
        || lower == "pacman"
        || lower == "rpm"
        || lower == "dnf"
        || lower == "zypper"
        || lower.starts_with("linux-image")
        || lower.starts_with("linux-headers")
        || lower.starts_with("linux-modules")
        || lower.starts_with("linux-firmware")
        || lower.starts_with("grub")
        || lower.starts_with("shim")
        || lower.starts_with("efibootmgr")
        || lower.starts_with("udev")
        || lower.starts_with("util-linux")
        || lower.starts_with("findutils")
        || lower.starts_with("libgcc")
        || lower.starts_with("libstdc++")
        // 2. Shell & Authentication & Security
        || lower == "bash"
        || lower == "dash"
        || lower == "sh"
        || lower == "login"
        || lower == "passwd"
        || lower == "shadow"
        || lower == "sudo"
        || lower == "dbus"
        || lower.starts_with("dbus-")
        || lower.starts_with("polkit")
        || lower.starts_with("policykit")
        || lower.starts_with("libpam")
        || lower.starts_with("openssl")
        || lower.starts_with("libssl")
        || lower == "ca-certificates"
        // 3. Desktop Environment & Window Manager & Graphics Core
        || lower.starts_with("gnome-shell")
        || lower.starts_with("gnome-session")
        || lower.starts_with("gnome-control-center")
        || lower.starts_with("gdm")
        || lower.starts_with("lightdm")
        || lower.starts_with("sddm")
        || lower.starts_with("xorg")
        || lower.starts_with("xserver-xorg")
        || lower.starts_with("wayland")
        || lower.starts_with("mutter")
        || lower.starts_with("plasma-desktop")
        || lower.starts_with("xfce4-session")
        || lower.starts_with("pipewire")
        || lower.starts_with("pulseaudio")
        || lower.starts_with("alsa-")
        || lower.starts_with("mesa-")
        || lower.starts_with("libgtk-")
        || lower.starts_with("libgl1")
        // 4. Networking Core
        || lower.starts_with("network-manager")
        || lower.starts_with("netplan")
        || lower.starts_with("wpasupplicant")
        || lower.starts_with("iproute2")
        || lower.starts_with("openssh-server")
}

impl Default for ApplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            items: Vec::new(),
            search_query: String::new(),
            source_filter: AppSourceFilter::All,
            sort_by: AppSortBy::Size,
            sort_descending: true,
            is_loading: false,
        };
        mgr.refresh();
        mgr
    }

    pub fn refresh(&mut self) {
        self.is_loading = true;
        let mut all_apps = Vec::new();

        // Fast index of DPKG mtimes for install timestamp and initial install detection
        let mut dpkg_mtimes: HashMap<String, u64> = HashMap::new();
        let mut initial_os_ts: Option<u64> = None;

        if let Ok(entries) = fs::read_dir("/var/lib/dpkg/info") {
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let s = fname.to_string_lossy();
                if s.ends_with(".list") {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            if let Ok(dur) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                                let secs = dur.as_secs();
                                let clean = s.trim_end_matches(".list");
                                let base = clean.split(':').next().unwrap_or(clean);
                                dpkg_mtimes.insert(base.to_string(), secs);

                                if base == "ubuntu-minimal"
                                    || base == "base-files"
                                    || base == "ubuntu-standard"
                                {
                                    initial_os_ts = match initial_os_ts {
                                        Some(old) => Some(old.min(secs)),
                                        None => Some(secs),
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }

        // 1. Try Debian/Ubuntu dpkg-query with Essential and Priority tags
        if let Ok(output) = Command::new("dpkg-query")
            .args(["-W", "-f=${binary:Package}\t${Version}\t${Installed-Size}\t${binary:Summary}\t${Essential}\t${Priority}\n"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 4 {
                        let name = parts[0].trim().to_string();
                        let version = parts[1].trim().to_string();
                        let raw_size: u64 = parts[2].trim().parse().unwrap_or(0);
                        let desc = parts[3].trim().to_string();
                        let essential_field = if parts.len() >= 5 { parts[4].trim() } else { "no" };
                        let priority_field = if parts.len() >= 6 { parts[5].trim() } else { "optional" };

                        let size_bytes = if raw_size > 5_000_000 {
                            raw_size
                        } else {
                            raw_size * 1024
                        };

                        let is_essential = essential_field == "yes"
                            || priority_field == "required"
                            || priority_field == "important"
                            || is_system_essential_package(&name);

                        let installed_time = dpkg_mtimes.get(&name).copied();
                        let is_initial_install = if let (Some(mtime), Some(init_ts)) = (installed_time, initial_os_ts) {
                            (mtime as i64 - init_ts as i64).abs() < 86400 * 2
                        } else {
                            is_essential
                        };

                        all_apps.push(ApplicationItem {
                            name: name.clone(),
                            version,
                            size_bytes,
                            description: desc,
                            source: "APT".to_string(),
                            package_id: name,
                            is_essential,
                            is_initial_install,
                            installed_time,
                        });
                    }
                }
            }
        }

        // 2. Try Arch pacman if APT wasn't found
        if all_apps.is_empty() {
            if let Ok(output) = Command::new("pacman").args(["-Qie"]).output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let mut cur_name = String::new();
                    let mut cur_version = String::new();
                    let mut cur_desc = String::new();
                    let mut cur_size = 0u64;

                    for line in text.lines() {
                        if line.starts_with("Name") {
                            if !cur_name.is_empty() {
                                let is_essential = is_system_essential_package(&cur_name);
                                all_apps.push(ApplicationItem {
                                    name: cur_name.clone(),
                                    version: cur_version.clone(),
                                    size_bytes: cur_size,
                                    description: cur_desc.clone(),
                                    source: "Pacman".to_string(),
                                    package_id: cur_name.clone(),
                                    is_essential,
                                    is_initial_install: is_essential,
                                    installed_time: None,
                                });
                            }
                            cur_name = line.split(':').nth(1).unwrap_or("").trim().to_string();
                            cur_version.clear();
                            cur_desc.clear();
                            cur_size = 0;
                        } else if line.starts_with("Version") {
                            cur_version = line.split(':').nth(1).unwrap_or("").trim().to_string();
                        } else if line.starts_with("Description") {
                            cur_desc = line.split(':').nth(1).unwrap_or("").trim().to_string();
                        } else if line.starts_with("Installed Size") {
                            let sz_str = line.split(':').nth(1).unwrap_or("").trim();
                            cur_size = Self::parse_size_string(sz_str);
                        }
                    }
                    if !cur_name.is_empty() {
                        let is_essential = is_system_essential_package(&cur_name);
                        all_apps.push(ApplicationItem {
                            name: cur_name.clone(),
                            version: cur_version,
                            size_bytes: cur_size,
                            description: cur_desc,
                            source: "Pacman".to_string(),
                            package_id: cur_name,
                            is_essential,
                            is_initial_install: is_essential,
                            installed_time: None,
                        });
                    }
                }
            }
        }

        // 3. Try Flatpak list
        if let Ok(output) = Command::new("flatpak")
            .args(["list", "--app", "--columns=name,application,version,size"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].trim().to_string();
                        let app_id = parts[1].trim().to_string();
                        let version = if parts.len() > 2 {
                            parts[2].trim().to_string()
                        } else {
                            "latest".to_string()
                        };
                        let size_bytes = if parts.len() > 3 {
                            Self::parse_size_string(parts[3].trim())
                        } else {
                            0
                        };

                        let is_essential = is_system_essential_package(&name);
                        all_apps.push(ApplicationItem {
                            name: name.clone(),
                            version,
                            size_bytes,
                            description: format!("Flatpak Application: {}", app_id),
                            source: "Flatpak".to_string(),
                            package_id: app_id,
                            is_essential,
                            is_initial_install: false,
                            installed_time: None,
                        });
                    }
                }
            }
        }

        // 4. Try Snap list
        if let Ok(output) = Command::new("snap").args(["list"]).output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines().skip(1) {
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.len() >= 3 {
                        let name = fields[0].to_string();
                        let version = fields[1].to_string();
                        let is_essential = is_system_essential_package(&name)
                            || name == "core"
                            || name == "snapd"
                            || name.starts_with("core2");

                        all_apps.push(ApplicationItem {
                            name: name.clone(),
                            version,
                            size_bytes: 0,
                            description: format!("Snap Package: {}", name),
                            source: "Snap".to_string(),
                            package_id: name,
                            is_essential,
                            is_initial_install: false,
                            installed_time: None,
                        });
                    }
                }
            }
        }

        // 5. Index Desktop applications (.desktop files)
        let desktop_dirs = [
            dirs_next_desktop(),
            Some(Path::new("/usr/local/share/applications").to_path_buf()),
            Some(Path::new("/usr/share/applications").to_path_buf()),
        ];

        for dir_opt in desktop_dirs.into_iter().flatten() {
            if let Ok(entries) = fs::read_dir(dir_opt) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "desktop") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let mut name = String::new();
                            let mut exec = String::new();
                            let mut comment = String::new();
                            let mut version = "1.0".to_string();
                            let mut is_nodisplay = false;

                            for line in content.lines() {
                                let trimmed = line.trim();
                                if trimmed == "[Desktop Action" {
                                    break;
                                }
                                if trimmed.starts_with("Name=") && name.is_empty() {
                                    name = trimmed.trim_start_matches("Name=").to_string();
                                } else if trimmed.starts_with("Exec=") && exec.is_empty() {
                                    exec = trimmed.trim_start_matches("Exec=").to_string();
                                } else if trimmed.starts_with("Comment=") && comment.is_empty() {
                                    comment = trimmed.trim_start_matches("Comment=").to_string();
                                } else if trimmed.starts_with("Version=") {
                                    version = trimmed.trim_start_matches("Version=").to_string();
                                } else if trimmed.starts_with("NoDisplay=true") {
                                    is_nodisplay = true;
                                }
                            }

                            if is_nodisplay
                                || name.is_empty()
                                || name.to_lowercase() == "stasis"
                                || name.to_lowercase() == "sysmon-tui"
                                || name.to_lowercase() == "vim"
                            {
                                continue;
                            }

                            let app_name_lower = name.to_lowercase();
                            let exists = all_apps.iter().any(|a: &ApplicationItem| {
                                a.name.to_lowercase() == app_name_lower
                                    || a.package_id.to_lowercase() == app_name_lower
                            });

                            if !exists {
                                let mut size_bytes = 0u64;
                                let exec_cmd = exec.split_whitespace().next().unwrap_or("");
                                if let Ok(which_out) = Command::new("which").arg(exec_cmd).output()
                                {
                                    if which_out.status.success() {
                                        let bin_path_str =
                                            String::from_utf8_lossy(&which_out.stdout)
                                                .trim()
                                                .to_string();
                                        if let Ok(meta) = fs::metadata(&bin_path_str) {
                                            size_bytes = meta.len();
                                        }
                                    }
                                }
                                if size_bytes == 0 {
                                    if let Ok(meta) = fs::metadata(exec_cmd) {
                                        size_bytes = meta.len();
                                    }
                                }

                                let is_essential = is_system_essential_package(&name);
                                let mut installed_time = None;
                                if let Ok(meta) = fs::metadata(&path) {
                                    if let Ok(mod_time) = meta.modified() {
                                        if let Ok(dur) =
                                            mod_time.duration_since(SystemTime::UNIX_EPOCH)
                                        {
                                            installed_time = Some(dur.as_secs());
                                        }
                                    }
                                }
                                let is_initial_install = if let (Some(mtime), Some(init_ts)) =
                                    (installed_time, initial_os_ts)
                                {
                                    (mtime as i64 - init_ts as i64).abs() < 86400 * 2
                                } else {
                                    false
                                };

                                all_apps.push(ApplicationItem {
                                    name: name.clone(),
                                    version,
                                    size_bytes,
                                    description: if comment.is_empty() {
                                        format!("Desktop application: {}", name)
                                    } else {
                                        comment
                                    },
                                    source: "Desktop".to_string(),
                                    package_id: path.display().to_string(),
                                    is_essential,
                                    is_initial_install,
                                    installed_time,
                                });
                            }
                        }
                    }
                }
            }
        }

        all_apps
            .retain(|a| a.name.to_lowercase() != "stasis" && a.name.to_lowercase() != "sysmon-tui");
        self.items = all_apps;
        self.apply_sorting();
        self.is_loading = false;
    }

    fn parse_size_string(sz_str: &str) -> u64 {
        let clean = sz_str.trim();
        if clean.is_empty() {
            return 0;
        }
        let parts: Vec<&str> = clean.split_whitespace().collect();
        if parts.is_empty() {
            return 0;
        }
        let num: f64 = parts[0].parse().unwrap_or(0.0);
        let unit = if parts.len() > 1 {
            parts[1].to_uppercase()
        } else {
            "B".to_string()
        };

        if unit.starts_with("K") || unit.starts_with("KIB") {
            (num * 1024.0) as u64
        } else if unit.starts_with("M") || unit.starts_with("MIB") {
            (num * 1024.0 * 1024.0) as u64
        } else if unit.starts_with("G") || unit.starts_with("GIB") {
            (num * 1024.0 * 1024.0 * 1024.0) as u64
        } else {
            num as u64
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            AppSortBy::Size => AppSortBy::Age,
            AppSortBy::Age => AppSortBy::Name,
            AppSortBy::Name => AppSortBy::Source,
            AppSortBy::Source => AppSortBy::Size,
        };
        self.apply_sorting();
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_descending = !self.sort_descending;
        self.apply_sorting();
    }

    pub fn apply_sorting(&mut self) {
        let desc = self.sort_descending;
        match self.sort_by {
            AppSortBy::Size => {
                self.items.sort_by(|a, b| {
                    if desc {
                        b.size_bytes.cmp(&a.size_bytes)
                    } else {
                        a.size_bytes.cmp(&b.size_bytes)
                    }
                });
            }
            AppSortBy::Age => {
                self.items.sort_by(|a, b| {
                    let ts_a = a.installed_time.unwrap_or(0);
                    let ts_b = b.installed_time.unwrap_or(0);
                    if desc {
                        ts_b.cmp(&ts_a)
                    } else {
                        ts_a.cmp(&ts_b)
                    }
                });
            }
            AppSortBy::Name => {
                self.items.sort_by(|a, b| {
                    if desc {
                        b.name.to_lowercase().cmp(&a.name.to_lowercase())
                    } else {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                });
            }
            AppSortBy::Source => {
                self.items.sort_by(|a, b| {
                    if desc {
                        b.source.cmp(&a.source)
                    } else {
                        a.source.cmp(&b.source)
                    }
                });
            }
        }
    }

    pub fn filtered_items(&self) -> Vec<&ApplicationItem> {
        let q = self.search_query.to_lowercase();
        self.items
            .iter()
            .filter(|app| match self.source_filter {
                AppSourceFilter::All => true,
                AppSourceFilter::UserInstalled => !app.is_initial_install && !app.is_essential,
                AppSourceFilter::InitialOS => app.is_initial_install,
                AppSourceFilter::DesktopOnly => app.source == "Desktop",
                AppSourceFilter::Flatpak => app.source == "Flatpak",
                AppSourceFilter::Snap => app.source == "Snap",
            })
            .filter(|app| {
                if !q.is_empty() {
                    app.name.to_lowercase().contains(&q)
                        || app.description.to_lowercase().contains(&q)
                        || app.package_id.to_lowercase().contains(&q)
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn uninstall_app(
        &mut self,
        app: &ApplicationItem,
        sudo_pass: Option<&str>,
    ) -> Result<String, String> {
        if app.is_essential {
            return Err(format!(
                "Cannot uninstall '{}': Protected system package essential for Linux operation",
                app.name
            ));
        }

        let (cmd_name, args, needs_sudo): (&str, Vec<&str>, bool) = match app.source.as_str() {
            "APT" => ("apt-get", vec!["remove", "-y", "-q", &app.package_id], true),
            "Pacman" => ("pacman", vec!["-R", "--noconfirm", &app.package_id], true),
            "Flatpak" => ("flatpak", vec!["uninstall", "-y", &app.package_id], false),
            "Snap" => ("snap", vec!["remove", &app.package_id], true),
            "Desktop" => {
                let path = Path::new(&app.package_id);
                if path.exists() {
                    let is_sys = app.package_id.starts_with("/usr/share")
                        || app.package_id.starts_with("/usr/local");
                    if is_sys {
                        crate::system::sudo::run_elevated_command(
                            "rm",
                            &["-f", &app.package_id],
                            sudo_pass,
                        )?;
                    } else {
                        fs::remove_file(path)
                            .map_err(|e| format!("Failed to delete desktop file: {}", e))?;
                    }
                    self.refresh();
                    return Ok(format!("Removed desktop entry for '{}'", app.name));
                } else {
                    return Err("File not found".to_string());
                }
            }
            _ => return Err("Unsupported package manager for uninstallation".to_string()),
        };

        let res = if needs_sudo {
            crate::system::sudo::run_elevated_command(cmd_name, &args, sudo_pass)
        } else {
            let output = Command::new(cmd_name)
                .args(args)
                .env("DEBIAN_FRONTEND", "noninteractive")
                .env("SYSTEMD_PAGER", "")
                .output()
                .map_err(|e| format!("Execution failed: {}", e))?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let err = String::from_utf8_lossy(&output.stderr);
                Err(if err.trim().is_empty() {
                    format!(
                        "Uninstallation exited with code: {:?}",
                        output.status.code()
                    )
                } else {
                    err.trim().to_string()
                })
            }
        };

        match res {
            Ok(_) => {
                self.refresh();
                Ok(format!("Successfully uninstalled '{}'", app.name))
            }
            Err(e) => Err(e),
        }
    }
}

fn dirs_next_desktop() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| Path::new(&h).join(".local/share/applications"))
}
