use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::App;
use crate::system::collector::format_bytes;
use crate::system::processes::{ProcessItem, ProcessSortBy};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if area.height >= 34 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search & Sort Header
                Constraint::Length(6), // Top Resource Eaters Snapshot Card
                Constraint::Min(10),   // Process Table
                Constraint::Length(4), // Process Detail & Clipboard Actions Bar
            ])
            .split(area);

        render_header(f, app, chunks[0]);
        render_top_eaters_card(f, app, chunks[1]);
        render_table(f, app, chunks[2]);
        render_detail(f, app, chunks[3]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search & Sort Header
                Constraint::Min(8),    // Process Table
                Constraint::Length(4), // Process Detail & Clipboard Actions Bar
            ])
            .split(area);

        render_header(f, app, chunks[0]);
        render_table(f, app, chunks[1]);
        render_detail(f, app, chunks[2]);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg));

    let sort_name = match app.process_mgr.sort_by {
        ProcessSortBy::Pid => "PID",
        ProcessSortBy::Name => "Name",
        ProcessSortBy::Cpu => "CPU %",
        ProcessSortBy::Memory => "Memory",
        ProcessSortBy::DiskRead => "Disk Read",
        ProcessSortBy::DiskWrite => "Disk Write",
    };

    let dir_arrow = if app.process_mgr.sort_descending {
        "▼ (Desc)"
    } else {
        "▲ (Asc)"
    };

    let search_display = if app.process_mgr.filter.is_empty() {
        Span::styled("None (Press '/' to filter)", theme.dim_style())
    } else {
        Span::styled(
            &app.process_mgr.filter,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )
    };

    let line = Line::from(vec![
        Span::styled(
            " 󰒺 Sort [s]: ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", sort_name),
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} [a]", dir_arrow),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   |   "),
        Span::styled(
            "  Filter [/]: ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        search_display,
        Span::raw("   |   "),
        Span::styled(
            " 📋 Copy [y] ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   |   "),
        Span::styled(
            format!("{} processes", app.process_list.len()),
            theme.dim_style(),
        ),
    ]);

    let p = Paragraph::new(line).block(block);
    f.render_widget(p, area);
}

fn render_top_eaters_card(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Calculate Top CPU processes
    let mut top_cpu: Vec<&ProcessItem> = app.process_list.iter().collect();
    top_cpu.sort_by(|a, b| {
        b.cpu_usage
            .partial_cmp(&a.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_3_cpu: Vec<&ProcessItem> = top_cpu.into_iter().take(3).collect();

    // Calculate Top Memory processes
    let mut top_mem: Vec<&ProcessItem> = app.process_list.iter().collect();
    top_mem.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    let top_3_mem: Vec<&ProcessItem> = top_mem.into_iter().take(3).collect();

    // Top CPU Card
    let cpu_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            " 🔥 Top CPU Consuming Processes ",
            theme.title_style(),
        ));

    let mut cpu_lines = Vec::new();
    for p in top_3_cpu {
        let color = if p.cpu_usage > 50.0 {
            theme.danger
        } else if p.cpu_usage > 20.0 {
            theme.warning
        } else {
            theme.success
        };
        cpu_lines.push(Line::from(vec![
            Span::styled(format!("  PID {:<5} ", p.pid), theme.dim_style()),
            Span::styled(
                format!("{:<16}", p.name),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>5.1}% CPU", p.cpu_usage),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" ({:.1}% RAM)", p.memory_pct), theme.dim_style()),
        ]));
    }
    let p_cpu = Paragraph::new(cpu_lines).block(cpu_block);
    f.render_widget(p_cpu, chunks[0]);

    // Top Memory Card
    let mem_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            " 󰍛 Top Memory Consuming Processes ",
            theme.title_style(),
        ));

    let mut mem_lines = Vec::new();
    for p in top_3_mem {
        mem_lines.push(Line::from(vec![
            Span::styled(format!("  PID {:<5} ", p.pid), theme.dim_style()),
            Span::styled(
                format!("{:<16}", p.name),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:>8}", format_bytes(p.memory_bytes)),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" ({:.1}%)", p.memory_pct), theme.dim_style()),
        ]));
    }
    let p_mem = Paragraph::new(mem_lines).block(mem_block);
    f.render_widget(p_mem, chunks[1]);
}

fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let w = area.width;

    let (header_cells, widths): (Vec<&str>, Vec<Constraint>) = if w < 105 {
        (
            vec!["PID", "Scope", "Name", "CPU %", "Memory", "Status"],
            vec![
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Percentage(35),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Min(8),
            ],
        )
    } else if w < 130 {
        (
            vec![
                "PID", "Scope", "Name", "User", "CPU %", "MEM %", "Memory", "Status",
            ],
            vec![
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Percentage(26),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(11),
                Constraint::Min(8),
            ],
        )
    } else {
        (
            vec![
                "PID", "Scope", "Name", "User", "CPU %", "MEM %", "Memory", "Disk R/s", "Disk W/s",
                "Status",
            ],
            vec![
                Constraint::Length(8),
                Constraint::Length(12),
                Constraint::Percentage(24),
                Constraint::Length(12),
                Constraint::Length(9),
                Constraint::Length(9),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(10),
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

    let rows: Vec<Row> = app
        .process_list
        .iter()
        .map(|item| {
            let status_style = if item.is_critical {
                theme.dim_style()
            } else {
                match item.status.as_str() {
                    "Running" => Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                    "Sleeping" => Style::default().fg(theme.text_dim),
                    "Zombie" | "Dead" => Style::default()
                        .fg(theme.danger)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default().fg(theme.warning),
                }
            };

            let scope_badge = if item.is_critical {
                Span::styled(" [🔒 SYSTEM] ", theme.dim_style())
            } else {
                Span::styled(
                    " [USER] ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let name_style = if item.is_critical {
                theme.dim_style()
            } else {
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
            };

            let cpu_style = if item.cpu_usage > 50.0 {
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD)
            } else if item.is_critical {
                theme.dim_style()
            } else if item.cpu_usage > 15.0 {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let cells = if w < 105 {
                vec![
                    Cell::from(item.pid.to_string()).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.accent)
                    }),
                    Cell::from(scope_badge),
                    Cell::from(item.name.clone()).style(name_style),
                    Cell::from(format!("{:.1}%", item.cpu_usage)).style(cpu_style),
                    Cell::from(format_bytes(item.memory_bytes)).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.fg)
                    }),
                    Cell::from(item.status.clone()).style(status_style),
                ]
            } else if w < 130 {
                vec![
                    Cell::from(item.pid.to_string()).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.accent)
                    }),
                    Cell::from(scope_badge),
                    Cell::from(item.name.clone()).style(name_style),
                    Cell::from(item.user.clone()).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.secondary)
                    }),
                    Cell::from(format!("{:.1}%", item.cpu_usage)).style(cpu_style),
                    Cell::from(format!("{:.1}%", item.memory_pct)).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.fg)
                    }),
                    Cell::from(format_bytes(item.memory_bytes)).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.fg)
                    }),
                    Cell::from(item.status.clone()).style(status_style),
                ]
            } else {
                vec![
                    Cell::from(item.pid.to_string()).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.accent)
                    }),
                    Cell::from(scope_badge),
                    Cell::from(item.name.clone()).style(name_style),
                    Cell::from(item.user.clone()).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.secondary)
                    }),
                    Cell::from(format!("{:.1}%", item.cpu_usage)).style(cpu_style),
                    Cell::from(format!("{:.1}%", item.memory_pct)).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.fg)
                    }),
                    Cell::from(format_bytes(item.memory_bytes)).style(if item.is_critical {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.fg)
                    }),
                    Cell::from(format!("{}/s", format_bytes(item.disk_read_bytes)))
                        .style(theme.dim_style()),
                    Cell::from(format!("{}/s", format_bytes(item.disk_write_bytes)))
                        .style(theme.dim_style()),
                    Cell::from(item.status.clone()).style(status_style),
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
            "  Process Table (↑/↓ Navigate, [s] Sort, [a] Direction, [/] Search) ",
            theme.title_style(),
        ));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme.selected_style());

    f.render_stateful_widget(table, area, &mut app.process_table_state);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            " 󰍹 Process Inspector & Actions ",
            theme.title_style(),
        ));

    let selected = app.selected_process();
    let text = match selected {
        Some(p) => {
            let cmd_display = if p.cmd.is_empty() {
                p.name.clone()
            } else {
                p.cmd.clone()
            };

            let scope_str = if p.is_critical {
                " [🔒 Protected System Process]"
            } else {
                " [User Process]"
            };

            vec![
                Line::from(vec![
                    Span::styled(
                        format!(" PID: {} ", p.pid),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        scope_str,
                        if p.is_critical {
                            theme.dim_style()
                        } else {
                            Style::default().fg(theme.success)
                        },
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!(" User: {} ", p.user),
                        Style::default().fg(theme.secondary),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!(" CPU: {:.1}% ", p.cpu_usage),
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!(
                            " RAM: {} ({:.1}%) ",
                            format_bytes(p.memory_bytes),
                            p.memory_pct
                        ),
                        Style::default().fg(theme.fg),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        " [y] Copy ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " [k] Kill ",
                        Style::default()
                            .fg(theme.danger)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " [t] Term ",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Command: ", Style::default().fg(theme.accent)),
                    Span::styled(cmd_display, theme.dim_style()),
                ]),
            ]
        }
        None => vec![Line::from(Span::styled(
            "No process selected",
            theme.dim_style(),
        ))],
    };

    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
