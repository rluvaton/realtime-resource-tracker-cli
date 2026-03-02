use ratatui::style::{Color, Modifier, Style};

pub const CPU_COLOR: Color = Color::Green;
pub const MEMORY_COLOR: Color = Color::Cyan;
pub const BORDER_COLOR: Color = Color::Yellow;
pub const LABEL_COLOR: Color = Color::White;
pub const AXIS_COLOR: Color = Color::Gray;
pub const ERROR_COLOR: Color = Color::Red;
pub const HIGHLIGHT_COLOR: Color = Color::Yellow;
pub const SEARCH_COLOR: Color = Color::White;

pub fn cpu_style() -> Style {
    Style::default().fg(CPU_COLOR)
}

pub fn memory_style() -> Style {
    Style::default().fg(MEMORY_COLOR)
}

pub fn border_style() -> Style {
    Style::default().fg(BORDER_COLOR)
}

pub fn label_style() -> Style {
    Style::default().fg(LABEL_COLOR)
}

pub fn axis_style() -> Style {
    Style::default().fg(AXIS_COLOR)
}

pub fn error_style() -> Style {
    Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD)
}

pub fn highlight_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(HIGHLIGHT_COLOR)
        .add_modifier(Modifier::BOLD)
}

pub fn search_style() -> Style {
    Style::default().fg(SEARCH_COLOR)
}
