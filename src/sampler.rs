use sysinfo::{Pid, System};

pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
}

pub trait ProcessSampler {
    fn sample(&mut self, pid: u32) -> Option<ProcessInfo>;
    fn list_all_processes(&mut self) -> Vec<ProcessInfo>;
}

pub struct Sampler {
    system: System,
    num_cpus: usize,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let num_cpus = system.cpus().len().max(1);
        Self { system, num_cpus }
    }

    /// Returns true if the given PID exists.
    pub fn pid_exists(&mut self, pid: u32) -> bool {
        self.system.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
        );
        self.system.process(Pid::from_u32(pid)).is_some()
    }
}

impl ProcessSampler for Sampler {
    fn sample(&mut self, pid: u32) -> Option<ProcessInfo> {
        self.system.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
        );

        let process = self.system.process(Pid::from_u32(pid))?;
        let cpu_raw = process.cpu_usage() as f64;
        let cpu_normalized = cpu_raw / self.num_cpus as f64;

        Some(ProcessInfo {
            pid,
            name: process.name().to_string_lossy().to_string(),
            cpu_percent: cpu_normalized.min(100.0),
            memory_bytes: process.memory(),
        })
    }

    fn list_all_processes(&mut self) -> Vec<ProcessInfo> {
        self.system.refresh_all();
        let num_cpus = self.num_cpus;

        let mut procs: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let cpu_raw = process.cpu_usage() as f64;
                ProcessInfo {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().to_string(),
                    cpu_percent: (cpu_raw / num_cpus as f64).min(100.0),
                    memory_bytes: process.memory(),
                }
            })
            .collect();

        procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
        procs
    }
}
