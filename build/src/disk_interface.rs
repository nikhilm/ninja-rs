use ninja_metrics::scoped_metric;
use std::{io::Result, path::Path, time::SystemTime};

pub trait DiskInterface {
    fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime>;
}

pub struct SystemDiskInterface;
impl DiskInterface for SystemDiskInterface {
    fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
        scoped_metric!("stat");
        std::fs::metadata(p)?.modified()
    }
}
