use super::TaskResult;
use ninja_interface::{BuildTask, Rebuilder};
use ninja_tasks::{Key, Task, Tasks};
use std::{ffi::OsStr, fs::metadata, os::unix::ffi::OsStrExt, time::SystemTime};

use crate::task::{CommandTask, NoopTask};

// Doesn't do anything right now, but we may want to cache mtimes.
#[derive(Debug, Default)]
pub struct MTimeState;

#[derive(Debug)]
pub struct MTimeRebuilder<'a> {
    mtime_state: MTimeState,
    tasks: &'a Tasks,
}

impl<'a> MTimeRebuilder<'a> {
    pub fn new(mtime_state: MTimeState, tasks: &'a Tasks) -> Self {
        Self { mtime_state, tasks }
    }
}

impl<'a> Rebuilder<Key, TaskResult, ()> for MTimeRebuilder<'a> {
    fn build<'b>(
        &self,
        key: Key,
        current_value: TaskResult,
        task: &'b Task,
    ) -> Box<dyn BuildTask<(), TaskResult> + 'b + Send> {
        // This function obviously needs a lot of error handling.
        // Only returns the command task if required, otherwise a dummy.
        let mtime: Option<SystemTime> = match key.clone() {
            Key::Single(_) => {
                let path_str = self.tasks.path_for(&key).unwrap();
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
                    .filter_map(|path_ref| {
                        let path_str = self.tasks.path_for(&Key::Single(*path_ref)).unwrap();
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
                    Some(std::time::UNIX_EPOCH)
                } else {
                    Some(times.into_iter().min().expect("at least one"))
                }
            }
        };
        let dirty = if mtime.is_none() {
            true
        } else {
            let mtime = mtime.unwrap();
            let dependencies = task.dependencies();
            eprintln!("DEPS {:?}", dependencies);
            // We could use iter.any, but that will short circuit and not check every file for
            // existence. not sure what we want here.
            let bools: Vec<bool> = dependencies
                .iter()
                .map(|dep| match dep {
                    Key::Single(path_ref) => {
                        let path_str = self.tasks.path_for(&Key::Single(*path_ref)).unwrap();
                        eprintln!("DEP {}", path_str);
                        let dep_mtime = metadata(path_str)
                            .expect(path_str)
                            .modified()
                            .expect("mtime");
                        dep_mtime > mtime
                    }
                    Key::Multi(_) => {
                        assert!(task.is_retrieve());
                        assert_eq!(dependencies.len(), 1);
                        false
                    }
                })
                .collect();
            bools.iter().any(|b| *b)
        };
        if dirty && task.is_command() {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            Box::new(CommandTask::new(task.command().unwrap()))
        } else {
            // TODO: current value?
            Box::new(NoopTask {})
        }
    }
}
