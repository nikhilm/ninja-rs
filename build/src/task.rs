use std::{ffi::OsStr, fs::metadata, process::Command, time::SystemTime};

use ninja_interface::BuildTask;
use ninja_tasks::{Key, Task};

use crate::{rebuilder::MTimeState, TaskResult};

pub trait ParallelTopoTask<State>: BuildTask<State, TaskResult> {}

#[derive(Debug)]
pub struct MTimeComparingCommandTask<'a> {
    key: Key,
    current_value: TaskResult,
    task: &'a Task,
    command: &'a str,
}

impl<'a> MTimeComparingCommandTask<'a> {
    pub fn new(
        key: Key,
        current_value: TaskResult,
        task: &'a Task,
        command: &'a str,
    ) -> MTimeComparingCommandTask<'a> {
        MTimeComparingCommandTask {
            key,
            current_value,
            task,
            command,
        }
    }

    pub fn run_command(&self) -> TaskResult {
        eprintln!("{}", &self.command);
        Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .expect("success");
        // TODO: Handle failure here and throughout the build graph.
        TaskResult {}
    }
}

impl<'a> BuildTask<MTimeState<'_>, TaskResult> for MTimeComparingCommandTask<'a> {
    fn run(&self, state: &MTimeState) -> TaskResult {
        // Only runs the inner task if required, otherwise succeeds.
        let mtime: Option<SystemTime> = match &self.key {
            Key::Single(_) => {
                let path_str = state.tasks.path_for(&self.key).unwrap();
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
                        let path_str = state.tasks.path_for(&Key::Single(*path_ref)).unwrap();
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
            let dependencies = self.task.dependencies();
            eprintln!("DEPS {:?}", dependencies);
            // We could use iter.any, but that will short circuit and not check every file for
            // existence. not sure what we want here.
            let bools: Vec<bool> = dependencies
                .iter()
                .map(|dep| match dep {
                    Key::Single(path_ref) => {
                        let path_str = state.tasks.path_for(&Key::Single(*path_ref)).unwrap();
                        eprintln!("DEP {}", path_str);
                        let dep_mtime = metadata(path_str)
                            .expect(path_str)
                            .modified()
                            .expect("mtime");
                        dep_mtime > mtime
                    }
                    Key::Multi(_) => {
                        panic!("Should never have a command task with a multi-key input!");
                    }
                })
                .collect();
            bools.iter().any(|b| *b)
        };
        if dirty {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            self.run_command()
        } else {
            // TODO: current value?
            TaskResult {}
        }
    }
}
