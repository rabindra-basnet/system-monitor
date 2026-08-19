use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Sparkline, Table, Wrap},
    Frame,
};

use crate::app::App;
use crate::system::collector::{format_bytes, format_speed};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if area.height >= 38 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 1. Filter & Mode Header
                Constraint::Length(9), // 2. Dual Ingress / Egress Telemetry
                Constraint::Length(7), // 3. Network Applications Traffic & Socket Breakdown
                Constraint::Min(10),   // 4. Interactive Sockets & Open Ports Table
                Constraint::Length(4), // 5. Socket Inspector & Quick Actions Bar
            ])
            .split(area);

        render_header(f, app, chunks[0]);
        render_bandwidth_cards(f, app, chunks[1]);
        render_app_traffic_card(f, app, chunks[2]);
        render_sockets_table(f, app, chunks[3]);
        render_socket_detail(f, app, chunks[4]);
    } else if area.height >= 28 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 1. Filter & Mode Header
                Constraint::Length(8), // 2. Dual Bandwidth Cards
                Constraint::Min(8),    // 3. Sockets Table
                Constraint::Length(4), // 4. Socket Inspector Bar
            ])
            .split(area);

        render_header(f, app, chunks[0]);
        render_bandwidth_cards(f, app, chunks[1]);
        render_sockets_table(f, app, chunks[2]);
        render_socket_detail(f, app, chunks[3]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);

        render_header(f, app, chunks[0]);
        render_sockets_table(f, app, chunks[1]);
        render_socket_detail(f, app, chunks[2]);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg));

    let filter_text = if app.network_mgr.filter.is_empty() {
        Span::styled("None (Press '/' to filter)", theme.dim_style())
    } else {
        Span::styled(
            &app.network_mgr.filter,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )
    };

    let total = app.network_mgr.sockets.len();
    let filtered = app.network_mgr.filtered_sockets().len();

    let line = Line::from(vec![
        Span::styled(
            " 󰲝 Mode [f]: ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", app.network_mgr.filter_mode.label()),
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   |   "),
        Span::styled(
            "  Filter [/]: ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        filter_text,
        Span::raw("   |   "),
        Span::styled(
            " 󰒺 Kill Port [k] ",
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   |   "),
        Span::styled(
            " 📋 Copy [y] ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   |   "),
        Span::styled(
            format!("{} / {} sockets", filtered, total),
            theme.dim_style(),
        ),
    ]);

    let p = Paragraph::new(line).block(block);
    f.render_widget(p, area);
}

fn render_bandwidth_cards(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Ingress Card
            Constraint::Percentage(50), // Egress Card
        ])
        .split(area);

    let col = &app.collector;
    let rx_speed_str = format_speed(col.current_rx_speed);
    let tx_speed_str = format_speed(col.current_tx_speed);

    // 1. Ingress (Download) Card
    let ingress_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            "  Ingress (Download) Live Telemetry & 60s History ",
            theme.title_style(),
        ));

    let in_inner = ingress_block.inner(chunks[0]);
    f.render_widget(ingress_block, chunks[0]);

    let in_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(2)])
        .split(in_inner);

    let p_in = Paragraph::new(Line::from(vec![
        Span::styled(
            " Download Rate: ",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", rx_speed_str),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("(Total Data In: {})", format_bytes(col.last_net_rx)),
            theme.dim_style(),
        ),
    ]));
    f.render_widget(p_in, in_rows[0]);

    let rx_data: Vec<u64> = col.net_rx_history.iter().copied().collect();
    let max_rx = *rx_data.iter().max().unwrap_or(&1024).max(&1024);
    let rx_sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled(
            "60s Realtime Ingress Sparkline",
            theme.dim_style(),
        )))
        .data(&rx_data)
        .max(max_rx)
        .style(Style::default().fg(theme.success));
    f.render_widget(rx_sparkline, in_rows[1]);

    // 2. Egress (Upload) Card
    let egress_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            "  Egress (Upload) Live Telemetry & 60s History ",
            theme.title_style(),
        ));

    let out_inner = egress_block.inner(chunks[1]);
    f.render_widget(egress_block, chunks[1]);

    let out_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(2)])
        .split(out_inner);

    let p_out = Paragraph::new(Line::from(vec![
        Span::styled(
            " Upload Rate: ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", tx_speed_str),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("(Total Data Out: {})", format_bytes(col.last_net_tx)),
            theme.dim_style(),
        ),
    ]));
    f.render_widget(p_out, out_rows[0]);

    let tx_data: Vec<u64> = col.net_tx_history.iter().copied().collect();
    let max_tx = *tx_data.iter().max().unwrap_or(&1024).max(&1024);
    let tx_sparkline = Sparkline::default()
        .block(Block::default().title(Span::styled(
            "60s Realtime Egress Sparkline",
            theme.dim_style(),
        )))
        .data(&tx_data)
        .max(max_tx)
        .style(Style::default().fg(theme.secondary));
    f.render_widget(tx_sparkline, out_rows[1]);
}

fn render_app_traffic_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            " 📊 Applications Using Network & Bandwidth Breakdown ",
            theme.title_style(),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let app_usages = &app.network_mgr.summary.app_network_usage;
    let mut lines = Vec::new();

    if app_usages.is_empty() {
        lines.push(Line::from(Span::styled(
            "No active network applications detected",
            theme.dim_style(),
        )));
    } else {
        let total_sockets = app.network_mgr.summary.total_sockets.max(1);

        for app_u in app_usages.iter().take(4) {
            let pct = ((app_u.socket_count as f64 / total_sockets as f64) * 100.0).round() as usize;
            let pid_str = app_u
                .pid
                .map(|p| format!(" (PID {})", p))
                .unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" • {:<16}", app_u.name),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(pid_str, theme.dim_style()),
                Span::raw(" ➔ "),
                Span::styled(
                    format!("{:>2} sockets ({}%)", app_u.socket_count, pct),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" | "),
                Span::styled(
                    format!("{} TCP / {} UDP", app_u.tcp_count, app_u.udp_count),
                    Style::default().fg(theme.secondary),
                ),
                Span::raw(" | "),
                Span::styled(
                    format!(
                        "{} Estab / {} Listen",
                        app_u.established_count, app_u.listening_count
                    ),
                    if app_u.established_count > 0 {
                        Style::default().fg(theme.success)
                    } else {
                        theme.dim_style()
                    },
                ),
                Span::raw(" | "),
                Span::styled(
                    format!(
                        "Recv-Q: {} / Send-Q: {}",
                        format_bytes(app_u.recv_q_bytes),
                        format_bytes(app_u.send_q_bytes)
                    ),
                    theme.dim_style(),
                ),
            ]));
        }
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}

fn render_sockets_table(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let w = area.width;

    let (header_cells, widths): (Vec<&str>, Vec<Constraint>) = if w < 110 {
        (
            vec!["Port", "Proto", "State", "Process", "PID", "Scope"],
            vec![
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Percentage(40),
                Constraint::Length(8),
                Constraint::Min(12),
            ],
        )
    } else if w < 135 {
        (
            vec![
                "Port",
                "Proto",
                "State",
                "Local Address",
                "Process",
                "PID",
                "Scope",
            ],
            vec![
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Percentage(25),
                Constraint::Percentage(30),
                Constraint::Length(8),
                Constraint::Min(12),
            ],
        )
    } else {
        (
            vec![
                "Port",
                "Proto",
                "State",
                "Local Address",
                "Remote Endpoint",
                "Process Name",
                "PID",
                "Scope",
            ],
            vec![
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Percentage(22),
                Constraint::Percentage(22),
                Constraint::Percentage(24),
                Constraint::Length(8),
                Constraint::Min(12),
            ],
        )
    };

    let header = Row::new(
        header_cells
            .iter()
            .map(|&h| Cell::from(Span::styled(h, theme.header_style()))),
    )
    .height(1)
    .bottom_margin(1);

    let filtered_sockets = app.network_mgr.filtered_sockets();

    let rows: Vec<Row> = filtered_sockets
        .iter()
        .map(|item| {
            let state_style = match item.state.as_str() {
                "LISTEN" => Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
                "ESTAB" | "ESTABLISHED" => Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
                "TIME_WAIT" | "CLOSE_WAIT" => theme.dim_style(),
                _ => Style::default().fg(theme.warning),
            };

            let scope_badge = if item.is_system {
                Span::styled(" [🔒 SYSTEM] ", theme.dim_style())
            } else {
                Span::styled(
                    " [USER] ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let pid_str = item
                .pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string());
            let remote_str = if item.peer_port == "*" {
                format!("{}:*", item.peer_addr)
            } else {
                format!("{}:{}", item.peer_addr, item.peer_port)
            };

            let cells = if w < 110 {
                vec![
                    Cell::from(item.local_port.to_string()).style(
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Cell::from(item.proto.clone()).style(Style::default().fg(theme.secondary)),
                    Cell::from(item.state.clone()).style(state_style),
                    Cell::from(item.proc_name.clone())
                        .style(Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                    Cell::from(pid_str).style(theme.dim_style()),
                    Cell::from(scope_badge),
                ]
            } else if w < 135 {
                vec![
                    Cell::from(item.local_port.to_string()).style(
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Cell::from(item.proto.clone()).style(Style::default().fg(theme.secondary)),
                    Cell::from(item.state.clone()).style(state_style),
                    Cell::from(item.local_addr.clone()).style(Style::default().fg(theme.fg)),
                    Cell::from(item.proc_name.clone())
                        .style(Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                    Cell::from(pid_str).style(theme.dim_style()),
                    Cell::from(scope_badge),
                ]
            } else {
                vec![
                    Cell::from(item.local_port.to_string()).style(
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Cell::from(item.proto.clone()).style(Style::default().fg(theme.secondary)),
                    Cell::from(item.state.clone()).style(state_style),
                    Cell::from(item.local_addr.clone()).style(Style::default().fg(theme.fg)),
                    Cell::from(remote_str).style(theme.dim_style()),
                    Cell::from(item.proc_name.clone())
                        .style(Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                    Cell::from(pid_str).style(theme.dim_style()),
                    Cell::from(scope_badge),
                ]
            };

            Row::new(cells).height(1)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg))
        .title(Span::styled(
            " 󰄲 Open Ports & Active Socket Map (↑/↓ Navigate, [k] Kill, [y] Copy) ",
            theme.title_style(),
        ));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme.selected_style());

    f.render_stateful_widget(table, area, &mut app.network_table_state);
}

fn render_socket_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            " 󰒺 Socket Inspector & Port Control ",
            theme.title_style(),
        ));

    let filtered = app.network_mgr.filtered_sockets();
    let selected_idx = app.network_table_state.selected().unwrap_or(0);
    let selected_socket = filtered.get(selected_idx).copied();

    let text = match selected_socket {
        Some(s) => {
            let pid_str = s
                .pid
                .map(|p| format!("PID {}", p))
                .unwrap_or_else(|| "Root Daemon (Hidden PID)".to_string());
            let remote_str = if s.peer_port == "*" {
                format!("{}:*", s.peer_addr)
            } else {
                format!("{}:{}", s.peer_addr, s.peer_port)
            };

            vec![
                Line::from(vec![
                    Span::styled(
                        format!(" Port: {} [{}] ", s.local_port, s.proto),
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!(" State: {} ", s.state),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!(" Bound Process: {} ({}) ", s.proc_name, pid_str),
                        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        " [k] Kill Port ",
                        Style::default()
                            .fg(theme.danger)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " [y] Copy ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Local Endpoint: ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        format!("{}:{} ", s.local_addr, s.local_port),
                        Style::default().fg(theme.fg),
                    ),
                    Span::styled(" ⇄ Remote Peer: ", Style::default().fg(theme.secondary)),
                    Span::styled(remote_str, theme.dim_style()),
                ]),
            ]
        }
        None => vec![Line::from(Span::styled(
            "No network socket selected",
            theme.dim_style(),
        ))],
    };

    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
