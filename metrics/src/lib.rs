pub use once_cell::unsync::Lazy;
use std::{
    cell::RefCell,
    fmt,
    sync::atomic::{AtomicBool, Ordering},
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

pub struct ScopedMetric<'m> {
    metric_index: usize,
    metrics: &'m Metrics,
    start: Instant,
}

impl<'m> ScopedMetric<'m> {
    fn new(metric_index: usize, metrics: &'m Metrics) -> Self {
        ScopedMetric {
            metric_index,
            metrics,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for ScopedMetric<'a> {
    fn drop(&mut self) {
        self.metrics.record(self.metric_index, self.start.elapsed());
    }
}

#[derive(Debug)]
struct Metrics {
    metrics: Vec<RefCell<Metric>>,
}

impl Metrics {
    pub fn new_metric(&mut self, name: &'static str) -> usize {
        let len = self.metrics.len();
        self.metrics.push(RefCell::new(Metric {
            name,
            ..Default::default()
        }));
        len
    }

    fn record(&self, i: usize, elapsed: Duration) {
        self.metrics[i].borrow_mut().record(elapsed);
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metrics = &self.metrics;

        let mut name_width = 7; // To fit "metric ".
        for metric in metrics {
            let metric = metric.borrow();
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
            let metric = metric.borrow();
            write!(
                f,
                "{:name_width$} {: >6} {:>9} {:>11.3}\n",
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
            static mut _metric: $crate::Lazy<usize> =
                $crate::Lazy::new(|| $crate::new_metric($name));
            ::core::option::Option::Some($crate::new_scoped_metric(unsafe { *&*_metric }))
        } else {
            ::core::option::Option::None
        };
    };
}

static mut METRICS: Metrics = Metrics { metrics: vec![] };
static mut ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable() {
    unsafe { ENABLED.store(true, Ordering::Relaxed) };
}

pub fn is_enabled() -> bool {
    unsafe { ENABLED.load(Ordering::Relaxed) }
}

pub fn dump() {
    unsafe {
        eprintln!("{}", METRICS);
    }
}

pub fn new_metric(name: &'static str) -> usize {
    unsafe { METRICS.new_metric(name) }
}

pub fn new_scoped_metric<'a>(metric: usize) -> ScopedMetric<'a> {
    ScopedMetric::new(metric, unsafe { &METRICS })
}
