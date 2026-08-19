use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Sparkline, Wrap},
    Frame,
};

use crate::app::App;
use crate::system::collector::{format_bytes, format_speed};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // System Specs Banner
            Constraint::Length(10), // CPU & Memory / Swap Gauges + Sparklines
            Constraint::Min(10),   // Disks, Network & Sensors/Cores
        ])
        .split(area);

    render_specs_banner(f, app, chunks[0]);
    render_cpu_memory_row(f, app, chunks[1]);
    render_bottom_resource_row(f, app, chunks[2]);
}

fn render_specs_banner(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󰍹 System Specification ", theme.title_style()));

    let col = &app.collector;
    let text = vec![
        Line::from(vec![
            Span::styled(" Host: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(&col.host_name, Style::default().fg(theme.fg)),
            Span::raw("   "),
            Span::styled(" OS: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(&col.os_name, Style::default().fg(theme.fg)),
            Span::raw("   "),
            Span::styled(" Kernel: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(&col.kernel_version, Style::default().fg(theme.fg)),
            Span::raw("   "),
            Span::styled(" Uptime: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(col.uptime_formatted(), Style::default().fg(theme.success)),
        ]),
        Line::from(vec![
            Span::styled(" CPU: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ({} Cores)", col.cpu_model, col.cpu_count), Style::default().fg(theme.fg)),
            Span::raw("   "),
            Span::styled(" Load Avg: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:.2}  {:.2}  {:.2}", col.load_avg_one, col.load_avg_five, col.load_avg_fifteen),
                Style::default().fg(theme.warning),
            ),
        ]),
    ];

    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn render_cpu_memory_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_cpu_card(f, app, chunks[0]);
    render_memory_card(f, app, chunks[1]);
}

fn render_cpu_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  CPU Utilization & History ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(inner);

    let cpu_pct = app.collector.sys.global_cpu_usage().round().clamp(0.0, 100.0) as u16;
    let cpu_color = if cpu_pct > 80 {
        theme.danger
    } else if cpu_pct > 50 {
        theme.warning
    } else {
        theme.success
    };

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(cpu_color).bg(theme.bg))
        .percent(cpu_pct)
        .label(format!("Global CPU Load: {}%", cpu_pct));
    f.render_widget(gauge, rows[0]);

    // Sparkline history
    let cpu_data: Vec<u64> = app.collector.cpu_history.iter().copied().collect();
    let sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled("CPU History (60s)", theme.dim_style())))
        .data(&cpu_data)
        .max(100)
        .style(Style::default().fg(theme.accent));
    f.render_widget(sparkline, rows[1]);
}

fn render_memory_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󰍛 Memory & Swap Utilization ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Min(2)])
        .split(inner);

    let total_mem = app.collector.sys.total_memory();
    let used_mem = app.collector.sys.used_memory();
    let mem_pct = if total_mem > 0 {
        ((used_mem as f64 / total_mem as f64) * 100.0).round().clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let mem_color = if mem_pct > 85 {
        theme.danger
    } else if mem_pct > 65 {
        theme.warning
    } else {
        theme.accent
    };

    let mem_gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(mem_color).bg(theme.bg))
        .percent(mem_pct)
        .label(format!(
            "RAM: {} / {} ({}%)",
            format_bytes(used_mem),
            format_bytes(total_mem),
            mem_pct
        ));
    f.render_widget(mem_gauge, rows[0]);

    // Swap Gauge
    let total_swap = app.collector.sys.total_swap();
    let used_swap = app.collector.sys.used_swap();
    let swap_pct = if total_swap > 0 {
        ((used_swap as f64 / total_swap as f64) * 100.0).round().clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let swap_gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(theme.secondary).bg(theme.bg))
        .percent(swap_pct)
        .label(format!(
            "Swap: {} / {} ({}%)",
            format_bytes(used_swap),
            format_bytes(total_swap),
            swap_pct
        ));
    f.render_widget(swap_gauge, rows[1]);

    // Memory sparkline
    let mem_data: Vec<u64> = app.collector.mem_history.iter().copied().collect();
    let sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled("RAM History (60s)", theme.dim_style())))
        .data(&mem_data)
        .max(100)
        .style(Style::default().fg(theme.success));
    f.render_widget(sparkline, rows[2]);
}

fn render_bottom_resource_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34), // Disks & Storage
            Constraint::Percentage(33), // Network Traffic
            Constraint::Percentage(33), // Cores & Sensors
        ])
        .split(area);

    render_disks_card(f, app, chunks[0]);
    render_network_card(f, app, chunks[1]);
    render_cores_sensors_card(f, app, chunks[2]);
}

fn render_disks_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  Storage Disks ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let disks = &app.collector.disks;
    if disks.is_empty() {
        let p = Paragraph::new("No mounted disks detected").style(theme.dim_style());
        f.render_widget(p, inner);
        return;
    }

    let constraints = vec![Constraint::Length(3); disks.len().min(4)];
    let disk_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, disk) in disks.iter().take(4).enumerate() {
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total.saturating_sub(avail);
        let pct = if total > 0 {
            ((used as f64 / total as f64) * 100.0).round().clamp(0.0, 100.0) as u16
        } else {
            0
        };

        let disk_color = if pct > 90 {
            theme.danger
        } else if pct > 75 {
            theme.warning
        } else {
            theme.accent
        };

        let mount = disk.mount_point().to_string_lossy();
        let fs_type = disk.file_system().to_string_lossy();

        let gauge = Gauge::default()
            .block(Block::default().title(Span::styled(
                format!("{} ({})", mount, fs_type),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )))
            .gauge_style(Style::default().fg(disk_color).bg(theme.bg))
            .percent(pct)
            .label(format!(
                "{} / {} ({}%)",
                format_bytes(used),
                format_bytes(total),
                pct
            ));

        f.render_widget(gauge, disk_chunks[i]);
    }
}

fn render_network_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󰲝 Network Traffic ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Speed Badges
            Constraint::Length(3), // Download Sparkline
            Constraint::Length(3), // Upload Sparkline
        ])
        .split(inner);

    let col = &app.collector;
    let rx_speed_str = format_speed(col.current_rx_speed);
    let tx_speed_str = format_speed(col.current_tx_speed);

    let speeds = Paragraph::new(Line::from(vec![
        Span::styled("  Down: ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::styled(&rx_speed_str, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
        Span::styled("  Up: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::styled(&tx_speed_str, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(speeds, rows[0]);

    // RX Sparkline
    let rx_data: Vec<u64> = col.net_rx_history.iter().copied().collect();
    let max_rx = *rx_data.iter().max().unwrap_or(&1024).max(&1024);
    let rx_sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled("Download History", theme.dim_style())))
        .data(&rx_data)
        .max(max_rx)
        .style(Style::default().fg(theme.success));
    f.render_widget(rx_sparkline, rows[1]);

    // TX Sparkline
    let tx_data: Vec<u64> = col.net_tx_history.iter().copied().collect();
    let max_tx = *tx_data.iter().max().unwrap_or(&1024).max(&1024);
    let tx_sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled("Upload History", theme.dim_style())))
        .data(&tx_data)
        .max(max_tx)
        .style(Style::default().fg(theme.secondary));
    f.render_widget(tx_sparkline, rows[2]);
}

fn render_cores_sensors_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  CPU Cores & Thermal Sensors ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cores = &app.collector.core_usages;
    let sensors = &app.sensor_collector.sensors;

    let mut lines = Vec::new();

    // CPU Cores header
    lines.push(Line::from(vec![
        Span::styled("Active CPU Cores: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} Cores Active", cores.len()), theme.dim_style()),
    ]));

    // Sample first 4 cores
    for core in cores.iter().take(4) {
        let pct = core.usage.round().clamp(0.0, 100.0) as u16;
        let color = if pct > 80 {
            theme.danger
        } else if pct > 50 {
            theme.warning
        } else {
            theme.success
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ({}MHz): ", core.name, core.frequency_mhz), theme.dim_style()),
            Span::styled(format!("{}%", pct), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
    }

    lines.push(Line::from(Span::raw("")));

    // Thermal Sensors
    lines.push(Line::from(vec![
        Span::styled("󰔏 Temperatures: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
    ]));

    if sensors.is_empty() {
        lines.push(Line::from(Span::styled("  No thermal sensors detected", theme.dim_style())));
    } else {
        for s in sensors.iter().take(3) {
            let temp_color = if s.temperature > 80.0 {
                theme.danger
            } else if s.temperature > 60.0 {
                theme.warning
            } else {
                theme.success
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", s.label), theme.dim_style()),
                Span::styled(format!("{:.1}°C", s.temperature), Style::default().fg(temp_color).add_modifier(Modifier::BOLD)),
            ]));
        }
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}
