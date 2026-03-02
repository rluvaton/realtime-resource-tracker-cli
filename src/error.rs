use std::fmt;

#[derive(Debug)]
pub enum AppError {
    ProcessNotFound(u32),
    IntervalTooSmall(f64),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ProcessNotFound(pid) => write!(f, "Process with PID {} not found", pid),
            AppError::IntervalTooSmall(val) => {
                write!(f, "Interval {val}s is too small (minimum 0.1s)")
            }
        }
    }
}

impl std::error::Error for AppError {}
