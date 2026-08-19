pub mod applications;
pub mod autostart;
pub mod cleaner;
pub mod dashboard;
pub mod modals;
pub mod processes;
pub mod services;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, AppTab};
use crate::ui::modals::{render_modals, render_toast};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top Tabs & Header
            Constraint::Min(10),   // Main Tab Content View
            Constraint::Length(1), // Bottom Status / Key Hints Bar
        ])
        .split(f.area());

    render_tabs_header(f, app, chunks[0]);
    render_active_tab(f, app, chunks[1]);
    render_bottom_bar(f, app, chunks[2]);

    // Floating overlays
    render_toast(f, app, f.area());
    render_modals(f, app);
}

fn render_tabs_header(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let tab_titles: Vec<Line> = AppTab::all()
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let num_str = format!(" [{}] ", i + 1);
            let title_str = tab.title();
            Line::from(vec![
                Span::styled(num_str, Style::default().fg(theme.secondary)),
                Span::raw(title_str),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(" 󱐋 stasis (Linux System Optimizer) ", theme.title_style()));

    let tabs = Tabs::new(tab_titles)
        .block(block)
        .select(app.active_tab as usize)
        .style(Style::default().fg(theme.fg))
        .highlight_style(
            Style::default()
                .fg(theme.accent)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" │ ", theme.dim_style()));

    f.render_widget(tabs, area);
}

fn render_active_tab(f: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        AppTab::Dashboard => dashboard::render(f, app, area),
        AppTab::Processes => processes::render(f, app, area),
        AppTab::Cleaner => cleaner::render(f, app, area),
        AppTab::Services => services::render(f, app, area),
        AppTab::Autostart => autostart::render(f, app, area),
        AppTab::Applications => applications::render(f, app, area),
    }
}

fn render_bottom_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let specific_hint = match app.active_tab {
        AppTab::Dashboard => "[r] Refresh Telemetry",
        AppTab::Processes => "[k] Kill  [t] Term  [s] Sort  [/] Filter",
        AppTab::Cleaner => "[s] Scan  [Space] Toggle  [c] Clean",
        AppTab::Services => "[s] Start  [x] Stop  [r] Restart  [u] User/System  [f] Filter",
        AppTab::Autostart => "[Space] Toggle  [n] Add App  [d] Delete",
        AppTab::Applications => "[u] Uninstall  [f] Source  [s] Sort  [/] Search",
    };

    let line = Line::from(vec![
        Span::styled(" [1-6/Tab] Switch Tabs ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::raw(" │ "),
        Span::styled(format!(" {} ", specific_hint), Style::default().fg(theme.fg)),
        Span::raw(" │ "),
        Span::styled(" [t] Theme ", Style::default().fg(theme.secondary)),
        Span::styled(format!("({})", app.theme.mode.name()), theme.dim_style()),
        Span::raw(" │ "),
        Span::styled(" [?] Help ", Style::default().fg(theme.warning)),
        Span::raw(" │ "),
        Span::styled(" [q] Quit ", Style::default().fg(theme.danger)),
    ]);

    let p = Paragraph::new(line)
        .style(Style::default().bg(theme.card_bg).fg(theme.fg))
        .alignment(Alignment::Center);

    f.render_widget(p, area);
}
