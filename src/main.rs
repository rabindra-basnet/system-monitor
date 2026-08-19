mod app;
mod system;
mod theme;
mod ui;

use std::env;
use std::io::stdout;
use std::panic;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::{App, AppTab, ConfirmAction, InputMode};
use crate::system::collector::format_bytes;

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

fn print_help() {
    println!(r#"
stasis 0.1.0 — Stacer-Inspired Lightweight Linux System Monitor & Optimizer

USAGE:
    stasis [OPTIONS]

OPTIONS:
    -t, --test, --diagnostics    Run non-interactive diagnostic telemetry check
    -h, --help                   Print help and keyboard navigation shortcuts
    -v, --version                Print version information

KEYBOARD SHORTCUTS:
    1 - 7 / F1 - F7              Switch active tabs
    Tab / Shift+Tab              Cycle through tabs forwards / backwards
    t                            Cycle color themes (Cyberpunk, Dracula, Nord, Monokai, Gruvbox)
    r                            Force instant telemetry refresh
    ?                            Open in-app Help and shortcuts modal
    q, Ctrl+C                    Quit application cleanly
    /                            Live search & filter (Processes, Services, Autostart, Apps)
"#);
}

fn run_diagnostics() -> Result<()> {
    println!("\x1b[1;36m====================================================\x1b[0m");
    println!("\x1b[1;36m       stasis System Diagnostics & Telemetry        \x1b[0m");
    println!("\x1b[1;36m====================================================\x1b[0m\n");

    let mut app = App::new();

    // 1. System Info
    println!("\x1b[1;32m[+] System Telemetry\x1b[0m");
    println!("  Host:           {}", app.collector.host_name);
    println!("  OS:             {}", app.collector.os_name);
    println!("  Kernel:         {}", app.collector.kernel_version);
    println!("  Uptime:         {}", app.collector.uptime_formatted());
    println!("  CPU:            {} ({} Cores)", app.collector.cpu_model, app.collector.cpu_count);
    println!("  Global CPU Load: {}%", app.collector.sys.global_cpu_usage().round() as u64);
    println!(
        "  Load Averages:  {:.2}, {:.2}, {:.2}",
        app.collector.load_avg_one, app.collector.load_avg_five, app.collector.load_avg_fifteen
    );

    // 2. Memory & Swap
    let total_mem = app.collector.sys.total_memory();
    let used_mem = app.collector.sys.used_memory();
    let total_swap = app.collector.sys.total_swap();
    let used_swap = app.collector.sys.used_swap();
    println!("\n\x1b[1;32m[+] Memory & Storage\x1b[0m");
    println!(
        "  RAM Usage:      {} / {} ({:.1}%)",
        format_bytes(used_mem),
        format_bytes(total_mem),
        if total_mem > 0 { (used_mem as f64 / total_mem as f64) * 100.0 } else { 0.0 }
    );
    println!(
        "  Swap Usage:     {} / {} ({:.1}%)",
        format_bytes(used_swap),
        format_bytes(total_swap),
        if total_swap > 0 { (used_swap as f64 / total_swap as f64) * 100.0 } else { 0.0 }
    );

    // 3. Disks
    for disk in app.collector.disks.iter() {
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total.saturating_sub(avail);
        println!(
            "  Mount: {} ({}) => {} / {} used",
            disk.mount_point().display(),
            disk.file_system().to_string_lossy(),
            format_bytes(used),
            format_bytes(total)
        );
    }

    // 4. Sensors
    println!("\n\x1b[1;32m[+] Thermal Sensors\x1b[0m");
    if app.sensor_collector.sensors.is_empty() {
        println!("  (No thermal sensors reported)");
    } else {
        for s in app.sensor_collector.sensors.iter().take(4) {
            println!("  Sensor [{}]: {:.1}°C", s.label, s.temperature);
        }
    }

    // 5. Processes
    println!("\n\x1b[1;32m[+] Process Management\x1b[0m");
    println!("  Total Processes: {}", app.process_list.len());
    println!("  Top 3 CPU Processes:");
    for p in app.process_list.iter().take(3) {
        println!("    - PID {}: {} ({:.1}% CPU, {})", p.pid, p.name, p.cpu_usage, format_bytes(p.memory_bytes));
    }

    // 6. System Cleaner Scan
    println!("\n\x1b[1;32m[+] Stacer System Cleaner Scan\x1b[0m");
    app.cleaner.scan();
    for cat in &app.cleaner.categories {
        println!(
            "  Category [{}]: {} ({} files)",
            cat.name,
            format_bytes(cat.total_size_bytes),
            cat.file_count
        );
    }
    println!(
        "  Total Reclaimable Space: {} across {} files",
        format_bytes(app.cleaner.total_scanned_bytes),
        app.cleaner.total_scanned_files
    );

    // 7. Services
    println!("\n\x1b[1;32m[+] systemd Services\x1b[0m");
    let active_count = app.service_mgr.services.iter().filter(|s| s.active_state == "active").count();
    let failed_count = app.service_mgr.services.iter().filter(|s| s.active_state == "failed").count();
    println!(
        "  Loaded Services: {} total ({} active, {} failed)",
        app.service_mgr.services.len(),
        active_count,
        failed_count
    );

    // 8. Autostart Applications
    println!("\n\x1b[1;32m[+] Autostart Applications\x1b[0m");
    let enabled_count = app.autostart_mgr.items.iter().filter(|i| i.enabled).count();
    println!(
        "  Startup Entries: {} total ({} enabled, {} disabled)",
        app.autostart_mgr.items.len(),
        enabled_count,
        app.autostart_mgr.items.len().saturating_sub(enabled_count)
    );

    // 9. Installed Packages & Applications
    println!("\n\x1b[1;32m[+] Installed Applications & Packages\x1b[0m");
    println!("  Detected Packages / Applications: {}", app.app_mgr.items.len());
    for item in app.app_mgr.items.iter().take(3) {
        println!("    - {} ({}) - {}", item.name, item.source, item.version);
    }

    println!("\n\x1b[1;32m✔ All subsystem checks PASSED successfully!\x1b[0m");
    println!("\x1b[1;33mTip: Run 'stasis' without arguments to launch the full interactive TUI!\x1b[0m\n");

    Ok(())
}

fn spawn_desktop_window(forward_args: &[String]) -> Result<()> {
    let current_exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("stasis"));
    let pass_args: Vec<String> = forward_args
        .iter()
        .filter(|a| *a != "--gui" && *a != "-g" && *a != "--window")
        .cloned()
        .collect();

    // 1. Try Native GTK3 Single-Window Wrapper (Zero Tabs, No '+' button, Clean Native GUI Window)
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let window_scripts = [
        format!("{}/.local/lib/stasis/stasis-window", home),
        "/usr/local/lib/stasis/stasis-window".to_string(),
        format!("{}/system-monitor/src/window.py", home),
    ];

    for script in &window_scripts {
        if std::path::Path::new(script).exists() {
            if std::process::Command::new("python3")
                .arg(script)
                .args(&pass_args)
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    // 2. Fallback to alacritty
    if std::process::Command::new("alacritty")
        .args([
            "--class", "stasis,stasis",
            "--title", "Stasis — System Optimizer",
            "-o", "window.dimensions.columns=134",
            "-o", "window.dimensions.lines=38",
            "-e",
        ])
        .arg(&current_exe)
        .arg("-i")
        .args(&pass_args)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // 3. Fallback to kitty
    if std::process::Command::new("kitty")
        .args([
            "--class=stasis",
            "--app-id=stasis",
            "--title=Stasis — System Optimizer",
            "-o", "initial_window_width=134c",
            "-o", "initial_window_height=38c",
        ])
        .arg(&current_exe)
        .arg("-i")
        .args(&pass_args)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // 4. Fallback to gnome-terminal
    if std::process::Command::new("gnome-terminal")
        .args([
            "--class=stasis",
            "--name=stasis",
            "--title=Stasis — System Optimizer",
            "--geometry=134x38",
            "--hide-menubar",
            "--",
        ])
        .arg(&current_exe)
        .arg("-i")
        .args(&pass_args)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    Err(anyhow::anyhow!("No supported window manager or terminal emulator found"))
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut initial_tab = AppTab::Dashboard;
    let mut force_inline = false;

    if args.len() > 1 {
        for arg in &args[1..] {
            match arg.as_str() {
                "-i" | "--inline" | "--cli" => {
                    force_inline = true;
                }
                "-g" | "--gui" => {
                    // GUI is default
                }
                "-h" | "--help" => {
                    print_help();
                    return Ok(());
                }
                "-v" | "--version" => {
                    println!("stasis 0.1.0");
                    return Ok(());
                }
                "-t" | "--test" | "--diagnostics" => {
                    return run_diagnostics();
                }
                "-p" | "--tab=processes" | "--processes" => {
                    initial_tab = AppTab::Processes;
                }
                "-c" | "--tab=cleaner" | "--cleaner" => {
                    initial_tab = AppTab::Cleaner;
                }
                "-s" | "--tab=services" | "--services" => {
                    initial_tab = AppTab::Services;
                }
                "--tab=autostart" | "--autostart" => {
                    initial_tab = AppTab::Autostart;
                }
                "-a" | "--tab=apps" | "--tab=applications" | "--apps" => {
                    initial_tab = AppTab::Applications;
                }
                "-r" | "--tab=resources" | "--resources" => {
                    initial_tab = AppTab::Dashboard;
                }
                unknown if unknown.starts_with('-') => {
                    eprintln!("Unknown argument: {}\nRun 'stasis --help' for usage.", unknown);
                    std::process::exit(1);
                }
                _ => {}
            }
        }
    }

    // By default, `stasis` opens the dedicated native desktop application directly!
    // Pass `-i` or `--cli` to run inside the existing terminal shell.
    if !force_inline {
        if spawn_desktop_window(&args[1..]).is_ok() {
            return Ok(());
        }
    }

    setup_panic_hook();

    // 1. Initialize app state BEFORE entering alternate screen to avoid any blank screen delay
    let mut app = App::new();
    app.active_tab = initial_tab;
    if initial_tab == AppTab::Processes {
        app.refresh_processes();
    }

    // 2. Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Render initial frame IMMEDIATELY (0ms blank screen)
    terminal.draw(|f| ui::render(f, &mut app))?;

    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = Instant::now();

    let res = run_app(&mut terminal, &mut app, tick_rate, &mut last_tick);

    // Teardown terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error running stasis: {err:?}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
    last_tick: &mut Instant,
) -> Result<()> {
    // Initial draw to ensure instantaneous UI presentation
    terminal.draw(|f| ui::render(f, app))?;

    while !app.should_quit {
        let timeout = Duration::from_millis(50);

        if event::poll(timeout)? {
            let mut need_redraw = false;
            while event::poll(Duration::from_millis(0))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            handle_key_event(app, key.code, key.modifiers);
                            need_redraw = true;
                        }
                    }
                    Event::Mouse(mouse) => {
                        let term_size = terminal.size()?;
                        handle_mouse_event(app, mouse, term_size.width, term_size.height);
                        need_redraw = true;
                    }
                    _ => {}
                }
            }
            if need_redraw {
                terminal.draw(|f| ui::render(f, app))?;
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();
            *last_tick = Instant::now();
            terminal.draw(|f| ui::render(f, app))?;
        }
    }

    Ok(())
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent, width: u16, height: u16) {
    // Handle modals first
    match &app.input_mode {
        InputMode::HelpModal => {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                app.input_mode = InputMode::Normal;
            }
            return;
        }
        InputMode::ConfirmModal(action) => {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let action = action.clone();
                let center_y = height / 2;
                // If clicked around confirmation buttons row
                if mouse.row >= center_y.saturating_sub(1) && mouse.row <= center_y + 3 {
                    let center_x = width / 2;
                    if mouse.column < center_x {
                        // Clicked Confirm [Y]
                        let needs_elevation = action.requires_elevation(app.service_mgr.user_mode);
                        if needs_elevation && !app.is_root() && app.sudo_password.is_none() {
                            app.input_mode = InputMode::SudoPasswordModal {
                                pending_action: Box::new(action),
                                password: String::new(),
                                error_msg: None,
                            };
                        } else {
                            app.execute_confirm_action(action);
                            app.input_mode = InputMode::Normal;
                        }
                    } else {
                        // Clicked Cancel [N]
                        app.input_mode = InputMode::Normal;
                    }
                } else if mouse.row < center_y.saturating_sub(5) || mouse.row > center_y + 6 {
                    // Clicked outside modal
                    app.input_mode = InputMode::Normal;
                }
            }
            return;
        }
        InputMode::SudoPasswordModal { .. } => {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let center_y = height / 2;
                if mouse.row < center_y.saturating_sub(6) || mouse.row > center_y + 6 {
                    app.input_mode = InputMode::Normal;
                }
            }
            return;
        }
        _ => {}
    }

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            match app.active_tab {
                AppTab::Processes => app.next_process(),
                AppTab::Applications => app.next_app(),
                AppTab::Services => app.next_service(),
                AppTab::Autostart => app.next_autostart(),
                AppTab::Cleaner => app.next_cleaner_category(),
                _ => {}
            }
        }
        MouseEventKind::ScrollUp => {
            match app.active_tab {
                AppTab::Processes => app.prev_process(),
                AppTab::Applications => app.prev_app(),
                AppTab::Services => app.prev_service(),
                AppTab::Autostart => app.prev_autostart(),
                AppTab::Cleaner => app.prev_cleaner_category(),
                _ => {}
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // 1. Top Tab Bar Click (Rows 0, 1, 2)
            if mouse.row <= 2 {
                let tabs = AppTab::all();
                let mut current_x = 1u16;
                let mut clicked_tab = None;

                for (i, tab) in tabs.iter().enumerate() {
                    let title_text = format!(" [{}] {}", i + 1, tab.title());
                    let tab_len = (title_text.chars().count() + 1) as u16;
                    let divider_len = if i + 1 < tabs.len() { 3u16 } else { 0u16 };

                    if mouse.column >= current_x && mouse.column < current_x + tab_len + divider_len {
                        clicked_tab = Some(*tab);
                        break;
                    }
                    current_x += tab_len + divider_len;
                }

                // Fallback to proportional division if clicked further to the right
                if clicked_tab.is_none() && mouse.column < width {
                    let tab_count = tabs.len() as u16;
                    let tab_width = width / tab_count;
                    if tab_width > 0 {
                        let idx = (mouse.column / tab_width).min(tab_count - 1) as usize;
                        clicked_tab = Some(tabs[idx]);
                    }
                }

                if let Some(tab) = clicked_tab {
                    app.active_tab = tab;
                    if tab == AppTab::Processes {
                        app.refresh_processes();
                    }
                }
                return;
            }

            // 2. Search Bar Click (Rows 3, 4, 5)
            if mouse.row >= 3 && mouse.row <= 5 {
                match app.active_tab {
                    AppTab::Processes => {
                        app.search_input = app.process_mgr.filter.clone();
                        app.input_mode = InputMode::Search;
                    }
                    AppTab::Services => {
                        app.search_input = app.service_mgr.search_query.clone();
                        app.input_mode = InputMode::Search;
                    }
                    AppTab::Autostart => {
                        app.search_input = app.autostart_mgr.search_query.clone();
                        app.input_mode = InputMode::Search;
                    }
                    AppTab::Applications => {
                        app.search_input = app.app_mgr.search_query.clone();
                        app.input_mode = InputMode::Search;
                    }
                    _ => {}
                }
                return;
            }

            // 3. Table / Item Click inside active tab (Rows >= 6)
            match app.active_tab {
                AppTab::Processes => {
                    // Header is at row 7, data rows start at row 9
                    if mouse.row >= 9 && mouse.row < height.saturating_sub(2) {
                        let visible_row = (mouse.row - 9) as usize;
                        let offset = app.process_table_state.offset();
                        let target_idx = offset + visible_row;
                        if target_idx < app.process_list.len() {
                            app.process_table_state.select(Some(target_idx));
                        }
                    }
                }
                AppTab::Cleaner => {
                    // Categories start at row 4, each category is 3 lines
                    if mouse.row >= 4 && mouse.row < height.saturating_sub(2) {
                        let rel_y = (mouse.row - 4) as usize;
                        let cat_idx = rel_y / 3;
                        if cat_idx < app.cleaner.categories.len() {
                            app.cleaner_selected_index = cat_idx;
                            app.toggle_cleaner_category();
                        }
                    }
                }
                AppTab::Services => {
                    // Header is at row 7, data rows start at row 9
                    if mouse.row >= 9 && mouse.row < height.saturating_sub(2) {
                        let visible_row = (mouse.row - 9) as usize;
                        let offset = app.service_table_state.offset();
                        let filtered = app.service_mgr.filtered_services();
                        let target_idx = offset + visible_row;
                        if target_idx < filtered.len() {
                            app.service_table_state.select(Some(target_idx));
                        }
                    }
                }
                AppTab::Applications => {
                    // Header is at row 7, data rows start at row 9
                    if mouse.row >= 9 && mouse.row < height.saturating_sub(2) {
                        let visible_row = (mouse.row - 9) as usize;
                        let offset = app.app_table_state.offset();
                        let filtered = app.app_mgr.filtered_items();
                        let target_idx = offset + visible_row;
                        if target_idx < filtered.len() {
                            app.app_table_state.select(Some(target_idx));
                        }
                    }
                }
                AppTab::Autostart => {
                    // Header is at row 7, data rows start at row 9
                    if mouse.row >= 9 && mouse.row < height.saturating_sub(2) {
                        let visible_row = (mouse.row - 9) as usize;
                        let offset = app.autostart_table_state.offset();
                        let filtered = app.autostart_mgr.filtered_items();
                        let target_idx = offset + visible_row;
                        if target_idx < filtered.len() {
                            app.autostart_table_state.select(Some(target_idx));
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Global quit on Ctrl+C
    if modifiers.contains(KeyModifiers::CONTROL) && (code == KeyCode::Char('c') || code == KeyCode::Char('C')) {
        app.should_quit = true;
        return;
    }

    match &app.input_mode {
        InputMode::ConfirmModal(action) => {
            let action_clone = action.clone();
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let needs_elevation = action_clone.requires_elevation(app.service_mgr.user_mode);
                    if needs_elevation && !app.is_root() && app.sudo_password.is_none() {
                        app.input_mode = InputMode::SudoPasswordModal {
                            pending_action: Box::new(action_clone),
                            password: String::new(),
                            error_msg: None,
                        };
                    } else {
                        app.input_mode = InputMode::Normal;
                        app.execute_confirm_action(action_clone);
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                }
                _ => {}
            }
        }
        InputMode::SudoPasswordModal {
            pending_action,
            password,
            error_msg: _,
        } => {
            let mut password = password.clone();
            let action = *pending_action.clone();

            match code {
                KeyCode::Enter => {
                    if password.is_empty() {
                        app.input_mode = InputMode::SudoPasswordModal {
                            pending_action: Box::new(action),
                            password,
                            error_msg: Some("Password cannot be empty".to_string()),
                        };
                    } else if crate::system::sudo::validate_sudo_password(&password) {
                        app.sudo_password = Some(password);
                        app.input_mode = InputMode::Normal;
                        app.show_toast("🔒 Sudo authenticated — executing action...", false);
                        app.execute_confirm_action(action);
                    } else {
                        app.input_mode = InputMode::SudoPasswordModal {
                            pending_action: Box::new(action),
                            password: String::new(),
                            error_msg: Some("Authentication failed: Incorrect sudo password".to_string()),
                        };
                    }
                }
                KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    password.pop();
                    app.input_mode = InputMode::SudoPasswordModal {
                        pending_action: Box::new(action),
                        password,
                        error_msg: None,
                    };
                }
                KeyCode::Char(c) => {
                    password.push(c);
                    app.input_mode = InputMode::SudoPasswordModal {
                        pending_action: Box::new(action),
                        password,
                        error_msg: None,
                    };
                }
                _ => {}
            }
        }
        InputMode::HelpModal => match code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        InputMode::Search => match code {
            KeyCode::Enter => {
                let query = app.search_input.clone();
                match app.active_tab {
                    AppTab::Processes => {
                        app.process_mgr.filter = query;
                        app.refresh_processes();
                    }
                    AppTab::Services => {
                        app.service_mgr.search_query = query;
                        app.service_table_state.select(Some(0));
                    }
                    AppTab::Autostart => {
                        app.autostart_mgr.search_query = query;
                        app.autostart_table_state.select(Some(0));
                    }
                    AppTab::Applications => {
                        app.app_mgr.search_query = query;
                        app.app_table_state.select(Some(0));
                    }
                    _ => {}
                }
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                app.search_input.clear();
                match app.active_tab {
                    AppTab::Processes => {
                        app.process_mgr.filter.clear();
                        app.refresh_processes();
                    }
                    AppTab::Services => {
                        app.service_mgr.search_query.clear();
                    }
                    AppTab::Autostart => {
                        app.autostart_mgr.search_query.clear();
                    }
                    AppTab::Applications => {
                        app.app_mgr.search_query.clear();
                    }
                    _ => {}
                }
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                app.search_input.pop();
            }
            KeyCode::Char(c) => {
                app.search_input.push(c);
            }
            _ => {}
        },
        InputMode::NewAutostartModal {
            name,
            exec,
            comment,
            active_field,
        } => {
            let mut name = name.clone();
            let mut exec = exec.clone();
            let mut comment = comment.clone();
            let mut active_field = *active_field;

            match code {
                KeyCode::Tab | KeyCode::Down => {
                    active_field = (active_field + 1) % 3;
                    app.input_mode = InputMode::NewAutostartModal {
                        name,
                        exec,
                        comment,
                        active_field,
                    };
                }
                KeyCode::BackTab | KeyCode::Up => {
                    active_field = if active_field == 0 { 2 } else { active_field - 1 };
                    app.input_mode = InputMode::NewAutostartModal {
                        name,
                        exec,
                        comment,
                        active_field,
                    };
                }
                KeyCode::Enter => {
                    match app.autostart_mgr.add_entry(&name, &exec, &comment) {
                        Ok(msg) => app.show_toast(&msg, false),
                        Err(e) => app.show_toast(&e, true),
                    }
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    match active_field {
                        0 => {
                            name.pop();
                        }
                        1 => {
                            exec.pop();
                        }
                        2 => {
                            comment.pop();
                        }
                        _ => {}
                    }
                    app.input_mode = InputMode::NewAutostartModal {
                        name,
                        exec,
                        comment,
                        active_field,
                    };
                }
                KeyCode::Char(c) => {
                    match active_field {
                        0 => name.push(c),
                        1 => exec.push(c),
                        2 => comment.push(c),
                        _ => {}
                    }
                    app.input_mode = InputMode::NewAutostartModal {
                        name,
                        exec,
                        comment,
                        active_field,
                    };
                }
                _ => {}
            }
        }
        InputMode::Normal => {
            // Global keybindings
            match code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    app.should_quit = true;
                    return;
                }
                KeyCode::Char('?') => {
                    app.input_mode = InputMode::HelpModal;
                    return;
                }
                KeyCode::Char('t') | KeyCode::Char('T') if app.active_tab != AppTab::Processes => {
                    app.cycle_theme();
                    return;
                }
                KeyCode::Char('r') | KeyCode::Char('R') if app.active_tab != AppTab::Services => {
                    app.tick();
                    app.show_toast("Refreshed system telemetry", false);
                    return;
                }
                KeyCode::Tab => {
                    app.active_tab = app.active_tab.next();
                    if app.active_tab == AppTab::Processes {
                        app.refresh_processes();
                    }
                    return;
                }
                KeyCode::BackTab => {
                    app.active_tab = app.active_tab.prev();
                    if app.active_tab == AppTab::Processes {
                        app.refresh_processes();
                    }
                    return;
                }
                KeyCode::Char('1') | KeyCode::F(1) => {
                    app.active_tab = AppTab::Dashboard;
                    return;
                }
                KeyCode::Char('2') | KeyCode::F(2) => {
                    app.active_tab = AppTab::Processes;
                    app.refresh_processes();
                    return;
                }
                KeyCode::Char('3') | KeyCode::F(3) => {
                    app.active_tab = AppTab::Cleaner;
                    return;
                }
                KeyCode::Char('4') | KeyCode::F(4) => {
                    app.active_tab = AppTab::Services;
                    return;
                }
                KeyCode::Char('5') | KeyCode::F(5) => {
                    app.active_tab = AppTab::Autostart;
                    return;
                }
                KeyCode::Char('6') | KeyCode::F(6) => {
                    app.active_tab = AppTab::Applications;
                    return;
                }
                _ => {}
            }

            // Tab-specific keybindings
            match app.active_tab {
                AppTab::Dashboard => match code {
                    KeyCode::Char('r') => {
                        app.tick();
                    }
                    _ => {}
                },
                AppTab::Processes => match code {
                    KeyCode::Down | KeyCode::Char('j') => app.next_process(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev_process(),
                    KeyCode::PageDown => app.page_down_processes(),
                    KeyCode::PageUp => app.page_up_processes(),
                    KeyCode::Char('s') => app.cycle_process_sort(),
                    KeyCode::Char('d') => app.toggle_process_sort_direction(),
                    KeyCode::Char('/') => {
                        app.search_input = app.process_mgr.filter.clone();
                        app.input_mode = InputMode::Search;
                    }
                    KeyCode::Char('K') | KeyCode::Char('x') | KeyCode::Delete => {
                        if let Some(p) = app.selected_process() {
                            if p.is_critical {
                                app.show_toast(
                                    &format!("🚫 Action Blocked: PID {} ({}) is a core system process and cannot be killed.", p.pid, p.name),
                                    true,
                                );
                            } else {
                                let pid = p.pid;
                                let name = p.name.clone();
                                app.input_mode =
                                    InputMode::ConfirmModal(ConfirmAction::KillProcess(pid, name));
                            }
                        }
                    }
                    KeyCode::Char('t') => {
                        if let Some(p) = app.selected_process() {
                            let pid = p.pid;
                            let name = p.name.clone();
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::TerminateProcess(pid, name),
                            );
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Some(p) = app.selected_process() {
                            let pid = p.pid;
                            let name = p.name.clone();
                            app.input_mode =
                                InputMode::ConfirmModal(ConfirmAction::StopProcess(pid, name));
                        }
                    }
                    KeyCode::Char('c') => {
                        if let Some(p) = app.selected_process() {
                            let pid = p.pid;
                            let name = p.name.clone();
                            app.input_mode =
                                InputMode::ConfirmModal(ConfirmAction::ResumeProcess(pid, name));
                        }
                    }
                    _ => {}
                },
                AppTab::Cleaner => match code {
                    KeyCode::Down | KeyCode::Char('j') => app.next_cleaner_category(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev_cleaner_category(),
                    KeyCode::Char(' ') => app.toggle_cleaner_category(),
                    KeyCode::Char('a') => {
                        let any_selected = app.cleaner.categories.iter().any(|c| c.selected);
                        app.cleaner.select_all(!any_selected);
                    }
                    KeyCode::Char('s') => app.scan_cleaner(),
                    KeyCode::Char('c') | KeyCode::Enter => {
                        let selected_names: Vec<String> = app
                            .cleaner
                            .categories
                            .iter()
                            .filter(|c| c.selected)
                            .map(|c| c.name.clone())
                            .collect();
                        if selected_names.is_empty() {
                            app.show_toast("No cleaner categories selected", true);
                        } else {
                            let bytes = app.cleaner.total_scanned_bytes;
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::CleanCategories(selected_names, bytes),
                            );
                        }
                    }
                    _ => {}
                },
                AppTab::Services => match code {
                    KeyCode::Down | KeyCode::Char('j') => app.next_service(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev_service(),
                    KeyCode::Char('f') => app.cycle_service_filter(),
                    KeyCode::Char('u') => app.toggle_service_mode(),
                    KeyCode::Char('/') => {
                        app.search_input = app.service_mgr.search_query.clone();
                        app.input_mode = InputMode::Search;
                    }
                    KeyCode::Char('s') => {
                        if let Some(unit) = app.selected_service_unit() {
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::ServiceAction("start".to_string(), unit),
                            );
                        }
                    }
                    KeyCode::Char('x') => {
                        if let Some(unit) = app.selected_service_unit() {
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::ServiceAction("stop".to_string(), unit),
                            );
                        }
                    }
                    KeyCode::Char('r') => {
                        if let Some(unit) = app.selected_service_unit() {
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::ServiceAction("restart".to_string(), unit),
                            );
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(unit) = app.selected_service_unit() {
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::ServiceAction("enable".to_string(), unit),
                            );
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(unit) = app.selected_service_unit() {
                            app.input_mode = InputMode::ConfirmModal(
                                ConfirmAction::ServiceAction("disable".to_string(), unit),
                            );
                        }
                    }
                    _ => {}
                },
                AppTab::Autostart => match code {
                    KeyCode::Down | KeyCode::Char('j') => app.next_autostart(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev_autostart(),
                    KeyCode::Char(' ') | KeyCode::Enter => app.toggle_selected_autostart(),
                    KeyCode::Char('n') => {
                        app.input_mode = InputMode::NewAutostartModal {
                            name: String::new(),
                            exec: String::new(),
                            comment: String::new(),
                            active_field: 0,
                        };
                    }
                    KeyCode::Char('d') | KeyCode::Delete => {
                        let filtered = app.autostart_mgr.filtered_items();
                        if let Some(sel) = app.autostart_table_state.selected() {
                            if let Some(&(real_idx, item)) = filtered.get(sel) {
                                if !item.is_user {
                                    app.show_toast("Cannot delete system autostart. Press Space to disable instead.", true);
                                } else {
                                    app.input_mode = InputMode::ConfirmModal(
                                        ConfirmAction::RemoveAutostart(real_idx, item.name.clone()),
                                    );
                                }
                            }
                        }
                    }
                    KeyCode::Char('/') => {
                        app.search_input = app.autostart_mgr.search_query.clone();
                        app.input_mode = InputMode::Search;
                    }
                    _ => {}
                },
                AppTab::Applications => match code {
                    KeyCode::Down | KeyCode::Char('j') => app.next_app(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev_app(),
                    KeyCode::Char('f') => {
                        app.app_mgr.source_filter = app.app_mgr.source_filter.next();
                        app.app_table_state.select(Some(0));
                        app.show_toast(&format!("Filter: {}", app.app_mgr.source_filter.label()), false);
                    }
                    KeyCode::Char('s') => {
                        app.app_mgr.cycle_sort();
                        app.show_toast(&format!("Sort: {:?}", app.app_mgr.sort_by), false);
                    }
                    KeyCode::Char('d') => {
                        app.app_mgr.toggle_sort_direction();
                        app.show_toast(&format!("Sort direction: {}", if app.app_mgr.sort_descending { "Descending" } else { "Ascending" }), false);
                    }
                    KeyCode::Char('/') => {
                        app.search_input = app.app_mgr.search_query.clone();
                        app.input_mode = InputMode::Search;
                    }
                    KeyCode::Char('u') | KeyCode::Delete => {
                        if let Some(item) = app.selected_app() {
                            if item.is_essential {
                                app.show_toast(
                                    &format!("🚫 Action Blocked: '{}' is a protected system component and cannot be uninstalled.", item.name),
                                    true,
                                );
                            } else {
                                app.input_mode = InputMode::ConfirmModal(
                                    ConfirmAction::UninstallApp(item),
                                );
                            }
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}
