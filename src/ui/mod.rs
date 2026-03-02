pub mod dashboard;
pub mod process_picker;
pub mod theme;

use ratatui::Frame;

use crate::app::{App, AppMode};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    if area.width < 40 || area.height < 10 {
        let msg = ratatui::widgets::Paragraph::new("Terminal too small. Please resize.")
            .style(theme::error_style());
        f.render_widget(msg, area);
        return;
    }

    match app.mode {
        AppMode::Picker => process_picker::draw(f, app, area),
        AppMode::Monitoring => dashboard::draw(f, app, area),
    }
}
