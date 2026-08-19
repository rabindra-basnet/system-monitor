use stasis::app::{App, AppTab, ConfirmAction};
use stasis::system::applications::{AppSortBy, AppSourceFilter, ApplicationManager};
use stasis::system::cleaner::SystemCleaner;
use stasis::system::collector::{format_bytes, format_speed, SystemCollector};
use stasis::system::processes::ProcessManager;
use stasis::system::sudo::is_root;
use stasis::theme::{Theme, ThemeMode};

#[test]
fn test_byte_formatting() {
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
}

#[test]
fn test_speed_formatting() {
    assert_eq!(format_speed(1024), "1.0 KB/s");
    assert_eq!(format_speed(5 * 1024 * 1024), "5.0 MB/s");
}

#[test]
fn test_theme_transitions() {
    let mut theme = Theme::new(ThemeMode::Cyberpunk);
    assert_eq!(theme.mode, ThemeMode::Cyberpunk);

    theme = Theme::new(theme.mode.next());
    assert_eq!(theme.mode, ThemeMode::Dracula);

    theme = Theme::new(theme.mode.next());
    assert_eq!(theme.mode, ThemeMode::Nord);

    theme = Theme::new(theme.mode.next());
    assert_eq!(theme.mode, ThemeMode::Monokai);

    theme = Theme::new(theme.mode.next());
    assert_eq!(theme.mode, ThemeMode::Gruvbox);

    theme = Theme::new(theme.mode.next());
    assert_eq!(theme.mode, ThemeMode::Cyberpunk);
}

#[test]
fn test_app_tab_cycling() {
    let tab = AppTab::Dashboard;
    assert_eq!(tab.next(), AppTab::Network);
    assert_eq!(tab.next().next(), AppTab::Processes);
    assert_eq!(tab.prev(), AppTab::Applications);
}

#[test]
fn test_system_collector_initialization() {
    let mut collector = SystemCollector::new();
    assert!(!collector.os_name.is_empty());
    assert!(!collector.host_name.is_empty());
    assert!(collector.cpu_count > 0);
    assert_eq!(collector.cpu_history.len(), 60);
    assert_eq!(collector.mem_history.len(), 60);

    collector.refresh();
    assert!(!collector.uptime_formatted().is_empty());
}

#[test]
fn test_cleaner_scan_and_selection() {
    let mut cleaner = SystemCleaner::new();
    assert!(!cleaner.categories.is_empty());

    cleaner.scan();
    let initial_selected = cleaner.categories.iter().filter(|c| c.selected).count();
    assert!(initial_selected > 0);

    // Toggle all off
    cleaner.select_all(false);
    assert_eq!(cleaner.total_scanned_bytes, 0);

    // Toggle all on
    cleaner.select_all(true);
    let all_bytes: u64 = cleaner.categories.iter().map(|c| c.total_size_bytes).sum();
    assert_eq!(cleaner.total_scanned_bytes, all_bytes);
}

#[test]
fn test_process_manager_filtering() {
    let mut proc_mgr = ProcessManager::new();
    let sys = sysinfo::System::new_all();
    let initial = proc_mgr.get_processes(&sys);
    assert!(!initial.is_empty());

    // Search for nonexistent process
    proc_mgr.filter = "nonexistent_proc_xyz_123456789".to_string();
    let filtered = proc_mgr.get_processes(&sys);
    assert!(filtered.is_empty());

    // Clear filter
    proc_mgr.filter.clear();
    let restored = proc_mgr.get_processes(&sys);
    assert_eq!(restored.len(), initial.len());
}

#[test]
fn test_application_manager() {
    let mut app_mgr = ApplicationManager::new();
    let initial_len = app_mgr.items.len();
    assert!(initial_len > 0);

    // Test filter transition
    assert_eq!(app_mgr.source_filter, AppSourceFilter::All);
    app_mgr.source_filter = app_mgr.source_filter.next();
    assert_eq!(app_mgr.source_filter, AppSourceFilter::UserInstalled);

    // Test sort transition
    assert_eq!(app_mgr.sort_by, AppSortBy::Size);
    app_mgr.cycle_sort();
    assert_eq!(app_mgr.sort_by, AppSortBy::Age);
    app_mgr.cycle_sort();
    assert_eq!(app_mgr.sort_by, AppSortBy::Name);
    app_mgr.cycle_sort();
    assert_eq!(app_mgr.sort_by, AppSortBy::Source);
}

#[test]
fn test_sudo_elevation_detection() {
    let _root = is_root();
    let sys_clean_action = ConfirmAction::CleanCategories(vec!["Package Caches".to_string()], 100);
    assert!(sys_clean_action.requires_elevation(false));

    let user_service_action =
        ConfirmAction::ServiceAction("start".to_string(), "test.service".to_string());
    assert!(!user_service_action.requires_elevation(true));
    assert!(user_service_action.requires_elevation(false));
}

#[test]
fn test_app_state_initialization() {
    let app = App::new();
    assert_eq!(app.active_tab, AppTab::Dashboard);
    assert!(!app.should_quit);
    assert!(!app.process_list.is_empty());
}

#[test]
fn test_network_manager() {
    let mut net_mgr = stasis::system::network::NetworkManager::new();
    net_mgr.refresh();
    assert!(net_mgr.summary.listening_ports.len() <= net_mgr.summary.total_sockets);
}

#[test]
fn test_gpu_collector() {
    let mut gpu_col = stasis::system::gpu::GpuCollector::new();
    gpu_col.refresh();
    // Verify it doesn't panic on systems with or without GPU
    let _ = gpu_col.is_available;
}
