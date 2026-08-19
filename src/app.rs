use std::time::{Duration, Instant};
use ratatui::widgets::TableState;

use crate::system::applications::{ApplicationItem, ApplicationManager};
use crate::system::autostart::AutostartManager;
use crate::system::cleaner::SystemCleaner;
use crate::system::collector::SystemCollector;
use crate::system::gpu::GpuCollector;
use crate::system::network::NetworkManager;
use crate::system::processes::{ProcessItem, ProcessManager, ProcessSortBy};
use crate::system::sensors::SensorCollector;
use crate::system::services::ServiceManager;
use crate::system::sudo::is_root;
use crate::theme::{Theme, ThemeMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppTab {
    Dashboard = 0,
    Processes = 1,
    Cleaner = 2,
    Services = 3,
    Autostart = 4,
    Applications = 5,
}

impl AppTab {
    pub fn all() -> &'static [AppTab] {
        &[
            AppTab::Dashboard,
            AppTab::Processes,
            AppTab::Cleaner,
            AppTab::Services,
            AppTab::Autostart,
            AppTab::Applications,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            AppTab::Dashboard => "󰍹 Dashboard & Resources",
            AppTab::Processes => " Processes",
            AppTab::Cleaner => "󰃢 Cleaner",
            AppTab::Services => " Services",
            AppTab::Autostart => "󱑞 Startup Apps",
            AppTab::Applications => "󰏖 Uninstaller",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            AppTab::Dashboard => AppTab::Processes,
            AppTab::Processes => AppTab::Cleaner,
            AppTab::Cleaner => AppTab::Services,
            AppTab::Services => AppTab::Autostart,
            AppTab::Autostart => AppTab::Applications,
            AppTab::Applications => AppTab::Dashboard,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            AppTab::Dashboard => AppTab::Applications,
            AppTab::Processes => AppTab::Dashboard,
            AppTab::Cleaner => AppTab::Processes,
            AppTab::Services => AppTab::Cleaner,
            AppTab::Autostart => AppTab::Services,
            AppTab::Applications => AppTab::Autostart,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    KillProcess(u32, String),
    TerminateProcess(u32, String),
    StopProcess(u32, String),
    ResumeProcess(u32, String),
    CleanCategories(Vec<String>, u64),
    ServiceAction(String, String),
    RemoveAutostart(usize, String),
    UninstallApp(ApplicationItem),
    KillPort {
        port: u16,
        proto: String,
        proc_name: String,
        pid: Option<u32>,
    },
}

impl ConfirmAction {
    pub fn requires_elevation(&self, user_mode_services: bool) -> bool {
        match self {
            ConfirmAction::CleanCategories(cats, _) => {
                cats.iter().any(|c| c.contains("Package") || c.contains("Crash") || c.contains("Log"))
            }
            ConfirmAction::ServiceAction(_, _) => !user_mode_services,
            ConfirmAction::UninstallApp(app) => {
                app.source == "APT" || app.source == "Pacman" || app.source == "RPM" || app.source == "Snap" || app.package_id.starts_with("/usr/share")
            }
            ConfirmAction::KillPort { pid, .. } => pid.map_or(true, |p| p <= 1000),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputMode {
    Normal,
    Search,
    ConfirmModal(ConfirmAction),
    SudoPasswordModal {
        pending_action: Box<ConfirmAction>,
        password: String,
        error_msg: Option<String>,
    },
    HelpModal,
    NewAutostartModal {
        name: String,
        exec: String,
        comment: String,
        active_field: usize, // 0: Name, 1: Exec, 2: Comment
    },
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub created_at: Instant,
}

pub struct App {
    pub active_tab: AppTab,
    pub input_mode: InputMode,
    pub theme: Theme,
    pub should_quit: bool,
    pub toast: Option<Toast>,
    pub sudo_password: Option<String>,

    // Managers & Collectors
    pub collector: SystemCollector,
    pub process_mgr: ProcessManager,
    pub cleaner: SystemCleaner,
    pub service_mgr: ServiceManager,
    pub autostart_mgr: AutostartManager,
    pub app_mgr: ApplicationManager,
    pub sensor_collector: SensorCollector,
    pub network_mgr: NetworkManager,
    pub gpu_collector: GpuCollector,

    // Selections
    pub selected_port_index: usize,
    pub process_list: Vec<ProcessItem>,
    pub process_table_state: TableState,
    pub cleaner_selected_index: usize,
    pub service_table_state: TableState,
    pub autostart_table_state: TableState,
    pub app_table_state: TableState,

    // Search query buffer
    pub search_input: String,
}

impl App {
    pub fn new() -> Self {
        let mut collector = SystemCollector::new();
        collector.refresh();

        let process_mgr = ProcessManager::new();
        let process_list = process_mgr.get_processes(&collector.sys);

        let mut process_table_state = TableState::default();
        if !process_list.is_empty() {
            process_table_state.select(Some(0));
        }

        let cleaner = SystemCleaner::new();

        let service_mgr = ServiceManager::new();
        let mut service_table_state = TableState::default();
        if !service_mgr.services.is_empty() {
            service_table_state.select(Some(0));
        }

        let autostart_mgr = AutostartManager::new();
        let mut autostart_table_state = TableState::default();
        if !autostart_mgr.items.is_empty() {
            autostart_table_state.select(Some(0));
        }

        let app_mgr = ApplicationManager::new();
        let mut app_table_state = TableState::default();
        if !app_mgr.items.is_empty() {
            app_table_state.select(Some(0));
        }

        let sensor_collector = SensorCollector::new();
        let network_mgr = NetworkManager::new();
        let gpu_collector = GpuCollector::new();

        Self {
            active_tab: AppTab::Dashboard,
            input_mode: InputMode::Normal,
            theme: Theme::new(ThemeMode::Cyberpunk),
            should_quit: false,
            toast: None,
            sudo_password: None,

            collector,
            process_mgr,
            cleaner,
            service_mgr,
            autostart_mgr,
            app_mgr,
            sensor_collector,
            network_mgr,
            gpu_collector,

            selected_port_index: 0,
            process_list,
            process_table_state,
            cleaner_selected_index: 0,
            service_table_state,
            autostart_table_state,
            app_table_state,

            search_input: String::new(),
        }
    }

    pub fn is_root(&self) -> bool {
        is_root()
    }

    pub fn show_toast(&mut self, message: &str, is_error: bool) {
        self.toast = Some(Toast {
            message: message.to_string(),
            is_error,
            created_at: Instant::now(),
        });
    }

    pub fn tick(&mut self) {
        // Clear expired toast (after 4 seconds)
        if let Some(t) = &self.toast {
            if t.created_at.elapsed() > Duration::from_secs(4) {
                self.toast = None;
            }
        }

        // Refresh system metrics
        self.collector.refresh();
        self.sensor_collector.refresh();
        self.network_mgr.refresh();
        self.gpu_collector.refresh();

        // Refresh process list if active
        if self.active_tab == AppTab::Processes {
            self.refresh_processes();
        }
    }

    pub fn cycle_theme(&mut self) {
        let next_mode = self.theme.mode.next();
        self.theme = Theme::new(next_mode);
        self.show_toast(&format!("Theme: {}", next_mode.name()), false);
    }

    pub fn refresh_processes(&mut self) {
        self.process_mgr.refresh_users();
        self.process_list = self.process_mgr.get_processes(&self.collector.sys);

        // Adjust selection if out of bounds
        if let Some(selected) = self.process_table_state.selected() {
            if selected >= self.process_list.len() && !self.process_list.is_empty() {
                self.process_table_state.select(Some(self.process_list.len() - 1));
            }
        } else if !self.process_list.is_empty() {
            self.process_table_state.select(Some(0));
        }
    }

    pub fn next_process(&mut self) {
        if self.process_list.is_empty() {
            return;
        }
        let i = match self.process_table_state.selected() {
            Some(i) => {
                if i + 1 < self.process_list.len() {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.process_table_state.select(Some(i));
    }

    pub fn prev_process(&mut self) {
        if self.process_list.is_empty() {
            return;
        }
        let i = match self.process_table_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.process_table_state.select(Some(i));
    }

    pub fn page_down_processes(&mut self) {
        if self.process_list.is_empty() {
            return;
        }
        let current = self.process_table_state.selected().unwrap_or(0);
        let next = (current + 10).min(self.process_list.len() - 1);
        self.process_table_state.select(Some(next));
    }

    pub fn page_up_processes(&mut self) {
        if self.process_list.is_empty() {
            return;
        }
        let current = self.process_table_state.selected().unwrap_or(0);
        let next = current.saturating_sub(10);
        self.process_table_state.select(Some(next));
    }

    pub fn cycle_process_sort(&mut self) {
        self.process_mgr.sort_by = match self.process_mgr.sort_by {
            ProcessSortBy::Cpu => ProcessSortBy::Memory,
            ProcessSortBy::Memory => ProcessSortBy::Pid,
            ProcessSortBy::Pid => ProcessSortBy::DiskRead,
            ProcessSortBy::DiskRead => ProcessSortBy::DiskWrite,
            ProcessSortBy::DiskWrite => ProcessSortBy::Name,
            ProcessSortBy::Name => ProcessSortBy::Cpu,
        };
        self.refresh_processes();
        self.show_toast(
            &format!("Sorted by: {:?}", self.process_mgr.sort_by),
            false,
        );
    }

    pub fn toggle_process_sort_direction(&mut self) {
        self.process_mgr.sort_descending = !self.process_mgr.sort_descending;
        self.refresh_processes();
        self.show_toast(
            &format!(
                "Sort direction: {}",
                if self.process_mgr.sort_descending { "Descending" } else { "Ascending" }
            ),
            false,
        );
    }

    pub fn selected_process(&self) -> Option<&ProcessItem> {
        let i = self.process_table_state.selected()?;
        self.process_list.get(i)
    }

    // Cleaner navigation
    pub fn next_cleaner_category(&mut self) {
        if !self.cleaner.categories.is_empty() {
            if self.cleaner_selected_index + 1 < self.cleaner.categories.len() {
                self.cleaner_selected_index += 1;
            }
        }
    }

    pub fn prev_cleaner_category(&mut self) {
        if self.cleaner_selected_index > 0 {
            self.cleaner_selected_index -= 1;
        }
    }

    pub fn toggle_cleaner_category(&mut self) {
        self.cleaner.toggle_category(self.cleaner_selected_index);
    }

    pub fn scan_cleaner(&mut self) {
        self.cleaner.scan();
        self.show_toast(
            &format!(
                "Scan complete: {} files ({})",
                self.cleaner.total_scanned_files,
                crate::system::collector::format_bytes(self.cleaner.total_scanned_bytes)
            ),
            false,
        );
    }

    pub fn execute_clean(&mut self) {
        match self.cleaner.clean_selected(self.sudo_password.as_deref()) {
            Ok((files, bytes, errors)) => {
                if errors.is_empty() {
                    self.show_toast(
                        &format!(
                            "Cleaned {} files ({}) freed!",
                            files,
                            crate::system::collector::format_bytes(bytes)
                        ),
                        false,
                    );
                } else {
                    self.show_toast(
                        &format!(
                            "Cleaned {} files ({}), {} skipped (requires root/sudo)",
                            files,
                            crate::system::collector::format_bytes(bytes),
                            errors.len()
                        ),
                        true,
                    );
                }
            }
            Err(e) => {
                self.show_toast(&format!("Clean error: {}", e), true);
            }
        }
    }

    // Services navigation
    pub fn next_service(&mut self) {
        let filtered = self.service_mgr.filtered_services();
        if filtered.is_empty() {
            return;
        }
        let i = match self.service_table_state.selected() {
            Some(i) => {
                if i + 1 < filtered.len() {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.service_table_state.select(Some(i));
    }

    pub fn prev_service(&mut self) {
        let filtered = self.service_mgr.filtered_services();
        if filtered.is_empty() {
            return;
        }
        let i = match self.service_table_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.service_table_state.select(Some(i));
    }

    pub fn selected_service_unit(&self) -> Option<String> {
        let filtered = self.service_mgr.filtered_services();
        let i = self.service_table_state.selected()?;
        filtered.get(i).map(|s| s.name.clone())
    }

    pub fn toggle_service_mode(&mut self) {
        self.service_mgr.user_mode = !self.service_mgr.user_mode;
        self.service_mgr.refresh();
        self.service_table_state.select(Some(0));
        self.show_toast(
            &format!(
                "Service Mode: {}",
                if self.service_mgr.user_mode { "User (--user)" } else { "System (root)" }
            ),
            false,
        );
    }

    pub fn cycle_service_filter(&mut self) {
        self.service_mgr.filter_state = self.service_mgr.filter_state.next();
        self.service_table_state.select(Some(0));
        self.show_toast(
            &format!("Services Filter: {}", self.service_mgr.filter_state.label()),
            false,
        );
    }

    // Autostart navigation
    pub fn next_autostart(&mut self) {
        let filtered = self.autostart_mgr.filtered_items();
        if filtered.is_empty() {
            return;
        }
        let i = match self.autostart_table_state.selected() {
            Some(i) => {
                if i + 1 < filtered.len() {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.autostart_table_state.select(Some(i));
    }

    pub fn prev_autostart(&mut self) {
        let filtered = self.autostart_mgr.filtered_items();
        if filtered.is_empty() {
            return;
        }
        let i = match self.autostart_table_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.autostart_table_state.select(Some(i));
    }

    pub fn toggle_selected_autostart(&mut self) {
        let filtered = self.autostart_mgr.filtered_items();
        if let Some(sel) = self.autostart_table_state.selected() {
            if let Some(&(real_index, _)) = filtered.get(sel) {
                match self.autostart_mgr.toggle_item(real_index) {
                    Ok(msg) => self.show_toast(&msg, false),
                    Err(e) => self.show_toast(&e, true),
                }
            }
        }
    }

    // Applications navigation
    pub fn next_app(&mut self) {
        let filtered = self.app_mgr.filtered_items();
        if filtered.is_empty() {
            return;
        }
        let i = match self.app_table_state.selected() {
            Some(i) => {
                if i + 1 < filtered.len() {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.app_table_state.select(Some(i));
    }

    pub fn prev_app(&mut self) {
        let filtered = self.app_mgr.filtered_items();
        if filtered.is_empty() {
            return;
        }
        let i = match self.app_table_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.app_table_state.select(Some(i));
    }

    pub fn selected_app(&self) -> Option<ApplicationItem> {
        let filtered = self.app_mgr.filtered_items();
        let i = self.app_table_state.selected()?;
        filtered.get(i).map(|&item| item.clone())
    }

    pub fn execute_confirm_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::KillProcess(pid, name) => {
                match ProcessManager::kill_process(pid) {
                    Ok(()) => self.show_toast(&format!("Killed process '{}' (PID: {})", name, pid), false),
                    Err(e) => self.show_toast(&format!("Failed to kill PID {}: {}", pid, e), true),
                }
                self.refresh_processes();
            }
            ConfirmAction::TerminateProcess(pid, name) => {
                match ProcessManager::terminate_process(pid) {
                    Ok(()) => self.show_toast(&format!("Terminated process '{}' (PID: {})", name, pid), false),
                    Err(e) => self.show_toast(&format!("Failed to terminate PID {}: {}", pid, e), true),
                }
                self.refresh_processes();
            }
            ConfirmAction::StopProcess(pid, name) => {
                match ProcessManager::stop_process(pid) {
                    Ok(()) => self.show_toast(&format!("Paused process '{}' (PID: {})", name, pid), false),
                    Err(e) => self.show_toast(&format!("Failed to pause PID {}: {}", pid, e), true),
                }
                self.refresh_processes();
            }
            ConfirmAction::ResumeProcess(pid, name) => {
                match ProcessManager::resume_process(pid) {
                    Ok(()) => self.show_toast(&format!("Resumed process '{}' (PID: {})", name, pid), false),
                    Err(e) => self.show_toast(&format!("Failed to resume PID {}: {}", pid, e), true),
                }
                self.refresh_processes();
            }
            ConfirmAction::CleanCategories(_, _) => {
                self.execute_clean();
            }
            ConfirmAction::ServiceAction(act, unit) => {
                let pass = self.sudo_password.as_deref();
                let res = match act.as_str() {
                    "start" => self.service_mgr.start_service(&unit, pass),
                    "stop" => self.service_mgr.stop_service(&unit, pass),
                    "restart" => self.service_mgr.restart_service(&unit, pass),
                    "enable" => self.service_mgr.enable_service(&unit, pass),
                    "disable" => self.service_mgr.disable_service(&unit, pass),
                    _ => Err("Unknown service action".to_string()),
                };
                match res {
                    Ok(msg) => self.show_toast(&msg, false),
                    Err(e) => self.show_toast(&e, true),
                }
            }
            ConfirmAction::RemoveAutostart(index, _) => {
                match self.autostart_mgr.remove_entry(index) {
                    Ok(msg) => self.show_toast(&msg, false),
                    Err(e) => self.show_toast(&e, true),
                }
            }
            ConfirmAction::UninstallApp(app) => {
                let pass = self.sudo_password.as_deref();
                match self.app_mgr.uninstall_app(&app, pass) {
                    Ok(msg) => self.show_toast(&msg, false),
                    Err(e) => self.show_toast(&e, true),
                }
            }
            ConfirmAction::KillPort { port, proto, proc_name, pid } => {
                let pwd = self.sudo_password.as_deref();
                match self.network_mgr.kill_port(port, &proto, pid, pwd) {
                    Ok(_) => {
                        self.show_toast(&format!("✔ Terminated port {} ({})", port, proc_name), false);
                        self.network_mgr.refresh();
                        self.collector.refresh();
                    }
                    Err(e) => {
                        self.show_toast(&format!("Failed to kill port {}: {}", port, e), true);
                    }
                }
            }
        }
    }
}
