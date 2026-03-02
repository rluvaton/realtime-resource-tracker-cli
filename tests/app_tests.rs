use crossterm::event::KeyCode;
use realtime_resource_tracker_cli::app::{App, AppMode, SortColumn};
use realtime_resource_tracker_cli::metrics::TimeSeries;
use realtime_resource_tracker_cli::sampler::{ProcessInfo, ProcessSampler};

/// Mock sampler that returns configurable process data.
struct MockSampler {
    processes: Vec<ProcessInfo>,
    /// When true, sample() returns None (simulates process exit).
    process_alive: bool,
}

impl MockSampler {
    fn new(processes: Vec<ProcessInfo>) -> Self {
        Self {
            processes,
            process_alive: true,
        }
    }

    fn single(pid: u32, name: &str, cpu: f64, mem_bytes: u64) -> Self {
        Self::new(vec![ProcessInfo {
            pid,
            name: name.to_string(),
            command: name.to_string(),
            cpu_percent: cpu,
            memory_bytes: mem_bytes,
        }])
    }
}

impl ProcessSampler for MockSampler {
    fn sample(&mut self, pid: u32) -> Option<ProcessInfo> {
        if !self.process_alive {
            return None;
        }
        self.processes
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                command: p.command.clone(),
                cpu_percent: p.cpu_percent,
                memory_bytes: p.memory_bytes,
            })
    }

    fn list_all_processes(&mut self) -> Vec<ProcessInfo> {
        self.processes
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                command: p.command.clone(),
                cpu_percent: p.cpu_percent,
                memory_bytes: p.memory_bytes,
            })
            .collect()
    }
}

fn mock_process_list() -> Vec<ProcessInfo> {
    vec![
        ProcessInfo {
            pid: 1000,
            name: "node".to_string(),
            command: "node /app/server.js".to_string(),
            cpu_percent: 45.2,
            memory_bytes: 128 * 1024 * 1024,
        },
        ProcessInfo {
            pid: 2000,
            name: "rust-analyzer".to_string(),
            command: "rust-analyzer --stdio".to_string(),
            cpu_percent: 12.5,
            memory_bytes: 256 * 1024 * 1024,
        },
        ProcessInfo {
            pid: 3000,
            name: "firefox".to_string(),
            command: "firefox --new-window https://example.com".to_string(),
            cpu_percent: 8.1,
            memory_bytes: 512 * 1024 * 1024,
        },
        ProcessInfo {
            pid: 4000,
            name: "node-worker".to_string(),
            command: "node /app/worker.js --threads 4".to_string(),
            cpu_percent: 3.0,
            memory_bytes: 64 * 1024 * 1024,
        },
    ]
}

// ─── TimeSeries tests ───

#[test]
fn time_series_push_and_latest() {
    let mut ts = TimeSeries::new(10);
    assert!(ts.latest().is_none());

    ts.push(1.0, 42.0);
    let dp = ts.latest().unwrap();
    assert_eq!(dp.time, 1.0);
    assert_eq!(dp.value, 42.0);
}

#[test]
fn time_series_ring_buffer_evicts_oldest() {
    let mut ts = TimeSeries::new(3);
    ts.push(1.0, 10.0);
    ts.push(2.0, 20.0);
    ts.push(3.0, 30.0);
    assert_eq!(ts.len(), 3);

    ts.push(4.0, 40.0);
    assert_eq!(ts.len(), 3);

    let data = ts.as_chart_data();
    assert_eq!(data[0], (2.0, 20.0));
    assert_eq!(data[2], (4.0, 40.0));
}

#[test]
fn time_series_max_value() {
    let mut ts = TimeSeries::new(10);
    ts.push(0.0, 5.0);
    ts.push(1.0, 99.0);
    ts.push(2.0, 50.0);
    assert_eq!(ts.max_value(), 99.0);
}

#[test]
fn time_series_time_range() {
    let mut ts = TimeSeries::new(10);
    assert!(ts.time_range().is_none());

    ts.push(1.0, 0.0);
    ts.push(5.0, 0.0);
    ts.push(10.0, 0.0);
    assert_eq!(ts.time_range(), Some((1.0, 10.0)));
}

#[test]
fn time_series_as_chart_data() {
    let mut ts = TimeSeries::new(10);
    ts.push(1.0, 10.0);
    ts.push(2.0, 20.0);

    let data = ts.as_chart_data();
    assert_eq!(data, vec![(1.0, 10.0), (2.0, 20.0)]);
}

// ─── Monitoring mode tests ───

#[test]
fn monitoring_tick_records_cpu_and_memory() {
    let sampler = MockSampler::single(1234, "test-proc", 55.0, 100 * 1024 * 1024);
    let mut app = App::new_monitoring_with_sampler(1234, 1.0, sampler);

    assert_eq!(app.mode, AppMode::Monitoring);
    assert!(app.last_sample.is_none());
    assert_eq!(app.cpu_series.len(), 0);

    app.tick();

    assert!(app.last_sample.is_some());
    let sample = app.last_sample.as_ref().unwrap();
    assert_eq!(sample.pid, 1234);
    assert_eq!(sample.name, "test-proc");
    assert_eq!(sample.cpu_percent, 55.0);

    assert_eq!(app.cpu_series.len(), 1);
    assert_eq!(app.mem_series.len(), 1);

    let cpu_data = app.cpu_series.as_chart_data();
    assert_eq!(cpu_data[0].1, 55.0);

    let mem_data = app.mem_series.as_chart_data();
    let expected_mb = 100.0; // 100 MB
    assert!((mem_data[0].1 - expected_mb).abs() < 0.01);
}

#[test]
fn monitoring_tick_multiple_samples() {
    let sampler = MockSampler::single(1234, "test-proc", 25.0, 50 * 1024 * 1024);
    let mut app = App::new_monitoring_with_sampler(1234, 1.0, sampler);

    app.tick();
    app.tick();
    app.tick();

    assert_eq!(app.cpu_series.len(), 3);
    assert_eq!(app.mem_series.len(), 3);
}

#[test]
fn monitoring_process_exit_sets_flag() {
    let mut sampler = MockSampler::single(1234, "test-proc", 10.0, 1024 * 1024);
    sampler.process_alive = true;
    let mut app = App::new_monitoring_with_sampler(1234, 1.0, sampler);

    app.tick();
    assert!(!app.process_exited);
    assert_eq!(app.cpu_series.len(), 1);

    // Simulate process exit
    app.sampler.process_alive = false;
    app.tick();
    assert!(app.process_exited);

    // Further ticks should not add data
    let len_before = app.cpu_series.len();
    app.tick();
    assert_eq!(app.cpu_series.len(), len_before);
}

#[test]
fn monitoring_nonexistent_pid_exits_immediately() {
    let sampler = MockSampler::new(vec![]); // no processes
    let mut app = App::new_monitoring_with_sampler(9999, 1.0, sampler);

    app.tick();
    assert!(app.process_exited);
    assert!(app.last_sample.is_none());
}

#[test]
fn monitoring_q_quits() {
    let sampler = MockSampler::single(1, "proc", 0.0, 0);
    let mut app = App::new_monitoring_with_sampler(1, 1.0, sampler);

    assert!(!app.should_quit);
    app.handle_monitoring_key(KeyCode::Char('q'));
    assert!(app.should_quit);
}

#[test]
fn monitoring_esc_quits() {
    let sampler = MockSampler::single(1, "proc", 0.0, 0);
    let mut app = App::new_monitoring_with_sampler(1, 1.0, sampler);

    app.handle_monitoring_key(KeyCode::Esc);
    assert!(app.should_quit);
}

#[test]
fn monitoring_other_keys_do_not_quit() {
    let sampler = MockSampler::single(1, "proc", 0.0, 0);
    let mut app = App::new_monitoring_with_sampler(1, 1.0, sampler);

    app.handle_monitoring_key(KeyCode::Char('a'));
    assert!(!app.should_quit);

    app.handle_monitoring_key(KeyCode::Up);
    assert!(!app.should_quit);
}

// ─── Picker mode tests ───

#[test]
fn picker_loads_all_processes() {
    let sampler = MockSampler::new(mock_process_list());
    let app = App::new_picker_with_sampler(1.0, sampler);

    assert_eq!(app.mode, AppMode::Picker);
    assert_eq!(app.all_processes.len(), 4);
    assert_eq!(app.filtered_processes.len(), 4);
    assert_eq!(app.picker_index, 0);
}

#[test]
fn picker_search_filters_by_name() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Type "node" to filter
    app.handle_picker_key(KeyCode::Char('n'));
    app.handle_picker_key(KeyCode::Char('o'));
    app.handle_picker_key(KeyCode::Char('d'));
    app.handle_picker_key(KeyCode::Char('e'));

    assert_eq!(app.search_query, "node");
    assert_eq!(app.filtered_processes.len(), 2); // "node" and "node-worker"
    assert!(app
        .filtered_processes
        .iter()
        .all(|p| p.name.contains("node")));
}

#[test]
fn picker_search_filters_by_pid() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Type "3000" to filter by PID
    app.handle_picker_key(KeyCode::Char('3'));
    app.handle_picker_key(KeyCode::Char('0'));
    app.handle_picker_key(KeyCode::Char('0'));
    app.handle_picker_key(KeyCode::Char('0'));

    assert_eq!(app.filtered_processes.len(), 1);
    assert_eq!(app.filtered_processes[0].pid, 3000);
    assert_eq!(app.filtered_processes[0].name, "firefox");
}

#[test]
fn picker_search_filters_by_command() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Type "server.js" to filter by command string
    for c in "server.js".chars() {
        app.handle_picker_key(KeyCode::Char(c));
    }

    assert_eq!(app.filtered_processes.len(), 1);
    assert_eq!(app.filtered_processes[0].pid, 1000);
    assert_eq!(app.filtered_processes[0].name, "node");
    assert!(app.filtered_processes[0].command.contains("server.js"));
}

#[test]
fn picker_search_is_case_insensitive() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    app.handle_picker_key(KeyCode::Char('F'));
    app.handle_picker_key(KeyCode::Char('I'));
    app.handle_picker_key(KeyCode::Char('R'));

    assert_eq!(app.filtered_processes.len(), 1);
    assert_eq!(app.filtered_processes[0].name, "firefox");
}

#[test]
fn picker_backspace_widens_filter() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Filter down to "firefox"
    app.handle_picker_key(KeyCode::Char('f'));
    app.handle_picker_key(KeyCode::Char('i'));
    app.handle_picker_key(KeyCode::Char('r'));
    assert_eq!(app.filtered_processes.len(), 1);

    // Backspace widens filter
    app.handle_picker_key(KeyCode::Backspace);
    app.handle_picker_key(KeyCode::Backspace);
    app.handle_picker_key(KeyCode::Backspace);
    assert_eq!(app.search_query, "");
    assert_eq!(app.filtered_processes.len(), 4);
}

#[test]
fn picker_navigate_up_down() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    assert_eq!(app.picker_index, 0);

    app.handle_picker_key(KeyCode::Down);
    assert_eq!(app.picker_index, 1);

    app.handle_picker_key(KeyCode::Down);
    assert_eq!(app.picker_index, 2);

    app.handle_picker_key(KeyCode::Up);
    assert_eq!(app.picker_index, 1);

    app.handle_picker_key(KeyCode::Up);
    assert_eq!(app.picker_index, 0);

    // Should not go below 0
    app.handle_picker_key(KeyCode::Up);
    assert_eq!(app.picker_index, 0);
}

#[test]
fn picker_navigate_clamps_at_end() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Go to the end
    for _ in 0..10 {
        app.handle_picker_key(KeyCode::Down);
    }
    assert_eq!(app.picker_index, 3); // 4 items, max index 3
}

#[test]
fn picker_enter_selects_process_and_switches_to_monitoring() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Navigate to second process (rust-analyzer, pid 2000)
    app.handle_picker_key(KeyCode::Down);
    assert_eq!(app.picker_index, 1);

    app.handle_picker_key(KeyCode::Enter);

    assert_eq!(app.mode, AppMode::Monitoring);
    assert_eq!(app.target_pid, 2000);
}

#[test]
fn picker_enter_after_filter_selects_correct_process() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Filter to "firefox"
    app.handle_picker_key(KeyCode::Char('f'));
    app.handle_picker_key(KeyCode::Char('i'));
    app.handle_picker_key(KeyCode::Char('r'));
    assert_eq!(app.filtered_processes.len(), 1);

    app.handle_picker_key(KeyCode::Enter);

    assert_eq!(app.mode, AppMode::Monitoring);
    assert_eq!(app.target_pid, 3000);
}

#[test]
fn picker_esc_quits() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    app.handle_picker_key(KeyCode::Esc);
    assert!(app.should_quit);
}

#[test]
fn picker_q_quits_only_when_search_empty() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // q with empty search should quit
    app.handle_picker_key(KeyCode::Char('q'));
    assert!(app.should_quit);
}

#[test]
fn picker_q_types_when_search_nonempty() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Type something first so search is non-empty
    app.handle_picker_key(KeyCode::Char('a'));
    assert!(!app.should_quit);

    // Now q should type, not quit
    app.handle_picker_key(KeyCode::Char('q'));
    assert!(!app.should_quit);
    assert_eq!(app.search_query, "aq");
}

#[test]
fn picker_index_adjusts_when_filter_shrinks() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Navigate to last item
    app.handle_picker_key(KeyCode::Down);
    app.handle_picker_key(KeyCode::Down);
    app.handle_picker_key(KeyCode::Down);
    assert_eq!(app.picker_index, 3);

    // Now filter to only 1 result — index should clamp to 0
    app.handle_picker_key(KeyCode::Char('f'));
    app.handle_picker_key(KeyCode::Char('i'));
    app.handle_picker_key(KeyCode::Char('r'));
    assert_eq!(app.filtered_processes.len(), 1);
    assert_eq!(app.picker_index, 0);
}

#[test]
fn picker_no_match_shows_empty_list() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    app.handle_picker_key(KeyCode::Char('z'));
    app.handle_picker_key(KeyCode::Char('z'));
    app.handle_picker_key(KeyCode::Char('z'));

    assert_eq!(app.filtered_processes.len(), 0);
    assert_eq!(app.picker_index, 0);
}

#[test]
fn picker_enter_on_empty_list_does_not_switch_mode() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Filter to nothing
    app.handle_picker_key(KeyCode::Char('z'));
    app.handle_picker_key(KeyCode::Char('z'));
    assert_eq!(app.filtered_processes.len(), 0);

    app.handle_picker_key(KeyCode::Enter);
    assert_eq!(app.mode, AppMode::Picker); // should stay in picker
}

// ─── Sort tests ───

#[test]
fn picker_default_sort_is_cpu_descending() {
    let sampler = MockSampler::new(mock_process_list());
    let app = App::new_picker_with_sampler(1.0, sampler);

    assert_eq!(app.sort_column, SortColumn::Cpu);
    assert!(!app.sort_ascending);
    // First process should have highest CPU (45.2)
    assert_eq!(app.filtered_processes[0].pid, 1000);
    assert_eq!(app.filtered_processes[0].cpu_percent, 45.2);
}

#[test]
fn picker_tab_toggles_sort_column() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    assert_eq!(app.sort_column, SortColumn::Cpu);

    // Tab switches to Memory
    app.handle_picker_key(KeyCode::Tab);
    assert_eq!(app.sort_column, SortColumn::Memory);
    // Should be sorted by memory descending — firefox has most memory (512MB)
    assert_eq!(app.filtered_processes[0].pid, 3000);

    // Tab switches back to CPU
    app.handle_picker_key(KeyCode::Tab);
    assert_eq!(app.sort_column, SortColumn::Cpu);
    assert_eq!(app.filtered_processes[0].pid, 1000);
}

#[test]
fn picker_sort_by_memory_descending() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    app.handle_picker_key(KeyCode::Tab); // switch to Memory sort
    assert_eq!(app.sort_column, SortColumn::Memory);

    // Verify descending order: 512MB > 256MB > 128MB > 64MB
    assert_eq!(app.filtered_processes[0].pid, 3000); // firefox, 512MB
    assert_eq!(app.filtered_processes[1].pid, 2000); // rust-analyzer, 256MB
    assert_eq!(app.filtered_processes[2].pid, 1000); // node, 128MB
    assert_eq!(app.filtered_processes[3].pid, 4000); // node-worker, 64MB
}

// ─── Full flow: picker → monitoring ───

#[test]
fn full_flow_picker_to_monitoring_with_ticks() {
    let sampler = MockSampler::new(mock_process_list());
    let mut app = App::new_picker_with_sampler(1.0, sampler);

    // Select the first process (node, pid 1000)
    app.handle_picker_key(KeyCode::Enter);
    assert_eq!(app.mode, AppMode::Monitoring);
    assert_eq!(app.target_pid, 1000);

    // Tick to collect data
    app.tick();
    assert!(app.last_sample.is_some());
    let sample = app.last_sample.as_ref().unwrap();
    assert_eq!(sample.name, "node");
    assert_eq!(sample.cpu_percent, 45.2);

    assert_eq!(app.cpu_series.len(), 1);
    assert_eq!(app.mem_series.len(), 1);
}
