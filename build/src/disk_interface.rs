use std::{io::Result, path::Path, time::SystemTime};

pub trait DiskInterface {
    fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime>;
}

pub struct SystemDiskInterface;
impl DiskInterface for SystemDiskInterface {
    fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
        std::fs::metadata(p)?.modified()
    }
}
