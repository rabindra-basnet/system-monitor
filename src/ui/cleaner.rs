use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::system::collector::format_bytes;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_categories_list(f, app, chunks[0]);
    render_summary_panel(f, app, chunks[1]);
}

fn render_categories_list(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg))
        .title(Span::styled(
            " 󰃢 System Cleaner Categories ",
            theme.title_style(),
        ));

    let items: Vec<ListItem> = app
        .cleaner
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_focused = i == app.cleaner_selected_index;
            let check_icon = if cat.selected { "󰄲" } else { "󰄱" };
            let check_style = if cat.selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.dim_style()
            };

            let name_style = if is_focused {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
            };

            let size_str = format_bytes(cat.total_size_bytes);
            let size_style = if cat.total_size_bytes > 0 {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.dim_style()
            };

            let tag_badge = if cat.requires_root {
                Span::styled(
                    " [⚠️ ROOT / SYSTEM] ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " [SAFE USER CACHE] ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", check_icon), check_style),
                    Span::styled(&cat.name, name_style),
                    tag_badge,
                    Span::styled(
                        format!("{} ({} files)", size_str, cat.file_count),
                        size_style,
                    ),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(&cat.description, theme.dim_style()),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(
                        "Target paths: ",
                        if is_focused {
                            Style::default().fg(theme.secondary)
                        } else {
                            theme.dim_style()
                        },
                    ),
                    Span::styled(
                        cat.paths
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        theme.dim_style(),
                    ),
                ]),
            ];

            let item = ListItem::new(lines);
            if is_focused {
                item.style(Style::default().bg(theme.selected_bg))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_summary_panel(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            " 󱕚 Cleaner Summary & Actions ",
            theme.title_style(),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Space stats card
            Constraint::Length(8), // Actions guide
            Constraint::Min(4),    // Info note
        ])
        .split(inner);

    // Space stats
    let total_bytes_str = format_bytes(app.cleaner.total_scanned_bytes);
    let total_files = app.cleaner.total_scanned_files;

    let stats_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" Reclaimable Space ", theme.title_style()));

    let stats_text = vec![
        Line::from(vec![
            Span::styled(
                " Total Size: ",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &total_bytes_str,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " Total Files: ",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", total_files),
                Style::default().fg(theme.warning),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Status: ", theme.dim_style()),
            Span::styled(
                if app.cleaner.is_busy {
                    "Scanning..."
                } else {
                    "Ready"
                },
                Style::default().fg(theme.accent),
            ),
        ]),
    ];

    let p_stats = Paragraph::new(stats_text).block(stats_block);
    f.render_widget(p_stats, rows[0]);

    // Actions block
    let actions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Quick Actions ", theme.title_style()));

    let actions_text = vec![
        Line::from(vec![
            Span::styled(
                " [s] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Scan System", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(
                " [Space] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Toggle Selection", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(
                " [a] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Select / Deselect All", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled(
                " [c] / [Enter] ",
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Clean Selected Files",
                Style::default()
                    .fg(theme.danger)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let p_actions = Paragraph::new(actions_text).block(actions_block);
    f.render_widget(p_actions, rows[1]);

    // Info note
    let info_text = vec![
        Line::from(Span::styled("💡 Tip:", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            "System package caches and system logs may require root permissions to delete. Run sysmon-tui with 'sudo' if you wish to purge root-owned cache archives.",
            theme.dim_style(),
        )),
    ];
    let p_info = Paragraph::new(info_text).wrap(Wrap { trim: true });
    f.render_widget(p_info, rows[2]);
}
