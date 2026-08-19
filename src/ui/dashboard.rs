use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Sparkline, Wrap},
    Frame,
};

use crate::app::App;
use crate::system::collector::{format_bytes, format_speed};
use crate::system::processes::ProcessItem;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if area.height >= 40 {
        // Expanded View: Specs Banner, CPU+GPU, All Cores+Storage, Network+Open Ports, Top Eaters
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // 1. Specs & Health Banner
                Constraint::Length(9),  // 2. CPU & GPU / Memory Telemetry
                Constraint::Length(8),  // 3. ALL CPU Cores Grid & Storage
                Constraint::Length(10), // 4. Network Ingress/Egress & Open Ports (Interactive)
                Constraint::Min(8),     // 5. System Eaters & Diagnostics
            ])
            .split(area);

        render_specs_banner(f, app, chunks[0]);
        render_cpu_gpu_row(f, app, chunks[1]);
        render_all_cores_storage_row(f, app, chunks[2]);
        render_network_sockets_row(f, app, chunks[3]);
        render_system_analysis_row(f, app, chunks[4]);
    } else if area.height >= 28 {
        // Medium View
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // 1. Specs Banner
                Constraint::Length(8),  // 2. CPU & GPU / Memory
                Constraint::Length(9),  // 3. Network & Open Ports
                Constraint::Min(7),     // 4. All Cores & System Eaters
            ])
            .split(area);

        render_specs_banner(f, app, chunks[0]);
        render_cpu_gpu_row(f, app, chunks[1]);
        render_network_sockets_row(f, app, chunks[2]);
        render_compact_bottom_row(f, app, chunks[3]);
    } else {
        // Compact View
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Min(8),
            ])
            .split(area);

        render_specs_banner(f, app, chunks[0]);
        render_cpu_gpu_row(f, app, chunks[1]);
        render_all_cores_storage_row(f, app, chunks[2]);
    }
}

fn render_specs_banner(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󰍹 System Specification & Health ", theme.title_style()));

    let col = &app.collector;
    let cpu_load = col.sys.global_cpu_usage();
    let health_badge = if cpu_load > 85.0 {
        Span::styled(" ● HIGH CPU LOAD ", Style::default().fg(theme.bg).bg(theme.danger).add_modifier(Modifier::BOLD))
    } else if col.load_avg_one > col.cpu_count as f64 {
        Span::styled(" ▲ ELEVATED LOAD ", Style::default().fg(theme.bg).bg(theme.warning).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" ✔ SYSTEM OPTIMAL ", Style::default().fg(theme.bg).bg(theme.success).add_modifier(Modifier::BOLD))
    };

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
            Span::raw("   "),
            health_badge,
        ]),
        Line::from(vec![
            Span::styled(" CPU: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ({} Cores)", col.cpu_model, col.cpu_count), Style::default().fg(theme.fg)),
            Span::raw("   "),
            Span::styled(" Load Average (1m, 5m, 15m): ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:.2}  {:.2}  {:.2}", col.load_avg_one, col.load_avg_five, col.load_avg_fifteen),
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn render_cpu_gpu_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_cpu_card(f, app, chunks[0]);
    render_gpu_memory_card(f, app, chunks[1]);
}

fn render_cpu_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  CPU Utilization & 60s Load History ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
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
        .block(Block::default().title(Span::styled("60-Second CPU Load History", theme.dim_style())))
        .data(&cpu_data)
        .max(100)
        .style(Style::default().fg(theme.accent));
    f.render_widget(sparkline, rows[1]);
}

fn render_gpu_memory_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    if let Some(gpu) = &app.gpu_collector.gpu {
        // Dedicated GPU Card
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.card_bg))
            .title(Span::styled(format!(" 󰢮 GPU: {} ", gpu.name), theme.title_style()));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // GPU Util Gauge
                Constraint::Length(2), // VRAM Gauge
                Constraint::Min(2),    // Stats & Apps
            ])
            .split(inner);

        let gpu_color = if gpu.utilization > 80 {
            theme.danger
        } else if gpu.utilization > 50 {
            theme.warning
        } else {
            theme.success
        };

        let gpu_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(gpu_color).bg(theme.bg))
            .percent(gpu.utilization)
            .label(format!("GPU Load: {}% (Driver: {})", gpu.utilization, gpu.driver));
        f.render_widget(gpu_gauge, rows[0]);

        // VRAM Gauge
        let vram_pct = if gpu.vram_total_mb > 0 {
            ((gpu.vram_used_mb as f64 / gpu.vram_total_mb as f64) * 100.0).round().clamp(0.0, 100.0) as u16
        } else {
            0
        };

        let vram_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(theme.secondary).bg(theme.bg))
            .percent(vram_pct)
            .label(format!(
                "VRAM: {} MB / {} MB ({}%)",
                gpu.vram_used_mb, gpu.vram_total_mb, vram_pct
            ));
        f.render_widget(vram_gauge, rows[1]);

        // GPU Stats & Processes
        let mut stats_spans = vec![
            Span::styled(" Temp: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}°C ", gpu.temperature), if gpu.temperature > 75 { Style::default().fg(theme.danger) } else { Style::default().fg(theme.success) }),
            Span::raw(" | "),
            Span::styled(" Power: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.1}W ", gpu.power_w), Style::default().fg(theme.warning)),
            Span::raw(" | "),
            Span::styled(" Apps: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        ];

        if gpu.processes.is_empty() {
            stats_spans.push(Span::styled("Xorg / Shell active", theme.dim_style()));
        } else {
            for p in gpu.processes.iter().take(2) {
                stats_spans.push(Span::styled(format!("{} ({}MB) ", p.name, p.memory_mb), Style::default().fg(theme.fg)));
            }
        }

        let p_stats = Paragraph::new(Line::from(stats_spans));
        f.render_widget(p_stats, rows[2]);
    } else {
        // Fallback to RAM & Swap Card
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
            .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Min(1)])
            .split(inner);

        let total_mem = app.collector.sys.total_memory();
        let used_mem = app.collector.sys.used_memory();
        let mem_pct = if total_mem > 0 {
            ((used_mem as f64 / total_mem as f64) * 100.0).round().clamp(0.0, 100.0) as u16
        } else {
            0
        };

        let mem_color = if mem_pct > 85 { theme.danger } else if mem_pct > 65 { theme.warning } else { theme.accent };

        let mem_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(mem_color).bg(theme.bg))
            .percent(mem_pct)
            .label(format!("RAM: {} / {} ({}%)", format_bytes(used_mem), format_bytes(total_mem), mem_pct));
        f.render_widget(mem_gauge, rows[0]);

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
            .label(format!("Swap: {} / {} ({}%)", format_bytes(used_swap), format_bytes(total_swap), swap_pct));
        f.render_widget(swap_gauge, rows[1]);

        let mem_data: Vec<u64> = app.collector.mem_history.iter().copied().collect();
        let sparkline = Sparkline::default()
            .block(Block::default().title(Span::styled("60-Second RAM Load History", theme.dim_style())))
            .data(&mem_data)
            .max(100)
            .style(Style::default().fg(theme.success));
        f.render_widget(sparkline, rows[2]);
    }
}

fn render_all_cores_storage_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_all_cores_grid(f, app, chunks[0]);
    render_disks_sensors_card(f, app, chunks[1]);
}

fn render_all_cores_grid(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let cores = &app.collector.core_usages;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(format!("  All CPU Cores Heatmap ({} Cores Active) ", cores.len()), theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if cores.is_empty() {
        return;
    }

    // Determine grid columns: 4 columns for 16 cores, or 2/3 columns depending on width
    let cols_count = if area.width >= 70 { 4 } else if area.width >= 50 { 3 } else { 2 };
    let rows_count = (cores.len() + cols_count - 1) / cols_count;

    let mut lines = Vec::new();
    for r in 0..rows_count {
        let mut spans = Vec::new();
        for c in 0..cols_count {
            let idx = r * cols_count + c;
            if let Some(core) = cores.get(idx) {
                let pct = core.usage.round().clamp(0.0, 100.0) as u16;
                let color = if pct > 80 {
                    theme.danger
                } else if pct > 50 {
                    theme.warning
                } else {
                    theme.success
                };

                let mini_bar = match pct {
                    0..=15 => " ",
                    16..=35 => "▂",
                    36..=55 => "▄",
                    56..=75 => "▆",
                    _ => "█",
                };

                spans.push(Span::styled(format!("C{:<2} ", idx), theme.dim_style()));
                spans.push(Span::styled(format!("{} ", mini_bar), Style::default().fg(color).add_modifier(Modifier::BOLD)));
                spans.push(Span::styled(format!("{:>3}% ", pct), Style::default().fg(color)));
                spans.push(Span::styled(format!("{:>4}M ", core.frequency_mhz), theme.dim_style()));
                if c + 1 < cols_count {
                    spans.push(Span::raw("│ "));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn render_disks_sensors_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("  Storage & Thermals ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let disks = &app.collector.disks;
    let sensors = &app.sensor_collector.sensors;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(inner);

    // Primary disk gauge
    if let Some(disk) = disks.first() {
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total.saturating_sub(avail);
        let pct = if total > 0 { ((used as f64 / total as f64) * 100.0).round().clamp(0.0, 100.0) as u16 } else { 0 };

        let disk_gauge = Gauge::default()
            .block(Block::default().title(Span::styled(
                format!("Disk: {} ({})", disk.mount_point().to_string_lossy(), disk.file_system().to_string_lossy()),
                Style::default().fg(theme.fg),
            )))
            .gauge_style(Style::default().fg(if pct > 85 { theme.danger } else { theme.accent }).bg(theme.bg))
            .percent(pct)
            .label(format!("{} / {} ({}%)", format_bytes(used), format_bytes(total), pct));
        f.render_widget(disk_gauge, rows[0]);
    }

    // Thermal sensors
    let mut sensor_spans = vec![
        Span::styled(" 󰔏 Thermals: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
    ];
    if sensors.is_empty() {
        sensor_spans.push(Span::styled("Normal ACPI", theme.dim_style()));
    } else {
        for s in sensors.iter().take(3) {
            let temp_color = if s.temperature > 80.0 { theme.danger } else if s.temperature > 60.0 { theme.warning } else { theme.success };
            sensor_spans.push(Span::styled(format!("{}: ", s.label), theme.dim_style()));
            sensor_spans.push(Span::styled(format!("{:.1}°C  ", s.temperature), Style::default().fg(temp_color).add_modifier(Modifier::BOLD)));
        }
    }
    let p_sens = Paragraph::new(Line::from(sensor_spans));
    f.render_widget(p_sens, rows[1]);
}

fn render_network_sockets_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);

    render_network_bandwidth_card(f, app, chunks[0]);
    render_open_ports_sockets_card(f, app, chunks[1]);
}

fn render_network_bandwidth_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󰲝 Network Traffic (Ingress & Egress) ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Speeds
            Constraint::Length(3), // Ingress
            Constraint::Length(3), // Egress
            Constraint::Min(1),    // Top Network Apps
        ])
        .split(inner);

    let col = &app.collector;
    let rx_speed_str = format_speed(col.current_rx_speed);
    let tx_speed_str = format_speed(col.current_tx_speed);

    let speeds = Paragraph::new(Line::from(vec![
        Span::styled("  Ingress (Down): ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::styled(&rx_speed_str, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
        Span::styled("  Egress (Up): ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::styled(&tx_speed_str, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(speeds, rows[0]);

    // Ingress Sparkline
    let rx_data: Vec<u64> = col.net_rx_history.iter().copied().collect();
    let max_rx = *rx_data.iter().max().unwrap_or(&1024).max(&1024);
    let rx_sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled("Live Ingress (Download) History", theme.dim_style())))
        .data(&rx_data)
        .max(max_rx)
        .style(Style::default().fg(theme.success));
    f.render_widget(rx_sparkline, rows[1]);

    // Egress Sparkline
    let tx_data: Vec<u64> = col.net_tx_history.iter().copied().collect();
    let max_tx = *tx_data.iter().max().unwrap_or(&1024).max(&1024);
    let tx_sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled("Live Egress (Upload) History", theme.dim_style())))
        .data(&tx_data)
        .max(max_tx)
        .style(Style::default().fg(theme.secondary));
    f.render_widget(tx_sparkline, rows[2]);

    // Top Network Consuming Apps
    let top_net_procs = &app.network_mgr.summary.top_network_processes;
    let mut net_procs_spans = vec![
        Span::styled(" Top Network Apps: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ];
    if top_net_procs.is_empty() {
        net_procs_spans.push(Span::styled("None", theme.dim_style()));
    } else {
        for (name, count) in top_net_procs.iter().take(3) {
            net_procs_spans.push(Span::styled(format!("{} ({} sock) ", name, count), Style::default().fg(theme.fg)));
        }
    }
    let p_net_procs = Paragraph::new(Line::from(net_procs_spans));
    f.render_widget(p_net_procs, rows[3]);
}

fn render_open_ports_sockets_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let summary = &app.network_mgr.summary;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            format!(" 󰒺 Open Ports & Sockets ({} Total) — [k] Kill Port ", summary.total_sockets),
            theme.title_style(),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    // Listening Ports Header
    lines.push(Line::from(vec![
        Span::styled("󰄲 Open Listening Ports (Select with ↑/↓, Kill with [k]):", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]));

    if summary.listening_ports.is_empty() {
        lines.push(Line::from(Span::styled("  No open listening ports detected", theme.dim_style())));
    } else {
        for (i, s) in summary.listening_ports.iter().take(4).enumerate() {
            let is_selected = i == app.selected_port_index;
            let pointer = if is_selected { "► " } else { "  " };
            let pid_str = s.pid.map(|p| format!(" (PID {})", p)).unwrap_or_default();

            let row_style = if is_selected {
                Style::default().fg(theme.accent).bg(theme.selected_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            lines.push(Line::from(vec![
                Span::styled(pointer, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                Span::styled(format!("Port {:<5} ", s.local_port), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[{}] ", s.proto), Style::default().fg(theme.secondary)),
                Span::styled(format!("➔ {}{}", s.proc_name, pid_str), row_style),
                Span::styled(if is_selected { "  [PRESS 'k' TO KILL]" } else { "" }, Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            ]));
        }
    }

    lines.push(Line::from(Span::raw("")));

    // Active Remote Connections
    lines.push(Line::from(vec![
        Span::styled("󰖟 Active Established Remote Connections:", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
    ]));

    if summary.active_connections.is_empty() {
        lines.push(Line::from(Span::styled("  No external active connections", theme.dim_style())));
    } else {
        for s in summary.active_connections.iter().take(2) {
            let pid_str = s.pid.map(|p| format!(" (PID {})", p)).unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(format!("  • {:<16} ", s.proc_name), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled(" ⇄ ", Style::default().fg(theme.accent)),
                Span::styled(format!("{}:{} ", s.peer_addr, s.peer_port), Style::default().fg(theme.secondary)),
                Span::styled(pid_str, theme.dim_style()),
            ]));
        }
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn render_system_analysis_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_top_consumers_card(f, app, chunks[0]);
    render_health_diagnostics_card(f, app, chunks[1]);
}

fn render_compact_bottom_row(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_all_cores_grid(f, app, chunks[0]);
    render_top_consumers_card(f, app, chunks[1]);
}

fn render_top_consumers_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 🔥 What is Eating Up the System (Top Consumers) ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Calculate Top CPU processes
    let mut top_cpu: Vec<&ProcessItem> = app.process_list.iter().collect();
    top_cpu.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    let top_3_cpu: Vec<&ProcessItem> = top_cpu.into_iter().take(3).collect();

    // Calculate Top Memory processes
    let mut top_mem: Vec<&ProcessItem> = app.process_list.iter().collect();
    top_mem.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    let top_3_mem: Vec<&ProcessItem> = top_mem.into_iter().take(3).collect();

    let mut lines = Vec::new();

    // Top CPU Hogs
    lines.push(Line::from(vec![
        Span::styled(" Top CPU Consuming Processes: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]));

    if top_3_cpu.is_empty() {
        lines.push(Line::from(Span::styled("  No active processes found", theme.dim_style())));
    } else {
        for p in top_3_cpu {
            let cpu_color = if p.cpu_usage > 50.0 {
                theme.danger
            } else if p.cpu_usage > 20.0 {
                theme.warning
            } else {
                theme.success
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  PID {:<5} ", p.pid), theme.dim_style()),
                Span::styled(format!("{:<18}", p.name), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>5.1}% CPU", p.cpu_usage), Style::default().fg(cpu_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  ({:.1}% RAM)", p.memory_pct), theme.dim_style()),
            ]));
        }
    }

    lines.push(Line::from(Span::raw("")));

    // Top Memory Hogs
    lines.push(Line::from(vec![
        Span::styled("󰍛 Top Memory Consuming Processes: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
    ]));

    if top_3_mem.is_empty() {
        lines.push(Line::from(Span::styled("  No memory consumers found", theme.dim_style())));
    } else {
        for p in top_3_mem {
            lines.push(Line::from(vec![
                Span::styled(format!("  PID {:<5} ", p.pid), theme.dim_style()),
                Span::styled(format!("{:<18}", p.name), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>9}", format_bytes(p.memory_bytes)), Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  ({:.1}%)", p.memory_pct), theme.dim_style()),
            ]));
        }
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn render_health_diagnostics_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 🛡️ System Health & Optimization Analysis ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let total_procs = app.process_list.len();
    let running_procs = app.process_list.iter().filter(|p| p.status.starts_with('R')).count();
    let sleeping_procs = app.process_list.iter().filter(|p| p.status.starts_with('S') || p.status.starts_with('I')).count();
    let zombie_procs = app.process_list.iter().filter(|p| p.status.starts_with('Z')).count();

    let cleanable_bytes = app.cleaner.total_scanned_bytes;
    let cleanable_str = if cleanable_bytes > 0 {
        format_bytes(cleanable_bytes)
    } else {
        "Scanned Clean".to_string()
    };

    let total_services = app.service_mgr.services.len();
    let active_services = app.service_mgr.services.iter().filter(|s| s.active_state == "active").count();
    let failed_services = app.service_mgr.services.iter().filter(|s| s.sub_state == "failed").count();

    let failed_style = if failed_services > 0 {
        Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.success)
    };

    let mut lines = Vec::new();

    // Process State Diagnostics
    lines.push(Line::from(vec![
        Span::styled("⚙ Process State: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} Total", total_procs), Style::default().fg(theme.fg)),
        Span::raw(" | "),
        Span::styled(format!("{} Running", running_procs), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(format!("{} Sleeping", sleeping_procs), theme.dim_style()),
        Span::raw(" | "),
        Span::styled(format!("{} Zombie", zombie_procs), if zombie_procs > 0 { Style::default().fg(theme.danger) } else { theme.dim_style() }),
    ]));

    // systemd Service Health
    lines.push(Line::from(vec![
        Span::styled(" Services Health: ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} Active / {} Total", active_services, total_services), Style::default().fg(theme.fg)),
        Span::raw(" | "),
        Span::styled(format!("{} Failed", failed_services), failed_style),
    ]));

    // System Cache & Cleaner Status
    lines.push(Line::from(vec![
        Span::styled("󰃢 Storage & Cache: ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
        Span::styled(cleanable_str, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
        Span::styled(" cleanable in Tab [3]", theme.dim_style()),
    ]));

    lines.push(Line::from(Span::raw("")));

    // Real-Time System Advisory
    let advisory_text = if zombie_procs > 0 {
        Line::from(vec![
            Span::styled("💡 Health Advisory: ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Found {} zombie processes. Check Tab [2] to inspect parent processes.", zombie_procs), Style::default().fg(theme.danger)),
        ])
    } else if failed_services > 0 {
        Line::from(vec![
            Span::styled("💡 Health Advisory: ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} systemd service(s) in failed state. Review logs in Tab [4].", failed_services), Style::default().fg(theme.danger)),
        ])
    } else if cleanable_bytes > 500 * 1024 * 1024 {
        Line::from(vec![
            Span::styled("💡 Health Advisory: ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Reclaimable disk cache available ({}). Press '3' to run cleaner.", format_bytes(cleanable_bytes)), Style::default().fg(theme.warning)),
        ])
    } else {
        Line::from(vec![
            Span::styled("💡 Health Advisory: ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("System operating in perfect equilibrium (Stasis achieved).", Style::default().fg(theme.success)),
        ])
    };

    lines.push(advisory_text);

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}
