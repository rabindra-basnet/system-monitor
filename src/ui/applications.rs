use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::App;
use crate::system::applications::{format_installation_age, AppSortBy};
use crate::system::collector::format_bytes;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filters & Header
            Constraint::Min(8),    // Table + Details Sidebar
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

    let sort_name = match app.app_mgr.sort_by {
        AppSortBy::Size => "Size",
        AppSortBy::Age => "Age / Installed",
        AppSortBy::Name => "Name",
        AppSortBy::Source => "Source",
    };
    let dir_arrow = if app.app_mgr.sort_descending { "▼" } else { "▲" };

    let search_display = if app.app_mgr.search_query.is_empty() {
        Span::styled("None (Press '/' to search)", theme.dim_style())
    } else {
        Span::styled(&app.app_mgr.search_query, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
    };

    let line = Line::from(vec![
        Span::styled(" 󰄲 Source [f]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(app.app_mgr.source_filter.label(), Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::raw("   |   "),
        Span::styled(" 󰒺 Sort [s]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} {}", sort_name, dir_arrow), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        Span::raw("   |   "),
        Span::styled("  Search [/]: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        search_display,
        Span::raw("   |   "),
        Span::styled(
            format!("{} apps", app.app_mgr.filtered_items().len()),
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

    render_apps_table(f, app, chunks[0]);
    render_app_sidebar(f, app, chunks[1]);
}

fn render_apps_table(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let w = area.width;

    let (header_cells, widths): (Vec<&str>, Vec<Constraint>) = if w < 80 {
        (
            vec!["Scope", "Application", "Size", "Age", "Source"],
            vec![
                Constraint::Length(12),
                Constraint::Percentage(35),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(8),
            ],
        )
    } else {
        (
            vec!["Scope", "Application / Package", "Version", "Size", "Age / Installed", "Source", "Summary"],
            vec![
                Constraint::Length(14),
                Constraint::Length(22),
                Constraint::Length(14),
                Constraint::Length(10),
                Constraint::Length(15),
                Constraint::Length(9),
                Constraint::Min(20),
            ],
        )
    };

    let header = Row::new(header_cells.iter().map(|&h| Cell::from(Span::styled(h, theme.header_style()))))
        .height(1)
        .bottom_margin(1);

    let filtered = app.app_mgr.filtered_items();
    let rows: Vec<Row> = filtered
        .iter()
        .map(|item| {
            let (scope_badge, name_style, desc_style) = if item.is_essential {
                (
                    Span::styled(" [🔒 SYSTEM] ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                    theme.dim_style(),
                    theme.dim_style(),
                )
            } else if item.is_initial_install {
                (
                    Span::styled(" [ORIGINAL OS] ", theme.dim_style()),
                    Style::default().fg(theme.fg),
                    theme.dim_style(),
                )
            } else {
                (
                    Span::styled(" [USER APP] ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.fg),
                )
            };

            let source_style = match item.source.as_str() {
                "APT" | "Pacman" | "RPM" => Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                "Flatpak" => Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
                "Snap" => Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(theme.success).add_modifier(Modifier::BOLD),
            };

            let size_display = if item.size_bytes > 0 {
                format_bytes(item.size_bytes)
            } else {
                "-".to_string()
            };

            let age_display = format_installation_age(item.installed_time, item.is_initial_install);
            let age_style = if item.is_initial_install {
                theme.dim_style()
            } else {
                Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)
            };

            let cells = if w < 80 {
                vec![
                    Cell::from(scope_badge),
                    Cell::from(item.name.clone()).style(name_style),
                    Cell::from(size_display).style(if item.is_essential { theme.dim_style() } else { Style::default().fg(theme.warning) }),
                    Cell::from(age_display).style(age_style),
                    Cell::from(item.source.clone()).style(source_style),
                ]
            } else {
                vec![
                    Cell::from(scope_badge),
                    Cell::from(item.name.clone()).style(name_style),
                    Cell::from(item.version.clone()).style(theme.dim_style()),
                    Cell::from(size_display).style(if item.is_essential { theme.dim_style() } else { Style::default().fg(theme.warning) }),
                    Cell::from(age_display).style(age_style),
                    Cell::from(item.source.clone()).style(source_style),
                    Cell::from(item.description.clone()).style(desc_style),
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
        .title(Span::styled(" 󰏖 Installed Applications & Packages ", theme.title_style()));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme.selected_style());

    f.render_stateful_widget(table, area, &mut app.app_table_state);
}

fn render_app_sidebar(f: &mut Frame, app: &App, area: Rect) {
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
            Constraint::Length(10), // Details card
            Constraint::Length(6),  // Action controls
            Constraint::Min(3),     // Tip
        ])
        .split(inner);

    let filtered = app.app_mgr.filtered_items();
    let selected_app = app
        .app_table_state
        .selected()
        .and_then(|idx| filtered.get(idx).copied());

    if let Some(item) = selected_app {
        let details_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if item.is_essential { theme.warning } else { theme.border }))
            .title(Span::styled(" Package Info ", theme.title_style()));

        let size_str = if item.size_bytes > 0 {
            format_bytes(item.size_bytes)
        } else {
            "Unknown".to_string()
        };

        let status_badge = if item.is_essential {
            Span::styled(" [🔒 Core System — Protected]", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
        } else if item.is_initial_install {
            Span::styled(" [Original OS Pre-installation]", Style::default().fg(theme.secondary))
        } else {
            Span::styled(" [User Installed Application]", Style::default().fg(theme.success))
        };

        let age_str = format_installation_age(item.installed_time, item.is_initial_install);

        let text = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled(&item.name, Style::default().fg(theme.fg)),
                Span::raw("  "),
                Span::styled(format!("({})", item.source), Style::default().fg(theme.secondary)),
            ]),
            Line::from(vec![
                status_badge,
            ]),
            Line::from(vec![
                Span::styled("Version: ", Style::default().fg(theme.accent)),
                Span::styled(&item.version, Style::default().fg(theme.fg)),
                Span::raw("   "),
                Span::styled("Size: ", Style::default().fg(theme.accent)),
                Span::styled(size_str, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Installed: ", Style::default().fg(theme.accent)),
                Span::styled(age_str, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("ID / Path: ", theme.dim_style()),
                Span::styled(&item.package_id, theme.dim_style()),
            ]),
            Line::from(vec![
                Span::styled("Summary: ", theme.dim_style()),
                Span::styled(&item.description, Style::default().fg(theme.fg)),
            ]),
        ];
        let p = Paragraph::new(text).block(details_block).wrap(Wrap { trim: true });
        f.render_widget(p, rows[0]);
    } else {
        let p = Paragraph::new("No application selected").style(theme.dim_style());
        f.render_widget(p, rows[0]);
    }

    // Actions block
    let actions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Management Controls ", theme.title_style()));

    let is_essential = selected_app.map_or(false, |a| a.is_essential);

    let uninstall_line = if is_essential {
        Line::from(vec![
            Span::styled(" [u] ", theme.dim_style()),
            Span::styled("Uninstall: 🚫 Blocked (Protected System Package)", Style::default().fg(theme.danger)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [u] ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled("Uninstall / Remove Application", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
        ])
    };

    let actions_text = vec![
        uninstall_line,
        Line::from(vec![
            Span::styled(" [f] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Filter Sources  ", Style::default().fg(theme.fg)),
            Span::styled(" [s] ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled("Sort Column", Style::default().fg(theme.fg)),
        ]),
    ];

    let p_actions = Paragraph::new(actions_text).block(actions_block);
    f.render_widget(p_actions, rows[1]);

    let note_text = vec![
        Line::from(Span::styled("💡 Safety Note:", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            "Essential system packages are marked with 🔒 and dimmed to protect OS stability. Only remove standalone user applications.",
            theme.dim_style(),
        )),
    ];
    let p_note = Paragraph::new(note_text).wrap(Wrap { trim: true });
    f.render_widget(p_note, rows[2]);
}
