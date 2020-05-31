pub use once_cell::unsync::Lazy;
use std::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
pub struct Metric {
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
    metric: &'static mut Metric,
    start: Instant,
}
impl ScopedMetric {
    pub fn new(metric: &'static mut Metric) -> Self {
        ScopedMetric {
            metric,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedMetric {
    fn drop(&mut self) {
        self.metric.record(self.start.elapsed());
    }
}

#[derive(Debug)]
pub struct Metrics {
    metrics: Vec<Box<Metric>>,
}

impl Metrics {
    pub fn new_metric(&mut self, name: &'static str) -> &mut Metric {
        let len = self.metrics.len();
        self.metrics.push(Box::new(Metric {
            name,
            ..Default::default()
        }));
        &mut self.metrics[len]
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut name_width = 7; // To fit "metric ".
        for metric in &self.metrics {
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
        for metric in &self.metrics {
            write!(
                f,
                "{:name_width$} {: >6} {:>9} {:>11.3}\n",
                metric.name,
                metric.sum,
                metric.count,
                metric.sum as f64 / metric.count as f64,
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
            static mut _metric: $crate::Lazy<&mut $crate::Metric> =
                $crate::Lazy::new(|| unsafe { $crate::METRICS.new_metric($name) });
            ::core::option::Option::Some($crate::ScopedMetric::new(unsafe { &mut _metric }))
        } else {
            ::core::option::Option::None
        };
    };
}

pub static mut METRICS: Lazy<Metrics> = Lazy::new(|| Metrics { metrics: vec![] });
static mut ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable() {
    unsafe { ENABLED.store(true, Ordering::Relaxed) };
}

pub fn is_enabled() -> bool {
    unsafe { ENABLED.load(Ordering::Relaxed) }
}

pub fn dump() {
    unsafe { eprintln!("{}", &*METRICS) };
}
