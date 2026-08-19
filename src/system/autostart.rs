use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AutostartItem {
    pub name: String,
    pub exec: String,
    pub comment: String,
    pub enabled: bool,
    pub file_path: PathBuf,
    pub is_user: bool,
}

pub struct AutostartManager {
    pub items: Vec<AutostartItem>,
    pub search_query: String,
}

impl AutostartManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            items: Vec::new(),
            search_query: String::new(),
        };
        mgr.refresh();
        mgr
    }

    pub fn refresh(&mut self) {
        self.items.clear();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let user_autostart = PathBuf::from(home).join(".config/autostart");
        let system_autostart = PathBuf::from("/etc/xdg/autostart");

        // Load user autostart
        if user_autostart.exists() {
            if let Ok(entries) = fs::read_dir(&user_autostart) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(item) = Self::parse_desktop_file(&path, true) {
                            self.items.push(item);
                        }
                    }
                }
            }
        }

        // Load system autostart (avoiding duplicates if overridden in user config)
        if system_autostart.exists() {
            if let Ok(entries) = fs::read_dir(&system_autostart) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(file_name) = path.file_name() {
                            let user_override = user_autostart.join(file_name);
                            if !user_override.exists() {
                                if let Some(item) = Self::parse_desktop_file(&path, false) {
                                    self.items.push(item);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort items alphabetically by name
        self.items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    fn parse_desktop_file(path: &Path, is_user: bool) -> Option<AutostartItem> {
        let content = fs::read_to_string(path).ok()?;
        let mut name = String::new();
        let mut exec = String::new();
        let mut comment = String::new();
        let mut hidden = false;
        let mut autostart_enabled = true;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("Name=") && name.is_empty() {
                name = line.trim_start_matches("Name=").to_string();
            } else if line.starts_with("Exec=") && exec.is_empty() {
                exec = line.trim_start_matches("Exec=").to_string();
            } else if line.starts_with("Comment=") && comment.is_empty() {
                comment = line.trim_start_matches("Comment=").to_string();
            } else if line.starts_with("Hidden=") {
                hidden = line.trim_start_matches("Hidden=").to_lowercase() == "true";
            } else if line.starts_with("X-GNOME-Autostart-enabled=") {
                autostart_enabled =
                    line.trim_start_matches("X-GNOME-Autostart-enabled=").to_lowercase() != "false";
            }
        }

        if name.is_empty() {
            name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
        }

        let enabled = !hidden && autostart_enabled;

        Some(AutostartItem {
            name,
            exec,
            comment,
            enabled,
            file_path: path.to_path_buf(),
            is_user,
        })
    }

    pub fn toggle_item(&mut self, index: usize) -> Result<String, String> {
        if index >= self.items.len() {
            return Err("Invalid index".to_string());
        }

        let item = &mut self.items[index];
        let new_state = !item.enabled;

        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let user_autostart_dir = PathBuf::from(&home).join(".config/autostart");
        if !user_autostart_dir.exists() {
            fs::create_dir_all(&user_autostart_dir)
                .map_err(|e| format!("Failed to create autostart dir: {}", e))?;
        }

        let target_file = if item.is_user {
            item.file_path.clone()
        } else {
            let file_name = item
                .file_path
                .file_name()
                .ok_or_else(|| "Invalid file name".to_string())?;
            user_autostart_dir.join(file_name)
        };

        let content = if target_file.exists() {
            fs::read_to_string(&target_file).unwrap_or_default()
        } else if item.file_path.exists() {
            fs::read_to_string(&item.file_path).unwrap_or_default()
        } else {
            format!(
                "[Desktop Entry]\nType=Application\nName={}\nExec={}\nComment={}\n",
                item.name, item.exec, item.comment
            )
        };

        let mut lines: Vec<String> = Vec::new();
        let mut has_hidden = false;
        let mut has_gnome_enabled = false;

        for line in content.lines() {
            if line.starts_with("Hidden=") {
                lines.push(format!("Hidden={}", !new_state));
                has_hidden = true;
            } else if line.starts_with("X-GNOME-Autostart-enabled=") {
                lines.push(format!("X-GNOME-Autostart-enabled={}", new_state));
                has_gnome_enabled = true;
            } else {
                lines.push(line.to_string());
            }
        }

        if !has_hidden {
            lines.push(format!("Hidden={}", !new_state));
        }
        if !has_gnome_enabled {
            lines.push(format!("X-GNOME-Autostart-enabled={}", new_state));
        }

        fs::write(&target_file, lines.join("\n"))
            .map_err(|e| format!("Failed to write desktop file: {}", e))?;

        item.enabled = new_state;
        item.file_path = target_file;
        item.is_user = true;

        Ok(format!(
            "{} autostart for '{}'",
            if new_state { "Enabled" } else { "Disabled" },
            item.name
        ))
    }

    pub fn add_entry(
        &mut self,
        name: &str,
        exec: &str,
        comment: &str,
    ) -> Result<String, String> {
        if name.trim().is_empty() || exec.trim().is_empty() {
            return Err("Name and Command cannot be empty".to_string());
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let user_autostart_dir = PathBuf::from(&home).join(".config/autostart");
        if !user_autostart_dir.exists() {
            fs::create_dir_all(&user_autostart_dir)
                .map_err(|e| format!("Failed to create autostart directory: {}", e))?;
        }

        let sanitized_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let file_path = user_autostart_dir.join(format!("{}.desktop", sanitized_name.to_lowercase()));

        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec={}\nComment={}\nHidden=false\nX-GNOME-Autostart-enabled=true\n",
            name.trim(),
            exec.trim(),
            comment.trim()
        );

        fs::write(&file_path, content)
            .map_err(|e| format!("Failed to write autostart file: {}", e))?;

        self.refresh();
        Ok(format!("Added autostart entry for '{}'", name))
    }

    pub fn remove_entry(&mut self, index: usize) -> Result<String, String> {
        if index >= self.items.len() {
            return Err("Invalid index".to_string());
        }

        let item = &self.items[index];
        if !item.is_user {
            return Err("Cannot remove system-wide autostart file. Toggle it to disable instead.".to_string());
        }

        if item.file_path.exists() {
            fs::remove_file(&item.file_path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }

        let name = item.name.clone();
        self.refresh();
        Ok(format!("Removed autostart entry '{}'", name))
    }

    pub fn filtered_items(&self) -> Vec<(usize, &AutostartItem)> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if self.search_query.is_empty() {
                    true
                } else {
                    let q = self.search_query.to_lowercase();
                    item.name.to_lowercase().contains(&q)
                        || item.exec.to_lowercase().contains(&q)
                        || item.comment.to_lowercase().contains(&q)
                }
            })
            .collect()
    }
}
