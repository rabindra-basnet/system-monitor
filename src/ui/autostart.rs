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
            Constraint::Length(3), // Search Header
            Constraint::Min(8),    // Table + Sidebar
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

    let search_display = if app.autostart_mgr.search_query.is_empty() {
        Span::styled("None (Press '/' to filter)", theme.dim_style())
    } else {
        Span::styled(&app.autostart_mgr.search_query, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
    };

    let line = Line::from(vec![
        Span::styled(" 󱑞 Startup Applications ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::raw("   |   "),
        Span::styled("  Search [/]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        search_display,
        Span::raw("   |   "),
        Span::styled(
            format!("{} items configured", app.autostart_mgr.items.len()),
            theme.dim_style(),
        ),
    ]);

    let p = Paragraph::new(line).block(block);
    f.render_widget(p, area);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    render_autostart_table(f, app, chunks[0]);
    render_autostart_sidebar(f, app, chunks[1]);
}

fn render_autostart_table(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;

    let header_cells = ["Status", "Name", "Exec Command", "Scope"]
        .iter()
        .map(|&h| Cell::from(Span::styled(h, theme.header_style())));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let filtered = app.autostart_mgr.filtered_items();
    let rows: Vec<Row> = filtered
        .iter()
        .map(|(_, item)| {
            let (status_text, status_style) = if item.enabled {
                ("ENABLED", Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
            } else {
                ("DISABLED", theme.dim_style())
            };

            let scope_str = if item.is_user { "User" } else { "System" };
            let scope_style = if item.is_user {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.secondary)
            };

            let cells = vec![
                Cell::from(format!("  {}  ", status_text)).style(status_style),
                Cell::from(item.name.clone()).style(Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Cell::from(item.exec.clone()).style(theme.dim_style()),
                Cell::from(scope_str).style(scope_style),
            ];

            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(22),
        Constraint::Min(24),
        Constraint::Length(10),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg))
        .title(Span::styled(" 󱑞 Autostart Entries ", theme.title_style()));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme.selected_style());

    f.render_stateful_widget(table, area, &mut app.autostart_table_state);
}

fn render_autostart_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󱕚 Application Details ", theme.title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Details card
            Constraint::Length(6), // Action controls
            Constraint::Min(3),    // Note
        ])
        .split(inner);

    let filtered = app.autostart_mgr.filtered_items();
    let selected_item = app
        .autostart_table_state
        .selected()
        .and_then(|idx| filtered.get(idx).map(|(_, it)| *it));

    if let Some(item) = selected_item {
        let details_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(" Desktop Entry ", theme.title_style()));

        let text = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled(&item.name, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("Exec: ", Style::default().fg(theme.secondary)),
                Span::styled(&item.exec, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("Path: ", theme.dim_style()),
                Span::styled(item.file_path.display().to_string(), theme.dim_style()),
            ]),
            Line::from(vec![
                Span::styled("Comment: ", theme.dim_style()),
                Span::styled(
                    if item.comment.is_empty() { "(none)" } else { &item.comment },
                    theme.dim_style(),
                ),
            ]),
        ];
        let p = Paragraph::new(text).block(details_block).wrap(Wrap { trim: true });
        f.render_widget(p, rows[0]);
    } else {
        let p = Paragraph::new("No autostart app selected").style(theme.dim_style());
        f.render_widget(p, rows[0]);
    }

    // Actions block
    let actions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Controls ", theme.title_style()));

    let actions_text = vec![
        Line::from(vec![
            Span::styled(" [Space] / [Enter] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle Status", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" [n] ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Add New Autostart App", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(" [d] / [Del] ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled("Delete Entry", Style::default().fg(theme.fg)),
        ]),
    ];

    let p_actions = Paragraph::new(actions_text).block(actions_block);
    f.render_widget(p_actions, rows[1]);

    let note_text = vec![
        Line::from(Span::styled("💡 Tip:", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            "Toggling system-wide entries automatically overrides them locally in ~/.config/autostart without modifying system root directories.",
            theme.dim_style(),
        )),
    ];
    let p_note = Paragraph::new(note_text).wrap(Wrap { trim: true });
    f.render_widget(p_note, rows[2]);
}
