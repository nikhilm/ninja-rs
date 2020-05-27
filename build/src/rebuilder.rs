use super::TaskResult;
use ninja_tasks::{Key, Task};
use std::{
    ffi::OsStr,
    fs::metadata,
    path::Path,
    string::FromUtf8Error,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

use std::os::unix::ffi::OsStrExt;

use crate::{
    disk_interface::DiskInterface,
    interface::{BuildTask, Rebuilder},
    task::{CommandTask, NoopTask},
};

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

    pub fn modified(&self, key: Key) -> std::io::Result<Option<SystemTime>> {
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

#[derive(Error, Debug)]
pub enum RebuilderError {
    #[error("utf-8 error")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("'{input}', needed by '{output}', missing and no known rule to make it")]
    MissingInput { output: String, input: String },
}

impl<Disk> Rebuilder<Key, TaskResult, (), RebuilderError> for MTimeRebuilder<Disk>
where
    Disk: DiskInterface,
{
    fn build(
        &self,
        key: Key,
        _current_value: TaskResult,
        task: &Task,
    ) -> Result<Box<dyn BuildTask<(), TaskResult> + Send>, RebuilderError> {
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
        let bools: std::result::Result<Vec<bool>, RebuilderError> = dependencies
            .iter()
            .map(|dep| match dep {
                Key::Single(ref dep_bytes) => {
                    let dep_mtime = self.mtime_state.modified(dep.clone()).expect("TODO");
                    if dep_mtime.is_none() {
                        let output = match key.clone() {
                            Key::Single(_) => String::from_utf8(key.as_bytes().to_vec())?,
                            Key::Multi(keys) => String::from_utf8(keys[0].as_bytes().to_vec())?,
                        };
                        return Err(RebuilderError::MissingInput {
                            input: String::from_utf8(dep_bytes.clone().to_vec())?,
                            output,
                        });
                    }
                    Ok(dep_mtime
                        .and_then(|dep_mtime| mtime.map(|m| dep_mtime > m))
                        .or_else(|| Some(false))
                        .unwrap_or(true))
                }
                Key::Multi(_) => {
                    assert!(task.is_retrieve());
                    assert_eq!(dependencies.len(), 1);
                    // Never actually need to do a retrieval action.
                    Ok(false)
                }
            })
            .collect();
        let bools = bools?;

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
            Ok(Box::new(CommandTask::new(task.command().unwrap().clone())))
        } else {
            // TODO: current value?
            Ok(Box::new(NoopTask {}))
        }
    }
}

#[cfg(test)]
mod test {
    use insta::assert_display_snapshot;
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
        let task = rebuilder
            .build(Key::Single(b"foo.o".to_vec()), TaskResult {}, &task)
            .expect("valid task");
        assert!(task.is_command());
    }

    /// A rule where the input does not exist should fail.
    #[test]
    fn test_input_does_not_exist() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
            }
        }

        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        // What we really want in this test is that if the input is not itself an output of
        // something else, then we want to error if it does not exist.
        // What we know: inputs MUST exist by the time a task is executed, because the scheduler
        // guarantees it.
        // that satisfies erroring out when inputs do not exist.
        // It probably won't satisfy "oh if this is wrapped in a phony". Actually it can, by
        // marking the mtimestate with some marker when the output did not exist. essentially a
        // "dirtiness" state instead of a mtimestate.
        let task = rebuilder.build(
            Key::Single(b"phony_user".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_err());
        match task {
            Err(e) => {
                assert_display_snapshot!(e);
            }
            _ => assert!(false, "Expected error"),
        }

        let task = rebuilder.build(
            Key::Single(b"phony_user".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
                variant: TaskVariant::Command("whatever".to_string()),
            },
        );
        assert!(task.is_err());
        match task {
            Err(e) => {
                assert_display_snapshot!(e);
            }
            _ => assert!(false, "Expected error"),
        }
    }

    #[test]
    fn test_input_does_not_exist_multiple_out_error_message() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
            }
        }

        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        let task = Task {
            dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
            variant: TaskVariant::Retrieve,
        };
        let task = rebuilder.build(
            Key::Multi(vec![
                Key::Single(b"phony_user".to_vec()),
                Key::Single(b"phony_user2".to_vec()),
            ]),
            TaskResult {},
            &task,
        );
        assert!(task.is_err());
        match task {
            Err(e) => {
                assert_display_snapshot!(e);
            }
            _ => assert!(false, "Expected error"),
        }
    }

    #[test]
    fn test_phony_input() {
        struct MockDiskInterface {}

        impl DiskInterface for MockDiskInterface {
            fn modified<P: AsRef<Path>>(&self, p: P) -> Result<SystemTime> {
                // This test should not hit disk.
                Err(Error::new(ErrorKind::NotFound, "mock not found"))
            }
        }

        let mock_disk = MockDiskInterface {};
        let state = MTimeState::new(mock_disk);
        let rebuilder = MTimeRebuilder::new(state);
        let task = rebuilder.build(
            Key::Single(b"phony_target_that_does_not_exist".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_ok());

        // Since the above marked the output as phony/dirty, this one should not fail because the
        // cache should treat it as dirty.
        let task = rebuilder.build(
            Key::Single(b"phony_user".to_vec()),
            TaskResult {},
            &Task {
                dependencies: vec![Key::Single(b"phony_target_that_does_not_exist".to_vec())],
                variant: TaskVariant::Retrieve,
            },
        );
        assert!(task.is_ok());
    }
}
