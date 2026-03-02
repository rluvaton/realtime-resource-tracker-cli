use image::{DynamicImage, RgbImage};
use plotters::prelude::*;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};

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

    let cpu_data = app.cpu_series.as_chart_data();
    let cpu_time_range = app.cpu_series.time_range();
    let mem_data = app.mem_series.as_chart_data();
    let mem_time_range = app.mem_series.time_range();
    let mem_max = app.mem_series.max_value();

    let (x_min_cpu, x_max_cpu) = cpu_time_range.unwrap_or((0.0, 60.0));
    let x_max_cpu = x_max_cpu.max(x_min_cpu + 10.0);
    let (x_min_mem, x_max_mem) = mem_time_range.unwrap_or((0.0, 60.0));
    let x_max_mem = x_max_mem.max(x_min_mem + 10.0);
    let y_max_mem = if mem_max <= 0.0 { 100.0 } else { mem_max * 1.1 };

    let cpu_cfg = ChartConfig {
        data: &cpu_data,
        x_range: (x_min_cpu, x_max_cpu),
        y_range: (0.0, 100.0),
        title: " CPU Usage ",
        line_color: &GREEN,
        border_color: Style::from(theme::CPU_COLOR),
    };
    let mem_cfg = ChartConfig {
        data: &mem_data,
        x_range: (x_min_mem, x_max_mem),
        y_range: (0.0, y_max_mem),
        title: " Memory (MB) ",
        line_color: &CYAN,
        border_color: Style::from(theme::MEMORY_COLOR),
    };

    if let Some(picker) = &mut app.image_picker {
        if uses_image_protocol(picker) {
            render_chart_image(f, chunks[1], picker, &cpu_cfg);
            render_chart_image(f, chunks[2], picker, &mem_cfg);
        } else {
            render_chart_halfblocks(f, chunks[1], &cpu_cfg);
            render_chart_halfblocks(f, chunks[2], &mem_cfg);
        }
    } else {
        render_chart_halfblocks(f, chunks[1], &cpu_cfg);
        render_chart_halfblocks(f, chunks[2], &mem_cfg);
    }

    draw_help_bar(f, chunks[3]);
}

fn uses_image_protocol(picker: &Picker) -> bool {
    use ratatui_image::picker::ProtocolType;
    matches!(
        picker.protocol_type(),
        ProtocolType::Sixel | ProtocolType::Kitty | ProtocolType::Iterm2
    )
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

/// Render chart into the plotters pixel buffer. Returns false on error.
fn render_plotters(buf: &mut [u8], pw: u32, ph: u32, cfg: &ChartConfig) -> bool {
    let (x_min, x_max) = cfg.x_range;
    let (y_min, y_max) = cfg.y_range;

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let backend = BitMapBackend::with_buffer(buf, (pw, ph));
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

    result.is_ok()
}

// ── High-res image path (Sixel / Kitty / iTerm2) ──

fn render_chart_image(f: &mut Frame, area: Rect, picker: &mut Picker, cfg: &ChartConfig) {
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

    let mut buf = vec![0u8; (pw * ph * 3) as usize];
    if !render_plotters(&mut buf, pw, ph, cfg) {
        return;
    }

    let Some(img) = RgbImage::from_raw(pw, ph, buf) else {
        return;
    };
    let dyn_img = DynamicImage::ImageRgb8(img);

    let mut protocol: StatefulProtocol = picker.new_resize_protocol(dyn_img);
    f.render_stateful_widget(StatefulImage::new(None), inner, &mut protocol);
}

// ── Halfblock fallback path (works in all terminals) ──

fn render_chart_halfblocks(f: &mut Frame, area: Rect, cfg: &ChartConfig) {
    let block = Block::default()
        .title(Span::styled(cfg.title, cfg.border_color))
        .borders(Borders::ALL)
        .border_style(cfg.border_color);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let pw = inner.width as u32;
    let ph = inner.height as u32 * 2;

    let mut buf = vec![0u8; (pw * ph * 3) as usize];
    if !render_plotters(&mut buf, pw, ph, cfg) {
        return;
    }

    let rbuf = f.buffer_mut();
    for cy in 0..inner.height {
        for cx in 0..inner.width {
            let top_row = (cy as u32) * 2;
            let bot_row = top_row + 1;

            let top_off = ((top_row * pw + cx as u32) * 3) as usize;
            let bot_off = ((bot_row * pw + cx as u32) * 3) as usize;

            let upper = Color::Rgb(buf[top_off], buf[top_off + 1], buf[top_off + 2]);
            let lower = Color::Rgb(buf[bot_off], buf[bot_off + 1], buf[bot_off + 2]);

            if let Some(cell) = rbuf.cell_mut((inner.x + cx, inner.y + cy)) {
                cell.set_char('▀').set_fg(upper).set_bg(lower);
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}
