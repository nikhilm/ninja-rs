use super::TaskResult;
use ninja_tasks::{Key, Task};
use std::{
    ffi::OsStr,
    fs::metadata,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use std::os::unix::ffi::OsStrExt;

use crate::{
    disk_interface::DiskInterface,
    interface::{BuildTask, Rebuilder},
    task::{CommandTask, NoopTask},
};
use std::io::Result;

#[derive(Debug, Default)]
pub struct MTimeState<Disk>
where
    Disk: DiskInterface,
{
    disk: Disk,
}

impl<Disk> MTimeState<Disk>
where
    Disk: DiskInterface,
{
    pub fn new(disk: Disk) -> Self {
        MTimeState { disk }
    }

    pub fn modified(&self, key: Key) -> Result<Option<SystemTime>> {
        // TODO: Cache
        self.disk
            .modified(OsStr::from_bytes(key.as_bytes()))
            .map(|x| Some(x))
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(e)
                }
            })
    }
}

#[derive(Debug)]
pub struct MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    mtime_state: MTimeState<Disk>,
}

impl<Disk> MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    pub fn new(mtime_state: MTimeState<Disk>) -> Self {
        Self { mtime_state }
    }
}

impl<Disk> Rebuilder<Key, TaskResult, ()> for MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    fn build(
        &self,
        key: Key,
        _current_value: TaskResult,
        task: &Task,
    ) -> Box<dyn BuildTask<(), TaskResult> + Send> {
        // This function obviously needs a lot of error handling.
        // Only returns the command task if required, otherwise a dummy.
        let mtime: Option<SystemTime> = match key.clone() {
            key @ Key::Single(_) => self.mtime_state.modified(key).expect("TODO"),
            Key::Multi(keys) => {
                // If the oldest output is older than any input, rebuild.
                let times: Vec<std::time::SystemTime> = keys
                    .iter()
                    .filter_map(|key| self.mtime_state.modified(key.clone()).expect("TODO"))
                    .collect();
                if times.len() < keys.len() {
                    // At least one output did not exist, so always build.
                    // But... if we return None here, we will run the script w/o verifying that all
                    // inputs actually exist before running the command. So use this instead.
                    None
                } else {
                    Some(times.into_iter().min().expect("at least one"))
                }
            }
        };

        // Iterate inputs to make sure they exist, regardless of what outputs were determined.
        let dependencies = task.dependencies();
        // We could use iter.any, but that will short circuit and not check every file for
        // existence. not sure what we want here.
        let bools: Vec<bool> = dependencies
            .iter()
            .map(|dep| match dep {
                key @ Key::Single(_) => {
                    let dep_mtime = self.mtime_state.modified(key.clone()).expect("TODO");
                    dep_mtime
                        .and_then(|dep_mtime| mtime.map(|m| dep_mtime > m))
                        .or_else(|| Some(false))
                        .unwrap_or(true)
                }
                Key::Multi(_) => {
                    assert!(task.is_retrieve());
                    assert_eq!(dependencies.len(), 1);
                    // Never actually need to do a retrieval action.
                    false
                }
            })
            .collect();

        let dirty = mtime.is_none() || {
            if bools.is_empty() {
                // If there were no inputs, consider dirty if outputs were missing.
                mtime.is_none()
            } else {
                let x = bools.iter().any(|b| *b);
                x
            }
        };
        eprintln!("dirty? {}", dirty);
        if dirty && task.is_command() {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            Box::new(CommandTask::new(task.command().unwrap().clone()))
        } else {
            // TODO: current value?
            Box::new(NoopTask {})
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        io::{Error, ErrorKind, Result},
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use ninja_tasks::*;

    // We need enough flexibility that we can test mock paths with mock mtimes and simulate the
    // relevant results we want. It would be nice to feed that to the actual rebuilder build fn.
    #[test]
    fn test_basic() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
                if p.as_ref() == Path::new("foo.c") {
                    Ok(UNIX_EPOCH.checked_add(Duration::from_secs(100)).unwrap())
                } else {
                    Err(Error::new(ErrorKind::NotFound, "mock not found"))
                }
            }
        }

        // TODO: Starting point for making the rebuilder depend more on the state and "fixing"
        // the state to be more trait-y as well as not always depend on IO.
        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        let task = Task {
            dependencies: vec![Key::Single(b"foo.c".to_vec())],
            variant: TaskVariant::Command("cc -c foo.c".to_owned()),
        };
        eprintln!("Task is command {}", task.is_command());
        let task = rebuilder.build(Key::Single(b"foo.o".to_vec()), TaskResult {}, &task);
        assert!(task.is_command());
    }

    /// A phony rule where the input does not exist should fail.
    #[test]
    fn test_phony_compatibility1() {
        // let task = Task {
        //     dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
        //     variant: TaskVariant::Retrieve,
        // };
        // let task = rebuilder.build(Key::Single(b"foo.o".to_vec()), TaskResult {}, &task);
        // assert!(task.is_command());
    }
}
