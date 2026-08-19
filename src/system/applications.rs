use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum AppSourceFilter {
    All,
    SystemPkg,
    DesktopOnly,
    Flatpak,
    Snap,
}

impl AppSourceFilter {
    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::SystemPkg,
            Self::SystemPkg => Self::DesktopOnly,
            Self::DesktopOnly => Self::Flatpak,
            Self::Flatpak => Self::Snap,
            Self::Snap => Self::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All Sources",
            Self::SystemPkg => "System Packages",
            Self::DesktopOnly => "Desktop Apps",
            Self::Flatpak => "Flatpak",
            Self::Snap => "Snap",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppSortBy {
    Size,
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
}

pub struct ApplicationManager {
    pub items: Vec<ApplicationItem>,
    pub search_query: String,
    pub source_filter: AppSourceFilter,
    pub sort_by: AppSortBy,
    pub sort_descending: bool,
    pub is_loading: bool,
}

pub fn is_system_essential_package(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "base-files"
        || lower.starts_with("systemd")
        || lower.starts_with("libc6")
        || lower.starts_with("coreutils")
        || lower == "dpkg"
        || lower == "apt"
        || lower.starts_with("linux-image")
        || lower.starts_with("linux-headers")
        || lower.starts_with("linux-modules")
        || lower.starts_with("grub")
        || lower.starts_with("udev")
        || lower == "bash"
        || lower == "dash"
        || lower == "login"
        || lower == "sudo"
        || lower == "dbus"
        || lower == "polkit"
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

        // 1. Try Debian/Ubuntu dpkg-query
        if let Ok(output) = Command::new("dpkg-query")
            .args(["-W", "-f=${binary:Package}\t${Version}\t${Installed-Size}\t${binary:Summary}\n"])
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

                        // Fix: Some 3rd-party .deb packages (e.g. Viber, Zoom) incorrectly record
                        // Installed-Size in raw bytes instead of KiB. If raw_size > 5,000,000 (>5GB),
                        // treat as raw bytes. Otherwise convert KiB to bytes (* 1024).
                        let size_bytes = if raw_size > 5_000_000 {
                            raw_size
                        } else {
                            raw_size * 1024
                        };

                        let is_essential = is_system_essential_package(&name);

                        all_apps.push(ApplicationItem {
                            name: name.clone(),
                            version,
                            size_bytes,
                            description: desc,
                            source: "APT".to_string(),
                            package_id: name,
                            is_essential,
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
                        });
                    }
                }
            }
        }

        // 3. Try Flatpak list
        if let Ok(output) = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,size,description"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 4 {
                        let app_id = parts[0].trim().to_string();
                        let version = parts[1].trim().to_string();
                        let size_str = parts[2].trim();
                        let desc = parts[3].trim().to_string();
                        let size_bytes = Self::parse_size_string(size_str);

                        all_apps.push(ApplicationItem {
                            name: app_id.split('.').last().unwrap_or(&app_id).to_string(),
                            version,
                            size_bytes,
                            description: desc,
                            source: "Flatpak".to_string(),
                            package_id: app_id,
                            is_essential: false,
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
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let name = parts[0].trim().to_string();
                        let version = parts[1].trim().to_string();
                        let summary = format!("Snap package: {}", name);

                        all_apps.push(ApplicationItem {
                            name: name.clone(),
                            version,
                            size_bytes: 0,
                            description: summary,
                            source: "Snap".to_string(),
                            package_id: name,
                            is_essential: false,
                        });
                    }
                }
            }
        }

        // 5. Scan standalone Desktop Applications (.desktop files)
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let desktop_dirs = vec![
            PathBuf::from(home).join(".local/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            PathBuf::from("/usr/share/applications"),
        ];

        for dir in desktop_dirs {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let mut name = String::new();
                            let mut comment = String::new();
                            let mut exec = String::new();
                            let mut version = "1.0".to_string();
                            let mut is_nodisplay = false;

                            for line in content.lines() {
                                if line.starts_with("Name=") && name.is_empty() {
                                    name = line["Name=".len()..].trim().to_string();
                                } else if line.starts_with("Comment=") && comment.is_empty() {
                                    comment = line["Comment=".len()..].trim().to_string();
                                } else if line.starts_with("Exec=") && exec.is_empty() {
                                    exec = line["Exec=".len()..].trim().to_string();
                                } else if line.starts_with("Version=") {
                                    version = line["Version=".len()..].trim().to_string();
                                } else if line.starts_with("NoDisplay=true") {
                                    is_nodisplay = true;
                                }
                            }

                            if is_nodisplay || name.is_empty() || name.to_lowercase() == "stasis" || name.to_lowercase() == "sysmon-tui" || name.to_lowercase() == "vim" {
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
                                if let Ok(which_out) = Command::new("which").arg(exec_cmd).output() {
                                    if which_out.status.success() {
                                        let bin_path_str = String::from_utf8_lossy(&which_out.stdout).trim().to_string();
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

                                all_apps.push(ApplicationItem {
                                    name: name.clone(),
                                    version,
                                    size_bytes,
                                    description: if comment.is_empty() { format!("Desktop application: {}", name) } else { comment },
                                    source: "Desktop".to_string(),
                                    package_id: path.display().to_string(),
                                    is_essential,
                                });
                            }
                        }
                    }
                }
            }
        }

        all_apps.retain(|a| a.name.to_lowercase() != "stasis" && a.name.to_lowercase() != "sysmon-tui");
        self.items = all_apps;
        self.apply_sorting();
        self.is_loading = false;
    }

    fn parse_size_string(s: &str) -> u64 {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return 0;
        }

        let num: f64 = parts[0].parse().unwrap_or(0.0);
        let unit = parts.get(1).map(|&u| u.to_lowercase()).unwrap_or_default();

        let multiplier: f64 = match unit.as_str() {
            "b" | "bytes" => 1.0,
            "kib" | "kb" | "k" => 1024.0,
            "mib" | "mb" | "m" => 1024.0 * 1024.0,
            "gib" | "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
            "tib" | "tb" | "t" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            _ => 1024.0 * 1024.0,
        };

        (num * multiplier) as u64
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            AppSortBy::Size => AppSortBy::Name,
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
            .filter(|app| {
                match self.source_filter {
                    AppSourceFilter::All => true,
                    AppSourceFilter::SystemPkg => app.source == "APT" || app.source == "Pacman" || app.source == "RPM",
                    AppSourceFilter::DesktopOnly => app.source == "Desktop",
                    AppSourceFilter::Flatpak => app.source == "Flatpak",
                    AppSourceFilter::Snap => app.source == "Snap",
                }
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

    pub fn uninstall_app(&mut self, app: &ApplicationItem, sudo_pass: Option<&str>) -> Result<String, String> {
        let (cmd_name, args, needs_sudo): (&str, Vec<&str>, bool) = match app.source.as_str() {
            "APT" => ("apt-get", vec!["remove", "-y", &app.package_id], true),
            "Pacman" => ("pacman", vec!["-R", "--noconfirm", &app.package_id], true),
            "Flatpak" => ("flatpak", vec!["uninstall", "-y", &app.package_id], false),
            "Snap" => ("snap", vec!["remove", &app.package_id], true),
            "Desktop" => {
                let path = Path::new(&app.package_id);
                if path.exists() {
                    let is_sys = app.package_id.starts_with("/usr/share") || app.package_id.starts_with("/usr/local");
                    if is_sys {
                        crate::system::sudo::run_elevated_command("rm", &["-f", &app.package_id], sudo_pass)?;
                    } else {
                        fs::remove_file(path).map_err(|e| format!("Failed to delete desktop file: {}", e))?;
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
                .output()
                .map_err(|e| format!("Execution failed: {}", e))?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let err = String::from_utf8_lossy(&output.stderr);
                Err(if err.trim().is_empty() {
                    format!("Uninstallation exited with code: {:?}", output.status.code())
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
