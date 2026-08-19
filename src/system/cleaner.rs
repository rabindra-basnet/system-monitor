use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CleanCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub paths: Vec<PathBuf>,
    pub selected: bool,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub requires_root: bool,
    pub is_scanning: bool,
}

pub struct SystemCleaner {
    pub categories: Vec<CleanCategory>,
    pub total_scanned_bytes: u64,
    pub total_scanned_files: usize,
    pub is_busy: bool,
}

impl Default for SystemCleaner {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCleaner {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let home_path = Path::new(&home);

        let categories = vec![
            CleanCategory {
                id: "app_cache".to_string(),
                name: "Application Caches".to_string(),
                description: "User application temporary cache files (~/.cache)".to_string(),
                paths: vec![home_path.join(".cache")],
                selected: true,
                file_count: 0,
                total_size_bytes: 0,
                requires_root: false,
                is_scanning: false,
            },
            CleanCategory {
                id: "thumbnails".to_string(),
                name: "Thumbnail Cache".to_string(),
                description: "Cached image, video and file preview thumbnails".to_string(),
                paths: vec![home_path.join(".cache/thumbnails")],
                selected: true,
                file_count: 0,
                total_size_bytes: 0,
                requires_root: false,
                is_scanning: false,
            },
            CleanCategory {
                id: "trash".to_string(),
                name: "User Trash Bin".to_string(),
                description: "Deleted files inside ~/.local/share/Trash".to_string(),
                paths: vec![home_path.join(".local/share/Trash")],
                selected: true,
                file_count: 0,
                total_size_bytes: 0,
                requires_root: false,
                is_scanning: false,
            },
            CleanCategory {
                id: "pkg_cache".to_string(),
                name: "Package Caches".to_string(),
                description: "APT, Pacman, DNF, Flatpak & Snap downloaded package archives"
                    .to_string(),
                paths: vec![
                    PathBuf::from("/var/cache/apt/archives"),
                    PathBuf::from("/var/cache/pacman/pkg"),
                    PathBuf::from("/var/cache/dnf"),
                    home_path.join(".var/app"),
                    PathBuf::from("/var/lib/snapd/cache"),
                ],
                selected: false,
                file_count: 0,
                total_size_bytes: 0,
                requires_root: true,
                is_scanning: false,
            },
            CleanCategory {
                id: "crash_reports".to_string(),
                name: "Crash Reports & Dumps".to_string(),
                description: "Application crash dumps, coredumps and diagnostic logs".to_string(),
                paths: vec![
                    PathBuf::from("/var/crash"),
                    PathBuf::from("/var/lib/systemd/coredump"),
                ],
                selected: false,
                file_count: 0,
                total_size_bytes: 0,
                requires_root: true,
                is_scanning: false,
            },
            CleanCategory {
                id: "system_logs".to_string(),
                name: "System & App Logs".to_string(),
                description: "System logs in /var/log and old user application session logs"
                    .to_string(),
                paths: vec![
                    PathBuf::from("/var/log"),
                    home_path.join(".local/share/xorg"),
                ],
                selected: false,
                file_count: 0,
                total_size_bytes: 0,
                requires_root: true,
                is_scanning: false,
            },
        ];

        Self {
            categories,
            total_scanned_bytes: 0,
            total_scanned_files: 0,
            is_busy: false,
        }
    }

    pub fn scan(&mut self) {
        self.is_busy = true;
        let mut total_bytes = 0;
        let mut total_files = 0;

        for cat in &mut self.categories {
            let mut cat_bytes = 0;
            let mut cat_files = 0;

            for path in &cat.paths {
                if !path.exists() {
                    continue;
                }

                for entry in WalkDir::new(path)
                    .min_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Ok(meta) = entry.metadata() {
                            cat_bytes += meta.len();
                            cat_files += 1;
                        }
                    }
                }
            }

            cat.file_count = cat_files;
            cat.total_size_bytes = cat_bytes;

            if cat.selected {
                total_bytes += cat_bytes;
                total_files += cat_files;
            }
        }

        self.total_scanned_bytes = total_bytes;
        self.total_scanned_files = total_files;
        self.is_busy = false;
    }

    pub fn clean_selected(
        &mut self,
        sudo_pass: Option<&str>,
    ) -> Result<(usize, u64, Vec<String>), String> {
        let mut cleaned_files = 0;
        let mut cleaned_bytes = 0;
        let mut errors = Vec::new();
        let mut elevated_paths = Vec::new();

        for cat in &mut self.categories {
            if !cat.selected {
                continue;
            }

            for path in &cat.paths {
                if !path.exists() {
                    continue;
                }

                if path.is_file() {
                    if let Ok(meta) = fs::metadata(path) {
                        let sz = meta.len();
                        if fs::remove_file(path).is_ok() {
                            cleaned_files += 1;
                            cleaned_bytes += sz;
                        } else if sudo_pass.is_some() {
                            elevated_paths.push((path.clone(), sz));
                        } else {
                            errors.push(format!("Permission denied: {}", path.display()));
                        }
                    }
                } else if path.is_dir() {
                    for entry in WalkDir::new(path)
                        .min_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            let p = entry.path();
                            if let Ok(meta) = entry.metadata() {
                                let sz = meta.len();
                                if fs::remove_file(p).is_ok() {
                                    cleaned_files += 1;
                                    cleaned_bytes += sz;
                                } else if sudo_pass.is_some() {
                                    elevated_paths.push((p.to_path_buf(), sz));
                                } else {
                                    errors.push(format!("Permission denied: {}", p.display()));
                                }
                            }
                        }
                    }
                }
            }

            cat.file_count = 0;
            cat.total_size_bytes = 0;
        }

        // Process elevated files in batches of 50
        if let Some(pass) = sudo_pass {
            for chunk in elevated_paths.chunks(50) {
                let path_strs: Vec<String> =
                    chunk.iter().map(|(p, _)| p.display().to_string()).collect();
                let args_refs: Vec<&str> = std::iter::once("-f")
                    .chain(path_strs.iter().map(|s| s.as_str()))
                    .collect();

                if crate::system::sudo::run_elevated_command("rm", &args_refs, Some(pass)).is_ok() {
                    for (_, sz) in chunk {
                        cleaned_files += 1;
                        cleaned_bytes += sz;
                    }
                } else {
                    for (p, _) in chunk {
                        errors.push(format!("Permission denied: {}", p.display()));
                    }
                }
            }
        }

        // Re-scan to update remaining sizes
        self.scan();

        Ok((cleaned_files, cleaned_bytes, errors))
    }

    pub fn toggle_category(&mut self, index: usize) {
        if let Some(cat) = self.categories.get_mut(index) {
            cat.selected = !cat.selected;
            self.recalc_totals();
        }
    }

    pub fn select_all(&mut self, select: bool) {
        for cat in &mut self.categories {
            cat.selected = select;
        }
        self.recalc_totals();
    }

    fn recalc_totals(&mut self) {
        let mut total_bytes = 0;
        let mut total_files = 0;
        for cat in &self.categories {
            if cat.selected {
                total_bytes += cat.total_size_bytes;
                total_files += cat.file_count;
            }
        }
        self.total_scanned_bytes = total_bytes;
        self.total_scanned_files = total_files;
    }
}
