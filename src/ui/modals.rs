use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::{App, ConfirmAction, InputMode};

pub fn render_modals(f: &mut Frame, app: &App) {
    match &app.input_mode {
        InputMode::ConfirmModal(action) => render_confirm_modal(f, app, action),
        InputMode::SudoPasswordModal { pending_action, password, error_msg } => {
            render_sudo_password_modal(f, app, pending_action, password, error_msg.as_deref());
        }
        InputMode::HelpModal => render_help_modal(f, app),
        InputMode::NewAutostartModal { name, exec, comment, active_field } => {
            render_new_autostart_modal(f, app, name, exec, comment, *active_field);
        }
        InputMode::Search => render_search_modal(f, app),
        InputMode::Normal => {}
    }
}

pub fn render_toast(f: &mut Frame, app: &App, area: Rect) {
    if let Some(toast) = &app.toast {
        let theme = &app.theme;
        let toast_style = if toast.is_error {
            Style::default().fg(theme.fg).bg(theme.danger).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.bg).bg(theme.success).add_modifier(Modifier::BOLD)
        };

        let msg = format!("  {}  ", toast.message);
        let width = (msg.len() as u16 + 4).min(area.width.saturating_sub(4));
        let toast_area = Rect {
            x: area.width.saturating_sub(width + 2),
            y: area.height.saturating_sub(4),
            width,
            height: 3,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(toast_style)
            .style(toast_style);

        let p = Paragraph::new(msg).block(block).alignment(Alignment::Center);
        f.render_widget(Clear, toast_area);
        f.render_widget(p, toast_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_confirm_modal(f: &mut Frame, app: &App, action: &ConfirmAction) {
    let theme = &app.theme;
    let area = centered_rect(50, 30, f.area());

    f.render_widget(Clear, area);

    let (title, message) = match action {
        ConfirmAction::KillProcess(pid, name) => {
            let is_sys = *pid <= 2 || name == "systemd" || name.starts_with("kworker");
            let warn = if is_sys {
                "\n\n🚨 WARNING: This is a CORE SYSTEM PROCESS! Killing it may cause an immediate crash or kernel panic!"
            } else {
                "\n\nThis sends SIGKILL immediately."
            };
            (
                " ⚠️ Confirm KILL Process ",
                format!("Are you sure you want to force kill process '{}' (PID: {})?{}", name, pid, warn),
            )
        }
        ConfirmAction::TerminateProcess(pid, name) => (
            " ⚠️ Confirm Terminate Process ",
            format!("Are you sure you want to gracefully terminate '{}' (PID: {})?\n\nThis sends SIGTERM.", name, pid),
        ),
        ConfirmAction::StopProcess(pid, name) => (
            " ⏸️ Confirm Pause Process ",
            format!("Are you sure you want to pause '{}' (PID: {})?\n\nThis sends SIGSTOP.", name, pid),
        ),
        ConfirmAction::ResumeProcess(pid, name) => (
            " ▶️ Confirm Resume Process ",
            format!("Are you sure you want to resume execution of '{}' (PID: {})?\n\nThis sends SIGCONT.", name, pid),
        ),
        ConfirmAction::CleanCategories(cats, bytes) => (
            " 󰃢 Confirm System Clean ",
            format!("Are you sure you want to clean {} categories (freeing ~{})?\n\nSelected items: {}", cats.len(), crate::system::collector::format_bytes(*bytes), cats.join(", ")),
        ),
        ConfirmAction::ServiceAction(act, unit) => (
            "  Confirm Service Action ",
            format!("Perform '{}' on systemd service '{}'?", act, unit),
        ),
        ConfirmAction::RemoveAutostart(_, name) => (
            " 󱑞 Confirm Remove Autostart ",
            format!("Remove startup configuration for '{}' from ~/.config/autostart?", name),
        ),
        ConfirmAction::UninstallApp(app_item) => {
            let warn = if app_item.is_essential {
                "\n\n🚨 WARNING: This is a CORE SYSTEM COMPONENT! Removing it may render your Linux system unbootable!"
            } else {
                ""
            };
            (
                " 󰏖 Confirm Application Uninstallation ",
                format!("Are you sure you want to uninstall '{}' ({}) via {}?\n\nPackage ID: {}{}", app_item.name, app_item.version, app_item.source, app_item.package_id, warn),
            )
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.danger))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(title, Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(3)])
        .split(inner);

    let p_msg = Paragraph::new(message).style(Style::default().fg(theme.fg)).wrap(Wrap { trim: true });
    f.render_widget(p_msg, chunks[0]);

    let controls = Paragraph::new(Line::from(vec![
        Span::styled(" [Y] / [Enter] Confirm ", Style::default().fg(theme.bg).bg(theme.danger).add_modifier(Modifier::BOLD)),
        Span::raw("     "),
        Span::styled(" [N] / [Esc] Cancel ", Style::default().fg(theme.fg).bg(theme.border)),
    ])).alignment(Alignment::Center);
    f.render_widget(controls, chunks[1]);
}

fn render_sudo_password_modal(
    f: &mut Frame,
    app: &App,
    action: &ConfirmAction,
    password: &str,
    error_msg: Option<&str>,
) {
    let theme = &app.theme;
    let area = centered_rect(55, 36, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.warning))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 🔒 Sudo / Root Authentication Required ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Action description
            Constraint::Length(3), // Password input box
            Constraint::Length(2), // Error message if any
            Constraint::Length(2), // Controls
        ])
        .split(inner);

    let action_desc = match action {
        ConfirmAction::CleanCategories(cats, _) => format!("Elevated privileges needed to purge system caches ({})", cats.join(", ")),
        ConfirmAction::ServiceAction(act, unit) => format!("Elevated privileges needed to {} systemd service '{}'", act, unit),
        ConfirmAction::UninstallApp(app) => format!("Elevated privileges needed to uninstall system package '{}'", app.name),
        ConfirmAction::KillProcess(pid, name) => format!("Elevated privileges needed to kill PID {} ({})", pid, name),
        _ => "Administrative superuser privileges required".to_string(),
    };

    let p_desc = Paragraph::new(action_desc).style(Style::default().fg(theme.fg)).wrap(Wrap { trim: true });
    f.render_widget(p_desc, chunks[0]);

    // Password input field (masked with bullet points)
    let masked_pw = "•".repeat(password.len());
    let pw_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" Sudo Password ", theme.title_style()));

    let pw_line = Line::from(vec![
        Span::styled(masked_pw, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        Span::styled("█", Style::default().fg(theme.accent)),
    ]);
    let p_pw = Paragraph::new(pw_line).block(pw_block);
    f.render_widget(p_pw, chunks[1]);

    // Error message display
    if let Some(err) = error_msg {
        let p_err = Paragraph::new(Span::styled(format!("❌ {}", err), Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)));
        f.render_widget(p_err, chunks[2]);
    } else {
        let p_hint = Paragraph::new(Span::styled("Enter your Linux sudo user password.", theme.dim_style()));
        f.render_widget(p_hint, chunks[2]);
    }

    let controls = Paragraph::new(Line::from(vec![
        Span::styled(" [Enter] Authenticate & Run ", Style::default().fg(theme.bg).bg(theme.success).add_modifier(Modifier::BOLD)),
        Span::raw("     "),
        Span::styled(" [Esc] Cancel ", Style::default().fg(theme.fg).bg(theme.border)),
    ])).alignment(Alignment::Center);
    f.render_widget(controls, chunks[3]);
}

fn render_search_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(50, 18, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  Live Search / Filter ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(inner);

    let input_display = Line::from(vec![
        Span::styled(" Query: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::styled(&app.search_input, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        Span::styled("█", Style::default().fg(theme.accent)),
    ]);
    let p_input = Paragraph::new(input_display);
    f.render_widget(p_input, rows[0]);

    let hints = Paragraph::new(Line::from(vec![
        Span::styled(" [Enter] Apply Search ", Style::default().fg(theme.success)),
        Span::raw("   "),
        Span::styled(" [Esc] Cancel ", theme.dim_style()),
    ])).alignment(Alignment::Right);
    f.render_widget(hints, rows[1]);
}

fn render_help_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(75, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  stasis Shortcuts & Reference Guide ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let shortcuts = [
        ("Global", "1-6 / F1-F6", "Switch active tabs directly"),
        ("Global", "Tab / Shift+Tab", "Cycle through tabs forwards / backwards"),
        ("Global", "t", "Cycle color themes (Cyberpunk, Dracula, Nord, Monokai, Gruvbox)"),
        ("Global", "r", "Force immediate telemetry refresh"),
        ("Global", "?", "Open / Close this Help & Keybindings modal"),
        ("Global", "q / Ctrl+C", "Quit stasis cleanly"),
        ("Processes", "↑/↓ or j/k", "Navigate process list (PageUp/PageDown for fast scroll)"),
        ("Processes", "s / d", "Cycle sort column (s) / Toggle sort direction (d)"),
        ("Processes", "/", "Open live process substring search filter"),
        ("Processes", "K / x / Del", "Kill process (SIGKILL)"),
        ("Processes", "t", "Terminate process (SIGTERM)"),
        ("Processes", "p / c", "Pause process (SIGSTOP) / Continue process (SIGCONT)"),
        ("Cleaner", "Space", "Toggle selection checkbox for active cleaner category"),
        ("Cleaner", "a", "Select All / Deselect All cleaner categories"),
        ("Cleaner", "s", "Scan system files to calculate reclaimable space"),
        ("Cleaner", "c / Enter", "Clean selected cache categories (prompts sudo if needed)"),
        ("Services", "u", "Toggle systemctl scope: System (root) ⇄ User (--user)"),
        ("Services", "f", "Cycle service state filter: All → Active → Inactive → Failed"),
        ("Services", "s / x / r", "Start (s) / Stop (x) / Restart (r) selected service unit"),
        ("Services", "e / d", "Enable at boot (e) / Disable from boot (d) selected service"),
        ("Autostart", "Space / Enter", "Toggle application autostart status (Enabled ⇄ Disabled)"),
        ("Autostart", "n", "Add new autostart application (.desktop generator)"),
        ("Autostart", "d / Delete", "Delete selected autostart configuration entry"),
        ("Applications", "↑/↓ or j/k", "Navigate installed applications and packages"),
        ("Applications", "f / s / d", "Filter sources (f) / Sort by size/name (s) / Invert sort (d)"),
        ("Applications", "u / Del", "Uninstall selected application (prompts sudo if needed)"),
        ("Applications", "/", "Live search packages by name or description"),
    ];

    let header_cells = ["Category", "Key", "Action / Function"]
        .iter()
        .map(|&h| Cell::from(Span::styled(h, theme.header_style())));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = shortcuts
        .iter()
        .map(|(cat, key, desc)| {
            let cells = vec![
                Cell::from(*cat).style(Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
                Cell::from(*key).style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Cell::from(*desc).style(Style::default().fg(theme.fg)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(20),
        Constraint::Percentage(25),
        Constraint::Percentage(55),
    ];

    let table = Table::new(rows, widths).header(header);
    f.render_widget(table, inner);
}

fn render_new_autostart_modal(
    f: &mut Frame,
    app: &App,
    name: &str,
    exec: &str,
    comment: &str,
    active_field: usize,
) {
    let theme = &app.theme;
    let area = centered_rect(60, 45, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󱑞 Add New Startup Application ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Field 0: Name
            Constraint::Length(3), // Field 1: Exec
            Constraint::Length(3), // Field 2: Comment
            Constraint::Length(3), // Controls
        ])
        .split(inner);

    // Render Field 0: Name
    let b0 = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active_field == 0 { theme.accent } else { theme.border }))
        .title(Span::styled(" Application Name (e.g. Discord) ", if active_field == 0 { theme.title_style() } else { theme.dim_style() }));
    let t0 = if active_field == 0 {
        Line::from(vec![
            Span::styled(name.to_string(), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(theme.accent)),
        ])
    } else {
        Line::from(Span::styled(if name.is_empty() { "(empty)" } else { name }, theme.dim_style()))
    };
    f.render_widget(Paragraph::new(t0).block(b0), chunks[0]);

    // Render Field 1: Exec
    let b1 = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active_field == 1 { theme.accent } else { theme.border }))
        .title(Span::styled(" Executable Command (e.g. /usr/bin/discord) ", if active_field == 1 { theme.title_style() } else { theme.dim_style() }));
    let t1 = if active_field == 1 {
        Line::from(vec![
            Span::styled(exec.to_string(), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(theme.accent)),
        ])
    } else {
        Line::from(Span::styled(if exec.is_empty() { "(empty)" } else { exec }, theme.dim_style()))
    };
    f.render_widget(Paragraph::new(t1).block(b1), chunks[1]);

    // Render Field 2: Comment
    let b2 = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active_field == 2 { theme.accent } else { theme.border }))
        .title(Span::styled(" Comment / Description (Optional) ", if active_field == 2 { theme.title_style() } else { theme.dim_style() }));
    let t2 = if active_field == 2 {
        Line::from(vec![
            Span::styled(comment.to_string(), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(theme.accent)),
        ])
    } else {
        Line::from(Span::styled(if comment.is_empty() { "(empty)" } else { comment }, theme.dim_style()))
    };
    f.render_widget(Paragraph::new(t2).block(b2), chunks[2]);

    let controls = Paragraph::new(Line::from(vec![
        Span::styled(" [Tab] Next Field ", Style::default().fg(theme.accent)),
        Span::raw("   "),
        Span::styled(" [Enter] Create Entry ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
        Span::styled(" [Esc] Cancel ", theme.dim_style()),
    ])).alignment(Alignment::Center);
    f.render_widget(controls, chunks[3]);
}
