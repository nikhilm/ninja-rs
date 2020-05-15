use super::TaskResult;
use ninja_interface::{BuildTask, Rebuilder};
use ninja_tasks::{Key, Task, Tasks};
use std::{ffi::OsStr, fs::metadata, os::unix::ffi::OsStrExt};

use crate::task::MTimeComparingCommandTask;

#[derive(Debug)]
pub struct MTimeState<'a> {
    pub(crate) tasks: &'a Tasks,
}
impl<'a> MTimeState<'a> {
    pub fn new(tasks: &'a Tasks) -> Self {
        MTimeState { tasks }
    }
}

#[derive(Debug, Default)]
pub struct MTimeRebuilder {}

impl<'b> Rebuilder<Key, TaskResult, MTimeState<'b>> for MTimeRebuilder {
    fn build<'a>(
        &self,
        key: Key,
        current_value: TaskResult,
        task: &'a Task,
    ) -> Box<dyn BuildTask<MTimeState<'b>, TaskResult> + 'a> {
        // This function obviously needs a lot of error handling.
        assert!(task.is_command());
        Box::new(MTimeComparingCommandTask::new(
            key,
            current_value,
            task,
            task.command().unwrap(),
        ))
    }
}
