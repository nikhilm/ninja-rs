use std::process::Command;

use ninja_interface::Task;
// actually needs a buffer result or something.
pub struct TaskResult {}

#[derive(Debug)]
pub struct CommandTask {
    command: String,
}

impl CommandTask {
    fn new<S: Into<String>>(c: S) -> CommandTask {
        CommandTask { command: c.into() }
    }
}

// sigh! Wish this didn't need to be in the desc crate.
impl Task<TaskResult> for CommandTask {
    fn run(&self) -> TaskResult {
        eprintln!("{}", &self.command);
        Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .expect("success");
        TaskResult {}
    }
}

#[derive(Debug)]
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
