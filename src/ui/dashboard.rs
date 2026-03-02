use image::{DynamicImage, RgbImage};
use plotters::prelude::*;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::{picker::Picker, StatefulImage};

use crate::app::App;
use crate::sampler::ProcessSampler;
use crate::ui::theme;

pub fn draw<S: ProcessSampler>(f: &mut Frame, app: &mut App<S>, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(50),
        Constraint::Percentage(50),
        Constraint::Length(3),
    ])
    .split(area);

    draw_summary(f, app, chunks[0]);

    // Extract chart data before mutably borrowing the picker
    let cpu_data = app.cpu_series.as_chart_data();
    let cpu_time_range = app.cpu_series.time_range();
    let mem_data = app.mem_series.as_chart_data();
    let mem_time_range = app.mem_series.time_range();
    let mem_max = app.mem_series.max_value();

    if let Some(picker) = &mut app.image_picker {
        draw_cpu_chart(f, chunks[1], picker, &cpu_data, cpu_time_range);
        draw_memory_chart(f, chunks[2], picker, &mem_data, mem_time_range, mem_max);
    }
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

struct ChartConfig<'a> {
    data: &'a [(f64, f64)],
    x_range: (f64, f64),
    y_range: (f64, f64),
    title: &'a str,
    line_color: &'a RGBColor,
    border_color: Style,
}

fn draw_cpu_chart(
    f: &mut Frame,
    area: Rect,
    picker: &mut Picker,
    data: &[(f64, f64)],
    time_range: Option<(f64, f64)>,
) {
    let (x_min, x_max) = time_range.unwrap_or((0.0, 60.0));
    let x_max = x_max.max(x_min + 10.0);

    render_chart(f, area, picker, &ChartConfig {
        data,
        x_range: (x_min, x_max),
        y_range: (0.0, 100.0),
        title: " CPU Usage ",
        line_color: &GREEN,
        border_color: Style::from(theme::CPU_COLOR),
    });
}

fn draw_memory_chart(
    f: &mut Frame,
    area: Rect,
    picker: &mut Picker,
    data: &[(f64, f64)],
    time_range: Option<(f64, f64)>,
    max_mem: f64,
) {
    let (x_min, x_max) = time_range.unwrap_or((0.0, 60.0));
    let x_max = x_max.max(x_min + 10.0);
    let y_max = if max_mem <= 0.0 { 100.0 } else { max_mem * 1.1 };

    render_chart(f, area, picker, &ChartConfig {
        data,
        x_range: (x_min, x_max),
        y_range: (0.0, y_max),
        title: " Memory (MB) ",
        line_color: &CYAN,
        border_color: Style::from(theme::MEMORY_COLOR),
    });
}

fn render_chart(f: &mut Frame, area: Rect, picker: &mut Picker, cfg: &ChartConfig) {
    // Render the ratatui block border + title around the chart area
    let block = Block::default()
        .title(Span::styled(cfg.title, cfg.border_color))
        .borders(Borders::ALL)
        .border_style(cfg.border_color);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (font_w, font_h) = picker.font_size();
    let pw = inner.width as u32 * font_w as u32;
    let ph = inner.height as u32 * font_h as u32;

    if pw == 0 || ph == 0 {
        return;
    }

    let (x_min, x_max) = cfg.x_range;
    let (y_min, y_max) = cfg.y_range;

    let mut buf = vec![0u8; (pw * ph * 3) as usize];

    // plotters rendering — no text (BitMapBackend panics without ttf).
    // Title/labels are handled by the ratatui Block above.
    let ok = (|| -> Result<(), Box<dyn std::error::Error>> {
        let backend = BitMapBackend::with_buffer(&mut buf, (pw, ph));
        let root = backend.into_drawing_area();
        root.fill(&RGBColor(26, 26, 46))?;

        let mut chart = ChartBuilder::on(&root)
            .margin(2)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)?;

        chart
            .configure_mesh()
            .disable_axes()
            .x_labels(0)
            .y_labels(0)
            .light_line_style(RGBColor(50, 50, 70))
            .draw()?;

        if cfg.data.len() >= 2 {
            chart.draw_series(LineSeries::new(
                cfg.data.iter().map(|&(x, y)| (x, y)),
                cfg.line_color,
            ))?;
        }

        root.present()?;
        Ok(())
    })();

    if ok.is_err() {
        return;
    }

    let Some(img) = RgbImage::from_raw(pw, ph, buf) else {
        return;
    };
    let dyn_img = DynamicImage::ImageRgb8(img);

    let mut protocol = picker.new_resize_protocol(dyn_img);
    let image_widget = StatefulImage::new(None);
    f.render_stateful_widget(image_widget, inner, &mut protocol);
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}
