use crate::system::sudo::run_elevated_command;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceFilterState {
    All,
    Active,
    Inactive,
    Failed,
}

impl ServiceFilterState {
    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Inactive,
            Self::Inactive => Self::Failed,
            Self::Failed => Self::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All States",
            Self::Active => "Active Only",
            Self::Inactive => "Inactive Only",
            Self::Failed => "Failed Units",
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ServiceItem {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
    pub is_user_unit: bool,
}

pub struct ServiceManager {
    pub services: Vec<ServiceItem>,
    pub user_mode: bool,
    pub filter_state: ServiceFilterState,
    pub search_query: String,
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            services: Vec::new(),
            user_mode: false,
            filter_state: ServiceFilterState::All,
            search_query: String::new(),
        };
        mgr.refresh();
        mgr
    }

    pub fn refresh(&mut self) {
        self.services = self.fetch_services();
    }

    fn fetch_services(&self) -> Vec<ServiceItem> {
        let mut cmd = Command::new("systemctl");
        if self.user_mode {
            cmd.arg("--user");
        }
        cmd.args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
            "--plain",
        ]);

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        if !output.status.success() {
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut items = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                if !name.ends_with(".service") {
                    continue;
                }
                let load_state = parts[1].to_string();
                let active_state = parts[2].to_string();
                let sub_state = parts[3].to_string();
                let description = if parts.len() > 4 {
                    parts[4..].join(" ")
                } else {
                    name.clone()
                };

                items.push(ServiceItem {
                    name,
                    load_state,
                    active_state,
                    sub_state,
                    description,
                    is_user_unit: self.user_mode,
                });
            }
        }

        items
    }

    pub fn filtered_services(&self) -> Vec<&ServiceItem> {
        self.services
            .iter()
            .filter(|s| {
                // State filter
                let matches_state = match self.filter_state {
                    ServiceFilterState::All => true,
                    ServiceFilterState::Active => s.active_state == "active",
                    ServiceFilterState::Inactive => s.active_state == "inactive",
                    ServiceFilterState::Failed => {
                        s.active_state == "failed" || s.sub_state == "failed"
                    }
                };

                if !matches_state {
                    return false;
                }

                // Text filter
                if !self.search_query.is_empty() {
                    let q = self.search_query.to_lowercase();
                    s.name.to_lowercase().contains(&q) || s.description.to_lowercase().contains(&q)
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn start_service(&mut self, unit: &str, sudo_pass: Option<&str>) -> Result<String, String> {
        self.run_systemctl_action("start", unit, sudo_pass)
    }

    pub fn stop_service(&mut self, unit: &str, sudo_pass: Option<&str>) -> Result<String, String> {
        self.run_systemctl_action("stop", unit, sudo_pass)
    }

    pub fn restart_service(
        &mut self,
        unit: &str,
        sudo_pass: Option<&str>,
    ) -> Result<String, String> {
        self.run_systemctl_action("restart", unit, sudo_pass)
    }

    pub fn enable_service(
        &mut self,
        unit: &str,
        sudo_pass: Option<&str>,
    ) -> Result<String, String> {
        self.run_systemctl_action("enable", unit, sudo_pass)
    }

    pub fn disable_service(
        &mut self,
        unit: &str,
        sudo_pass: Option<&str>,
    ) -> Result<String, String> {
        self.run_systemctl_action("disable", unit, sudo_pass)
    }

    fn run_systemctl_action(
        &mut self,
        action: &str,
        unit: &str,
        sudo_pass: Option<&str>,
    ) -> Result<String, String> {
        if self.user_mode {
            let mut cmd = Command::new("systemctl");
            cmd.arg("--user").arg(action).arg(unit);
            match cmd.output() {
                Ok(out) => {
                    if out.status.success() {
                        self.refresh();
                        Ok(format!(
                            "Successfully performed '{}' on user service {}",
                            action, unit
                        ))
                    } else {
                        let err = String::from_utf8_lossy(&out.stderr);
                        Err(if err.trim().is_empty() {
                            format!(
                                "Failed to {} {} (exit code: {:?})",
                                action,
                                unit,
                                out.status.code()
                            )
                        } else {
                            err.trim().to_string()
                        })
                    }
                }
                Err(e) => Err(format!("Command execution failed: {}", e)),
            }
        } else {
            // System unit requires elevated privileges
            let res = run_elevated_command("systemctl", &[action, unit], sudo_pass);
            match res {
                Ok(_) => {
                    self.refresh();
                    Ok(format!(
                        "Successfully performed '{}' on system service {}",
                        action, unit
                    ))
                }
                Err(e) => Err(e),
            }
        }
    }
}
