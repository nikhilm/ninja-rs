use std::{ffi::OsStr, os::unix::ffi::OsStrExt, process::Command};

use ninja_interface::Task;
// actually needs a buffer result or something.
pub struct TaskResult {}

#[derive(Debug)]
pub struct CommandTask {
    command: Vec<u8>,
}

impl CommandTask {
    pub fn new<S: Into<Vec<u8>>>(c: S) -> CommandTask {
        CommandTask { command: c.into() }
    }
}

// sigh! Wish this didn't need to be in the desc crate.
impl Task<TaskResult> for CommandTask {
    fn run(&self) -> TaskResult {
        let command_str: &OsStr = OsStrExt::from_bytes(&self.command);
        println!("{}", std::str::from_utf8(&self.command).unwrap());
        // POSIX only
        Command::new("/bin/sh")
            .arg("-c")
            .arg(command_str)
            .status()
            .expect("success");
        TaskResult {}
    }
}

#[derive(Debug, Default)]
pub struct PhonyTask {}

impl PhonyTask {
    fn new() -> PhonyTask {
        PhonyTask {}
    }
}

impl Task<TaskResult> for PhonyTask {
    fn run(&self) -> TaskResult {
        TaskResult {}
    }
}
