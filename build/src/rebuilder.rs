use super::TaskResult;
use ninja_interface::{BuildTask, Rebuilder};
use ninja_tasks::{Key, Task};
use std::{
    ffi::OsStr,
    fs::metadata,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::task::{CommandTask, NoopTask};

pub enum MTime {
    Unknown,
    Time(SystemTime),
}

#[derive(Debug, Default)]
pub struct MTimeState;

#[derive(Debug)]
pub struct MTimeRebuilder {
    mtime_state: MTimeState,
}

impl MTimeRebuilder {
    pub fn new(mtime_state: MTimeState) -> Self {
        Self { mtime_state }
    }
}

impl Rebuilder<Key, TaskResult, ()> for MTimeRebuilder {
    fn build(
        &self,
        key: Key,
        _current_value: TaskResult,
        task: &Task,
    ) -> Box<dyn BuildTask<(), TaskResult> + Send> {
        // This function obviously needs a lot of error handling.
        // Only returns the command task if required, otherwise a dummy.
        let mtime: Option<SystemTime> = match key.clone() {
            Key::Single(_) => {
                let path_str = std::str::from_utf8(key.as_bytes()).unwrap();
                let path_os: &OsStr = OsStr::new(path_str);
                let path = std::path::Path::new(path_os);
                if path.exists() {
                    Some(metadata(path).expect("metadata").modified().expect("mtime"))
                } else {
                    None
                }
            }
            Key::Multi(syms) => {
                // If the oldest output is older than any input, rebuild.
                let times: Vec<std::time::SystemTime> = syms
                    .iter()
                    .filter_map(|key| {
                        let path_str = std::str::from_utf8(key.as_bytes()).unwrap();
                        let path_os: &OsStr = OsStr::new(path_str);
                        let path = std::path::Path::new(path_os);
                        if path.exists() {
                            Some(metadata(path).expect("metadata").modified().expect("mtime"))
                        } else {
                            None
                        }
                    })
                    .collect();
                if times.len() < syms.len() {
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
                Key::Single(_) => {
                    let path_str = std::str::from_utf8(dep.as_bytes()).unwrap();
                    let result = metadata(path_str);
                    if result.is_err() {
                        let e = result.unwrap_err();
                        if e.kind() == std::io::ErrorKind::NotFound && task.is_retrieve() {
                            // It is OK for inputs to phony's to not exist.
                            false
                        } else {
                            panic!("oops what do i do here");
                        }
                    } else {
                        let dep_mtime = result.expect(path_str).modified().expect("mtime");
                        // If one of the outputs did not exist return true so the iterator check says
                        // dirty.
                        mtime.map(|m| dep_mtime > m).unwrap_or(true)
                    }
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
    // We need enough flexibility that we can test mock paths with mock mtimes and simulate the
    // relevant results we want. It would be nice to feed that to the actual rebuilder build fn.
}
