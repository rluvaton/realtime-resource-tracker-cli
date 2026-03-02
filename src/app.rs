use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::metrics::TimeSeries;
use crate::sampler::{ProcessInfo, ProcessSampler, Sampler};

const RING_CAPACITY: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Picker,
    Monitoring,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Cpu,
    Memory,
}

pub struct App<S: ProcessSampler = Sampler> {
    pub mode: AppMode,
    pub target_pid: u32,
    pub last_sample: Option<ProcessInfo>,
    pub cpu_series: TimeSeries,
    pub mem_series: TimeSeries,
    pub process_exited: bool,
    pub should_quit: bool,

    // Picker state
    pub search_query: String,
    pub all_processes: Vec<ProcessInfo>,
    pub filtered_processes: Vec<ProcessInfo>,
    pub picker_index: usize,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    last_refresh: Instant,

    pub sampler: S,
    start_time: Instant,
    interval_ms: u64,
}

impl App<Sampler> {
    pub fn new_monitoring(pid: u32, interval_secs: f64) -> Self {
        Self::new_monitoring_with_sampler(pid, interval_secs, Sampler::new())
    }

    pub fn new_picker(interval_secs: f64) -> Self {
        Self::new_picker_with_sampler(interval_secs, Sampler::new())
    }
}

impl<S: ProcessSampler> App<S> {
    pub fn new_monitoring_with_sampler(pid: u32, interval_secs: f64, sampler: S) -> Self {
        Self {
            mode: AppMode::Monitoring,
            target_pid: pid,
            last_sample: None,
            cpu_series: TimeSeries::new(RING_CAPACITY),
            mem_series: TimeSeries::new(RING_CAPACITY),
            process_exited: false,
            should_quit: false,
            search_query: String::new(),
            all_processes: Vec::new(),
            filtered_processes: Vec::new(),
            picker_index: 0,
            sort_column: SortColumn::Cpu,
            sort_ascending: false,
            last_refresh: Instant::now(),
            sampler,
            start_time: Instant::now(),
            interval_ms: (interval_secs * 1000.0) as u64,
        }
    }

    pub fn new_picker_with_sampler(interval_secs: f64, mut sampler: S) -> Self {
        let all = sampler.list_all_processes();
        let filtered = clone_process_list(&all);

        Self {
            mode: AppMode::Picker,
            target_pid: 0,
            last_sample: None,
            cpu_series: TimeSeries::new(RING_CAPACITY),
            mem_series: TimeSeries::new(RING_CAPACITY),
            process_exited: false,
            should_quit: false,
            search_query: String::new(),
            all_processes: all,
            filtered_processes: filtered,
            picker_index: 0,
            sort_column: SortColumn::Cpu,
            sort_ascending: false,
            last_refresh: Instant::now(),
            sampler,
            start_time: Instant::now(),
            interval_ms: (interval_secs * 1000.0) as u64,
        }
    }

    pub fn tick(&mut self) {
        match self.mode {
            AppMode::Monitoring => self.tick_monitoring(),
            AppMode::Picker => self.tick_picker(),
        }
    }

    fn tick_picker(&mut self) {
        if self.last_refresh.elapsed().as_secs() >= 2 {
            self.all_processes = self.sampler.list_all_processes();
            self.update_filter();
            self.last_refresh = Instant::now();
        }
    }

    fn tick_monitoring(&mut self) {
        if self.process_exited {
            return;
        }

        let elapsed = self.start_time.elapsed().as_secs_f64();

        match self.sampler.sample(self.target_pid) {
            Some(info) => {
                let mem_mb = info.memory_bytes as f64 / (1024.0 * 1024.0);
                self.cpu_series.push(elapsed, info.cpu_percent);
                self.mem_series.push(elapsed, mem_mb);
                self.last_sample = Some(info);
            }
            None => {
                self.process_exited = true;
            }
        }
    }

    pub fn handle_event(&mut self) -> anyhow::Result<()> {
        let timeout = std::time::Duration::from_millis(self.interval_ms);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }
                match self.mode {
                    AppMode::Monitoring => self.handle_monitoring_key(key.code),
                    AppMode::Picker => self.handle_picker_key(key.code),
                }
            }
        }
        Ok(())
    }

    pub fn handle_monitoring_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    pub fn handle_picker_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('q') if self.search_query.is_empty() => {
                self.should_quit = true;
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_filter();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_filter();
            }
            KeyCode::Up => {
                if self.picker_index > 0 {
                    self.picker_index -= 1;
                }
            }
            KeyCode::Down => {
                if !self.filtered_processes.is_empty()
                    && self.picker_index < self.filtered_processes.len() - 1
                {
                    self.picker_index += 1;
                }
            }
            KeyCode::Tab => {
                self.sort_column = match self.sort_column {
                    SortColumn::Cpu => SortColumn::Memory,
                    SortColumn::Memory => SortColumn::Cpu,
                };
                self.sort_ascending = false;
                self.sort_filtered();
            }
            KeyCode::Enter => {
                if let Some(proc) = self.filtered_processes.get(self.picker_index) {
                    self.target_pid = proc.pid;
                    self.mode = AppMode::Monitoring;
                    self.start_time = Instant::now();
                }
            }
            _ => {}
        }
    }

    fn update_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_processes = self
            .all_processes
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.command.to_lowercase().contains(&query)
                    || p.pid.to_string().contains(&query)
            })
            .map(clone_process_info)
            .collect();

        self.sort_filtered();

        if self.picker_index >= self.filtered_processes.len() {
            self.picker_index = self.filtered_processes.len().saturating_sub(1);
        }
    }

    fn sort_filtered(&mut self) {
        let asc = self.sort_ascending;
        match self.sort_column {
            SortColumn::Cpu => {
                self.filtered_processes.sort_by(|a, b| {
                    let cmp = a.cpu_percent.partial_cmp(&b.cpu_percent).unwrap();
                    if asc { cmp } else { cmp.reverse() }
                });
            }
            SortColumn::Memory => {
                self.filtered_processes.sort_by(|a, b| {
                    let cmp = a.memory_bytes.cmp(&b.memory_bytes);
                    if asc { cmp } else { cmp.reverse() }
                });
            }
        }
    }
}

fn clone_process_info(p: &ProcessInfo) -> ProcessInfo {
    ProcessInfo {
        pid: p.pid,
        name: p.name.clone(),
        command: p.command.clone(),
        cpu_percent: p.cpu_percent,
        memory_bytes: p.memory_bytes,
    }
}

fn clone_process_list(list: &[ProcessInfo]) -> Vec<ProcessInfo> {
    list.iter().map(clone_process_info).collect()
}
