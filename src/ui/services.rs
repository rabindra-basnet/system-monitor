use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filters & Scope Header
            Constraint::Min(8),    // Services Table + Details Sidebar
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_body(f, app, chunks[1]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg));

    let scope_str = if app.service_mgr.user_mode { "User (--user)" } else { "System (root)" };
    let filter_str = app.service_mgr.filter_state.label();

    let search_display = if app.service_mgr.search_query.is_empty() {
        Span::styled("None (Press '/' to filter)", theme.dim_style())
    } else {
        Span::styled(&app.service_mgr.search_query, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
    };

    let line = Line::from(vec![
        Span::styled(" 󰒺 Scope [u]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(scope_str, Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::raw("   |   "),
        Span::styled(" 󰄲 Status [f]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(filter_str, Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::raw("   |   "),
        Span::styled("  Search [/]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        search_display,
        Span::raw("   |   "),
        Span::styled(
            format!("{} services", app.service_mgr.filtered_services().len()),
            theme.dim_style(),
        ),
    ]);

    let p = Paragraph::new(line).block(block);
    f.render_widget(p, area);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let (table_pct, sidebar_pct) = if area.width < 110 {
        (60, 40)
    } else if area.width > 150 {
        (70, 30)
    } else {
        (65, 35)
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(table_pct), Constraint::Percentage(sidebar_pct)])
        .split(area);

    render_services_table(f, app, chunks[0]);
    render_service_sidebar(f, app, chunks[1]);
}

fn render_services_table(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let w = area.width;

    let header_cells = ["Scope", "Unit Name", "Active", "Sub", "Description"]
        .iter()
        .map(|&h| Cell::from(Span::styled(h, theme.header_style())));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let filtered = app.service_mgr.filtered_services();
    let rows: Vec<Row> = filtered
        .iter()
        .map(|s| {
            let is_sys_core = s.name.starts_with("systemd-") || s.name.starts_with("dbus") || s.name.starts_with("udev") || s.name.starts_with("polkit");
            let tag_cell = if s.is_user_unit {
                Span::styled(" [USER] ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
            } else if is_sys_core {
                Span::styled(" [🔒 SYSTEM] ", theme.dim_style())
            } else {
                Span::styled(" [DAEMON] ", Style::default().fg(theme.secondary))
            };

            let name_style = if is_sys_core && !s.is_user_unit {
                theme.dim_style()
            } else {
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
            };

            let active_style = if s.active_state == "failed" {
                Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)
            } else if is_sys_core && !s.is_user_unit {
                theme.dim_style()
            } else if s.active_state == "active" {
                Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
            } else {
                theme.dim_style()
            };

            let cells = vec![
                Cell::from(tag_cell),
                Cell::from(s.name.clone()).style(name_style),
                Cell::from(s.active_state.clone()).style(active_style),
                Cell::from(s.sub_state.clone()).style(theme.dim_style()),
                Cell::from(s.description.clone()).style(if is_sys_core && !s.is_user_unit { theme.dim_style() } else { Style::default().fg(theme.fg) }),
            ];

            Row::new(cells).height(1)
        })
        .collect();

    let widths = if w < 80 {
        vec![
            Constraint::Length(12),
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(10),
        ]
    } else {
        vec![
            Constraint::Length(14),
            Constraint::Length(30),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(20),
        ]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg))
        .title(Span::styled("  systemd Service Units ", theme.title_style()));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme.selected_style());

    f.render_stateful_widget(table, area, &mut app.service_table_state);
}

fn render_service_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󱕚 Unit Details & Controls ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Details card
            Constraint::Length(9),  // Action controls
            Constraint::Min(3),     // Tip
        ])
        .split(inner);

    let filtered = app.service_mgr.filtered_services();
    let selected_unit = app
        .service_table_state
        .selected()
        .and_then(|idx| filtered.get(idx).copied());

    if let Some(s) = selected_unit {
        let is_sys_core = s.name.starts_with("systemd-") || s.name.starts_with("dbus") || s.name.starts_with("udev") || s.name.starts_with("polkit");

        let details_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if is_sys_core && !s.is_user_unit { theme.warning } else { theme.border }))
            .title(Span::styled(" Service Info ", theme.title_style()));

        let text = vec![
            Line::from(vec![
                Span::styled("Unit: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled(&s.name, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("Active: ", Style::default().fg(theme.accent)),
                Span::styled(&s.active_state, if s.active_state == "active" { Style::default().fg(theme.success).add_modifier(Modifier::BOLD) } else { theme.dim_style() }),
                Span::raw(" ("),
                Span::styled(&s.sub_state, theme.dim_style()),
                Span::raw(")"),
            ]),
            Line::from(vec![
                Span::styled("Load: ", Style::default().fg(theme.accent)),
                Span::styled(&s.load_state, Style::default().fg(theme.secondary)),
            ]),
            Line::from(vec![
                Span::styled("Description: ", theme.dim_style()),
                Span::styled(&s.description, Style::default().fg(theme.fg)),
            ]),
        ];
        let p = Paragraph::new(text).block(details_block).wrap(Wrap { trim: true });
        f.render_widget(p, rows[0]);
    } else {
        let p = Paragraph::new("No service unit selected").style(theme.dim_style());
        f.render_widget(p, rows[0]);
    }

    // Actions block
    let actions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Management Controls ", theme.title_style()));

    let actions_text = vec![
        Line::from(vec![
            Span::styled(" [s] ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Start Service", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" [x] ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled("Stop Service", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" [r] ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Restart Service", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" [e] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Enable at Boot  ", Style::default().fg(theme.fg)),
            Span::styled(" [d] ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled("Disable", Style::default().fg(theme.fg)),
        ]),
    ];

    let p_actions = Paragraph::new(actions_text).block(actions_block);
    f.render_widget(p_actions, rows[1]);

    let note_text = vec![
        Line::from(Span::styled("💡 Tip:", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            "Press 'u' to switch between System (root) and User (--user) service units.",
            theme.dim_style(),
        )),
    ];
    let p_note = Paragraph::new(note_text).wrap(Wrap { trim: true });
    f.render_widget(p_note, rows[2]);
}
