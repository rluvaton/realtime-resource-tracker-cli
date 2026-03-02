use std::collections::VecDeque;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DataPoint {
    pub time: f64,
    pub value: f64,
}

pub struct TimeSeries {
    data: VecDeque<DataPoint>,
    capacity: usize,
}

#[allow(dead_code)]
impl TimeSeries {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, time: f64, value: f64) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(DataPoint { time, value });
    }

    pub fn latest(&self) -> Option<&DataPoint> {
        self.data.back()
    }

    pub fn max_value(&self) -> f64 {
        self.data
            .iter()
            .map(|d| d.value)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Returns data as (x, y) pairs suitable for ratatui's Dataset.
    pub fn as_chart_data(&self) -> Vec<(f64, f64)> {
        self.data.iter().map(|d| (d.time, d.value)).collect()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn time_range(&self) -> Option<(f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let first = self.data.front().unwrap().time;
        let last = self.data.back().unwrap().time;
        Some((first, last))
    }
}
