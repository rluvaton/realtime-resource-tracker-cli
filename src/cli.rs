use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rt-tracker", about = "Real-time process resource tracker")]
pub struct Args {
    /// Process ID to monitor. If omitted, an interactive picker is shown.
    #[arg(short, long)]
    pub pid: Option<u32>,

    /// Sampling interval in seconds (minimum 0.1)
    #[arg(short, long, default_value_t = 1.0)]
    pub interval: f64,
}
