use crate::{
    build_task::CommandTaskResult,
    caching_mtime_rebuilder,
    disk_interface::SystemDiskInterface,
    interface::Rebuilder,
    task::{Key, Task},
    CachingMTimeRebuilder, DiskDirtyCache,
};
use std::cell::Cell;

type InnerRebuilder = CachingMTimeRebuilder<DiskDirtyCache<SystemDiskInterface>>;
pub struct TrackingRebuilder {
    inner: InnerRebuilder,
    key_to_track: Key,
    required_rebuild: Cell<bool>,
}

impl TrackingRebuilder {
    pub fn with_caching_rebuilder(key: Key) -> Self {
        TrackingRebuilder {
            inner: caching_mtime_rebuilder(),
            key_to_track: key,
            required_rebuild: Cell::new(false),
        }
    }

    pub fn required_rebuild(&self) -> bool {
        self.required_rebuild.get()
    }
}

impl Rebuilder<Key, CommandTaskResult> for TrackingRebuilder {
    type Error = <InnerRebuilder as Rebuilder<Key, CommandTaskResult>>::Error;
    type Task = <InnerRebuilder as Rebuilder<Key, CommandTaskResult>>::Task;

    fn build(
        &self,
        key: Key,
        _unused: Option<CommandTaskResult>,
        task: &Task,
    ) -> Result<Option<Box<Self::Task>>, Self::Error> {
        let matches = key == self.key_to_track;
        let build_task = self.inner.build(key, _unused, task)?;
        if matches && build_task.is_some() {
            self.required_rebuild.set(true);
        }
        Ok(build_task)
    }
}
