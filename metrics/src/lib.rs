use std::{
    cell::RefCell,
    fmt,
    sync::atomic::{AtomicBool, Ordering},
    thread_local,
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
struct Metric {
    name: &'static str,
    count: usize,
    sum: u128,
}

impl Metric {
    pub fn record(&mut self, elapsed: Duration) {
        self.count += 1;
        self.sum += elapsed.as_micros();
    }
}

pub struct ScopedMetric {
    metric_index: usize,
    start: Instant,
}

impl ScopedMetric {
    pub fn new(metric_index: usize) -> Self {
        ScopedMetric {
            metric_index,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedMetric {
    fn drop(&mut self) {
        METRICS.with(|m| {
            m.borrow_mut()
                .record(self.metric_index, self.start.elapsed())
        });
    }
}

#[derive(Debug)]
struct Metrics {
    metrics: Vec<Metric>,
}

impl Metrics {
    pub fn new_metric(&mut self, name: &'static str) -> usize {
        let len = self.metrics.len();
        self.metrics.push(Metric {
            name,
            ..Default::default()
        });
        len
    }

    fn record(&mut self, i: usize, elapsed: Duration) {
        self.metrics[i].record(elapsed);
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metrics = &self.metrics;

        let mut name_width = 7; // To fit "metric ".
        for metric in metrics {
            name_width = std::cmp::max(name_width, metric.name.len());
        }
        write!(
            f,
            "{:name_width$} {:>6} {:>9} {:>11}\n",
            "metric ",
            "count",
            "avg (us)",
            "total (us)",
            name_width = name_width
        )?;
        write!(
            f,
            "{:-<name_width$} {:-^6} {:-^9} {:-^11}\n",
            "",
            "",
            "",
            "",
            name_width = name_width
        )?;
        for metric in metrics {
            write!(
                f,
                "{:name_width$} {: >6} {:>9.3} {:>11}\n",
                metric.name,
                metric.count,
                metric.sum as f64 / metric.count as f64,
                metric.sum,
                name_width = name_width
            )?;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! scoped_metric {
    ($name:literal) => {
        // let _scoped_metric = new_scoped_metric($name);
        let _scoped_metric = if $crate::is_enabled() {
            thread_local! {
                static _metric: usize = $crate::new_metric($name);
            }
            ::core::option::Option::Some($crate::ScopedMetric::new(_metric.with(|m| *&*m)))
        } else {
            ::core::option::Option::None
        };
    };
}

thread_local! {
    static METRICS: RefCell<Metrics> = RefCell::new(Metrics { metrics: vec![] });
}
static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn dump() {
    METRICS.with(|m| {
        eprintln!("{}", m.borrow());
    })
}

pub fn new_metric(name: &'static str) -> usize {
    METRICS.with(|m| m.borrow_mut().new_metric(name))
}
