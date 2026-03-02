use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::sampler::ProcessSampler;
use crate::ui::theme;

pub fn draw<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    draw_search_box(f, app, chunks[0]);
    draw_process_list(f, app, chunks[1]);
    draw_help_bar(f, chunks[2]);
}

fn draw_search_box<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let input = format!("> {}", app.search_query);

    let block = Block::default()
        .title(Span::styled(" Search ", theme::search_style()))
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(input, theme::search_style()),
        Span::styled("█", Style::default().fg(theme::HIGHLIGHT_COLOR)),
    ]))
    .block(block);

    f.render_widget(paragraph, area);
}

fn draw_process_list<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let header = ListItem::new(Line::from(vec![
        Span::styled(format!("{:<8}", "PID"), theme::label_style()),
        Span::styled(format!("{:<20}", "NAME"), theme::label_style()),
        Span::styled(format!("{:>7}", "CPU%"), theme::cpu_style()),
        Span::styled(format!("{:>10}", "MEM (MB)"), theme::memory_style()),
    ]));

    let mut items = vec![header];

    for proc in &app.filtered_processes {
        let item = ListItem::new(Line::from(vec![
            Span::styled(format!("{:<8}", proc.pid), theme::label_style()),
            Span::styled(
                format!("{:<20}", truncate_str(&proc.name, 19)),
                theme::label_style(),
            ),
            Span::styled(format!("{:>7.1}", proc.cpu_percent), theme::cpu_style()),
            Span::styled(
                format!("{:>10.1}", proc.memory_bytes as f64 / (1024.0 * 1024.0)),
                theme::memory_style(),
            ),
        ]));
        items.push(item);
    }

    let block = Block::default()
        .title(Span::styled(
            format!(" Processes ({}) ", app.filtered_processes.len()),
            theme::label_style(),
        ))
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::highlight_style());

    // Offset list_state index by 1 to account for the header row
    let mut state = ListState::default();
    state.select(Some(app.picker_index + 1));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_help_bar(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ ", theme::highlight_style()),
        Span::styled(" Navigate  ", theme::label_style()),
        Span::styled(" Enter ", theme::highlight_style()),
        Span::styled(" Select  ", theme::label_style()),
        Span::styled(" Esc/q ", theme::highlight_style()),
        Span::styled(" Quit ", theme::label_style()),
    ]))
    .block(block);

    f.render_widget(help, area);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}
