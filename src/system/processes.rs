use std::cmp::Ordering;
use sysinfo::{ProcessStatus, System, Users};

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessSortBy {
    Pid,
    Name,
    Cpu,
    Memory,
    DiskRead,
    DiskWrite,
}

#[derive(Clone, Debug)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cmd: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_pct: f32,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub status: String,
    pub user: String,
    pub is_critical: bool,
}

pub fn is_system_critical_process(pid: u32, name: &str, cmd: &str, user: &str) -> bool {
    if pid <= 2 {
        return true;
    }
    if (cmd.starts_with('[') && cmd.ends_with(']')) || cmd.starts_with("[kworker") {
        return true;
    }
    let lower_name = name.to_lowercase();
    if lower_name == "systemd"
        || lower_name == "systemd-journald"
        || lower_name == "systemd-udevd"
        || lower_name == "systemd-logind"
        || lower_name == "systemd-resolved"
        || lower_name == "systemd-timesyncd"
        || lower_name == "dbus-daemon"
        || lower_name == "polkitd"
        || lower_name == "kcompactd0"
        || lower_name == "accounts-daemon"
        || lower_name == "networkmanager"
        || lower_name == "wpa_supplicant"
        || lower_name == "cron"
        || lower_name == "rsyslogd"
        || lower_name == "avahi-daemon"
        || lower_name == "pipewire"
        || lower_name == "pipewire-pulse"
        || lower_name == "wireplumber"
        || lower_name == "rtkit-daemon"
        || lower_name == "udisksd"
        || lower_name == "upowerd"
        || (user == "root" && pid < 1000)
    {
        return true;
    }
    false
}

pub struct ProcessManager {
    pub users: Users,
    pub sort_by: ProcessSortBy,
    pub sort_descending: bool,
    pub filter: String,
}

impl ProcessManager {
    pub fn new() -> Self {
        let users = Users::new_with_refreshed_list();
        Self {
            users,
            sort_by: ProcessSortBy::Cpu,
            sort_descending: true,
            filter: String::new(),
        }
    }

    pub fn refresh_users(&mut self) {
        self.users.refresh();
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            ProcessSortBy::Cpu => ProcessSortBy::Memory,
            ProcessSortBy::Memory => ProcessSortBy::Pid,
            ProcessSortBy::Pid => ProcessSortBy::Name,
            ProcessSortBy::Name => ProcessSortBy::DiskRead,
            ProcessSortBy::DiskRead => ProcessSortBy::DiskWrite,
            ProcessSortBy::DiskWrite => ProcessSortBy::Cpu,
        };
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_descending = !self.sort_descending;
    }

    pub fn get_processes(&self, sys: &System) -> Vec<ProcessItem> {
        let total_mem = sys.total_memory() as f32;
        let mut items = Vec::with_capacity(sys.processes().len());

        for (&pid, proc) in sys.processes() {
            let proc_name = proc.name().to_string_lossy().to_string();
            let cmd = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" ");

            let mem_bytes = proc.memory();
            let mem_pct = if total_mem > 0.0 {
                (mem_bytes as f32 / total_mem) * 100.0
            } else {
                0.0
            };

            let disk_usage = proc.disk_usage();

            let status_str = match proc.status() {
                ProcessStatus::Run => "Running",
                ProcessStatus::Sleep => "Sleeping",
                ProcessStatus::Idle => "Idle",
                ProcessStatus::Zombie => "Zombie",
                ProcessStatus::Stop => "Stopped",
                ProcessStatus::Dead => "Dead",
                ProcessStatus::LockBlocked => "Blocked",
                ProcessStatus::UninterruptibleDiskSleep => "DiskSleep",
                _ => "Other",
            }
            .to_string();

            let user_str = proc
                .user_id()
                .and_then(|uid| self.users.get_user_by_id(uid))
                .map(|u| u.name().to_string())
                .unwrap_or_else(|| {
                    proc.user_id()
                        .map(|uid| uid.to_string())
                        .unwrap_or_else(|| "-".to_string())
                });

            // Filter
            if !self.filter.is_empty() {
                let q = self.filter.to_lowercase();
                let matches_name = proc_name.to_lowercase().contains(&q);
                let matches_cmd = cmd.to_lowercase().contains(&q);
                let matches_pid = pid.as_u32().to_string().contains(&q);
                let matches_user = user_str.to_lowercase().contains(&q);

                if !matches_name && !matches_cmd && !matches_pid && !matches_user {
                    continue;
                }
            }

            let effective_name = if proc_name.is_empty() {
                cmd.split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                proc_name
            };

            let is_critical = is_system_critical_process(pid.as_u32(), &effective_name, &cmd, &user_str);

            items.push(ProcessItem {
                pid: pid.as_u32(),
                name: effective_name,
                cmd: if cmd.is_empty() {
                    proc.name().to_string_lossy().to_string()
                } else {
                    cmd
                },
                cpu_usage: proc.cpu_usage(),
                memory_bytes: mem_bytes,
                memory_pct: mem_pct,
                disk_read_bytes: disk_usage.read_bytes,
                disk_write_bytes: disk_usage.written_bytes,
                status: status_str,
                user: user_str,
                is_critical,
            });
        }

        // Sort
        items.sort_by(|a, b| {
            let ordering = match self.sort_by {
                ProcessSortBy::Pid => a.pid.cmp(&b.pid),
                ProcessSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                ProcessSortBy::Cpu => a
                    .cpu_usage
                    .partial_cmp(&b.cpu_usage)
                    .unwrap_or(Ordering::Equal),
                ProcessSortBy::Memory => a.memory_bytes.cmp(&b.memory_bytes),
                ProcessSortBy::DiskRead => a.disk_read_bytes.cmp(&b.disk_read_bytes),
                ProcessSortBy::DiskWrite => a.disk_write_bytes.cmp(&b.disk_write_bytes),
            };

            if self.sort_descending {
                ordering.reverse()
            } else {
                ordering
            }
        });

        items
    }

    pub fn kill_process(pid: u32) -> Result<(), String> {
        let res = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        if res == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        }
    }

    pub fn terminate_process(pid: u32) -> Result<(), String> {
        let res = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
        if res == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        }
    }

    pub fn stop_process(pid: u32) -> Result<(), String> {
        let res = unsafe { libc::kill(pid as libc::pid_t, libc::SIGSTOP) };
        if res == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        }
    }

    pub fn resume_process(pid: u32) -> Result<(), String> {
        let res = unsafe { libc::kill(pid as libc::pid_t, libc::SIGCONT) };
        if res == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        }
    }
}
