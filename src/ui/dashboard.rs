use ratatui::{
    layout::{Constraint, Layout, Rect},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

use crate::app::App;
use crate::sampler::ProcessSampler;
use crate::ui::theme;

pub fn draw<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(50),
        Constraint::Percentage(50),
        Constraint::Length(3),
    ])
    .split(area);

    draw_summary(f, app, chunks[0]);
    draw_cpu_chart(f, app, chunks[1]);
    draw_memory_chart(f, app, chunks[2]);
    draw_help_bar(f, chunks[3]);
}

fn draw_help_bar(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" q/Esc ", theme::highlight_style()),
        Span::styled(" Quit ", theme::label_style()),
    ]))
    .block(block);

    f.render_widget(help, area);
}

fn draw_summary<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let (pid, name, cpu, mem) = if let Some(info) = &app.last_sample {
        (
            format!("{}", info.pid),
            info.name.clone(),
            format!("{:.1}%", info.cpu_percent),
            format_bytes(info.memory_bytes),
        )
    } else {
        (
            format!("{}", app.target_pid),
            String::from("—"),
            String::from("—"),
            String::from("—"),
        )
    };

    let status_text = if app.process_exited {
        vec![Span::styled(" [Process Exited]", theme::error_style())]
    } else {
        vec![]
    };

    let mut spans = vec![
        Span::styled(" PID: ", theme::label_style()),
        Span::styled(pid, theme::label_style()),
        Span::styled("  │  ", theme::axis_style()),
        Span::styled("Name: ", theme::label_style()),
        Span::styled(name, theme::label_style()),
        Span::styled("  │  ", theme::axis_style()),
        Span::styled("CPU: ", theme::label_style()),
        Span::styled(cpu, theme::cpu_style()),
        Span::styled("  │  ", theme::axis_style()),
        Span::styled("Memory: ", theme::label_style()),
        Span::styled(mem, theme::memory_style()),
    ];
    spans.extend(status_text);

    let block = Block::default()
        .title(Span::styled(" Process Info ", theme::label_style()))
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_cpu_chart<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let cpu_data = app.cpu_series.as_chart_data();

    let (x_min, x_max) = app.cpu_series.time_range().unwrap_or((0.0, 60.0));
    let x_max = x_max.max(x_min + 10.0);

    let x_labels = make_x_labels(x_min, x_max);
    let y_labels = vec![
        Span::styled("0%", theme::axis_style()),
        Span::styled("25%", theme::axis_style()),
        Span::styled("50%", theme::axis_style()),
        Span::styled("75%", theme::axis_style()),
        Span::styled("100%", theme::axis_style()),
    ];

    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::HalfBlock)
        .graph_type(GraphType::Line)
        .style(theme::cpu_style())
        .data(&cpu_data)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(" CPU Usage ", theme::cpu_style()))
                .borders(Borders::ALL)
                .border_style(Style::from(theme::CPU_COLOR)),
        )
        .x_axis(
            Axis::default()
                .title(Span::styled("Time (s)", theme::axis_style()))
                .style(theme::axis_style())
                .bounds([x_min, x_max])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .style(theme::axis_style())
                .bounds([0.0, 100.0])
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

fn draw_memory_chart<S: ProcessSampler>(f: &mut Frame, app: &App<S>, area: Rect) {
    let mem_data = app.mem_series.as_chart_data();

    let (x_min, x_max) = app.mem_series.time_range().unwrap_or((0.0, 60.0));
    let x_max = x_max.max(x_min + 10.0);

    let max_mem = app.mem_series.max_value();
    let y_max = if max_mem <= 0.0 { 100.0 } else { max_mem * 1.1 };

    let x_labels = make_x_labels(x_min, x_max);
    let y_labels = make_mem_y_labels(y_max);

    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::HalfBlock)
        .graph_type(GraphType::Line)
        .style(theme::memory_style())
        .data(&mem_data)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(" Memory (MB) ", theme::memory_style()))
                .borders(Borders::ALL)
                .border_style(Style::from(theme::MEMORY_COLOR)),
        )
        .x_axis(
            Axis::default()
                .title(Span::styled("Time (s)", theme::axis_style()))
                .style(theme::axis_style())
                .bounds([x_min, x_max])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .style(theme::axis_style())
                .bounds([0.0, y_max])
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

fn make_x_labels(x_min: f64, x_max: f64) -> Vec<Span<'static>> {
    let mid = (x_min + x_max) / 2.0;
    vec![
        Span::styled(format!("{:.0}", x_min), theme::axis_style()),
        Span::styled(format!("{:.0}", mid), theme::axis_style()),
        Span::styled(format!("{:.0}", x_max), theme::axis_style()),
    ]
}

fn make_mem_y_labels(y_max: f64) -> Vec<Span<'static>> {
    let steps = 4;
    (0..=steps)
        .map(|i| {
            let val = y_max * i as f64 / steps as f64;
            Span::styled(format!("{:.0}", val), theme::axis_style())
        })
        .collect()
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

use ratatui::style::Style;
