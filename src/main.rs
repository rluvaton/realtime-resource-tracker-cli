use std::io;
use std::panic::catch_unwind;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::{Picker, ProtocolType};

use realtime_resource_tracker_cli::app::App;
use realtime_resource_tracker_cli::cli::Args;
use realtime_resource_tracker_cli::error::AppError;
use realtime_resource_tracker_cli::sampler::Sampler;
use realtime_resource_tracker_cli::ui;

/// RAII guard that restores the terminal on drop, even on panic.
struct TerminalGuard;

impl TerminalGuard {
    fn init() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.interval < 0.1 {
        return Err(AppError::IntervalTooSmall(args.interval).into());
    }

    // Detect terminal graphics protocol before entering raw mode.
    // from_query_stdio() actually tests the terminal's capabilities.
    // The fallback explicitly forces Halfblocks to avoid env-var heuristics
    // that pick wrong protocols in SSH/remote environments.
    let picker = if args.no_image {
        let mut p = Picker::from_fontsize((8, 16));
        p.set_protocol_type(ProtocolType::Halfblocks);
        p
    } else {
        catch_unwind(Picker::from_query_stdio)
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_else(|| {
                let mut p = Picker::from_fontsize((8, 16));
                p.set_protocol_type(ProtocolType::Halfblocks);
                p
            })
    };

    let mut app = if let Some(pid) = args.pid {
        let mut sampler = Sampler::new();
        if !sampler.pid_exists(pid) {
            return Err(AppError::ProcessNotFound(pid).into());
        }
        App::new_monitoring(pid, args.interval, picker)
    } else {
        App::new_picker(args.interval, picker)
    };

    let _guard = TerminalGuard;
    let mut terminal = TerminalGuard::init()?;

    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        app.handle_event()?;
        app.tick();
    }

    Ok(())
}
