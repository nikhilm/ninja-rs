/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::process::Output;
use tokio::process::Command;

use thiserror::Error;

use crate::interface::BuildTask;

use crate::TaskResult;

use async_trait::async_trait;

#[derive(Error, Debug)]
pub enum CommandTaskError {
    #[error("{0}")]
    SpawnFailed(#[from] std::io::Error),
    #[error("failed with {}", .0.status)]
    CommandFailed(Output),
}

pub type CommandTaskResult = Result<Output, CommandTaskError>;

#[derive(Debug)]
pub struct CommandTask {
    command: String,
}

impl CommandTask {
    pub fn new(command: String) -> CommandTask {
        CommandTask { command }
    }

    pub async fn run_command(&self) -> CommandTaskResult {
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .output()
            .await?;
        if !output.status.success() {
            return Err(CommandTaskError::CommandFailed(output));
        }
        Ok(output)
    }
}

#[async_trait(?Send)]
impl<State> BuildTask<State, TaskResult> for CommandTask {
    async fn run(&self, _state: &State) -> TaskResult {
        self.run_command().await
    }

    #[cfg(test)]
    fn is_command(&self) -> bool {
        true
    }
}
