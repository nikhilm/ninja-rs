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

use anyhow::{self, Context};
use thiserror::Error;

use ninja_builder::{
    build, build_externals, caching_mtime_rebuilder,
    task::{description_to_tasks, description_to_tasks_with_start, Key},
    tracking_rebuilder::TrackingRebuilder,
    ParallelTopoScheduler,
};
use ninja_metrics::scoped_metric;
use ninja_parse::{build_representation, Loader};
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

/// Nothing to do with rustc debug vs. release.
/// This is just ninja terminology.
#[derive(Debug, PartialEq, Eq)]
pub enum DebugMode {
    List,
    Stats,
}

#[derive(Error, Debug)]
#[error("Unknown debug setting '{0}'")]
pub struct DebugModeError(String);

impl std::str::FromStr for DebugMode {
    type Err = DebugModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stats" => Ok(DebugMode::Stats),
            "list" => Ok(DebugMode::List),
            e @ _ => Err(DebugModeError(e.to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub execution_dir: Option<String>,
    pub parallelism: usize,
    pub build_file: String,
    pub debug_modes: Vec<DebugMode>,
    pub targets: Vec<String>,
}

struct FileLoader {}
impl Loader for FileLoader {
    fn load(&mut self, from: Option<&[u8]>, request: &[u8]) -> std::io::Result<Vec<u8>> {
        let path = if let Some(from) = from {
            let src_path = Path::new(OsStr::from_bytes(from));
            let req_path = Path::new(OsStr::from_bytes(request));
            if req_path.components().count() > 1 {
                todo!("handle relative paths");
            } else {
                src_path.with_file_name(req_path)
            }
        } else {
            Path::new(OsStr::from_bytes(request)).to_owned()
        };
        std::fs::read(path)
    }
}

pub fn run(config: Config) -> anyhow::Result<()> {
    ninja_builder::greet();
    if let Some(dir) = &config.execution_dir {
        std::env::set_current_dir(&dir).with_context(|| format!("changing to {} for -C", &dir))?;
    }

    let metrics_enabled = config.debug_modes.iter().any(|v| v == &DebugMode::Stats);
    if metrics_enabled {
        ninja_metrics::enable();
    }

    let mut loader = FileLoader {};

    for _ in 1..=100 {
        let build_key = Key::Path(config.build_file.clone().into_bytes().into());
        let repr = build_representation(&mut loader, config.build_file.clone().into_bytes())?;
        // // at this point we should basically have a structure where all commands are fully expanded and
        // // ready to go.
        // Unlike a suspending/restarting + monadic tasks combination, and also because our tasks are
        // specified in a different language, we do need a separate way to get the dependencies for a
        // key.
        // This should also deal with multiple output keys.
        // Since each scheduler has additional execution strategies around async-ness for example, we
        // don't spit out executable tasks, instead just having an enum.
        let (tasks, requested) = {
            scoped_metric!("to_tasks");
            let targets_clone = config.targets.clone();
            if targets_clone.is_empty() {
                description_to_tasks(repr)
            } else {
                description_to_tasks_with_start(
                    repr,
                    Some(targets_clone.into_iter().map(|v| v.into_bytes()).collect()),
                )
            }
        };

        let scheduler = ParallelTopoScheduler::new(config.parallelism);

        if tasks.task(&build_key).is_some() {
            let rebuilder = TrackingRebuilder::with_caching_rebuilder(build_key.clone());
            // let build_task = rebuilder.build(build_key, None, task)?;
            build(&scheduler, &rebuilder, &tasks, vec![build_key])?;
            // TODO: How do we determine if it was already up to date!
            if rebuilder.required_rebuild() {
                // Re-parse and try again.
                continue;
            }
        }

        // BTW, one way to model cheap string/byte references by index without having to pass lifetimes
        // and refs everywhere is to have things that need to go back tothe string/byte sequence
        // explicitly require the intern lookup object to be passed in.

        // Ready to build.
        // let _state = BuildLog::read();
        //let mut store = DiskStore::new();
        // TODO: This can all hide behind the build constructor right?
        // So this could be just a function according to the paper, as long as it followed a certain
        // signature. Fn(k, v, task) -> Task
        // We may want to pass an mtime oracle here instead of making mtimerebuilder aware of the
        // filesystem.
        {
            let rebuilder = caching_mtime_rebuilder();
            scoped_metric!("build");
            if let Some(requested) = requested {
                build(
                    &scheduler,
                    &rebuilder,
                    &tasks,
                    requested.into_iter().map(Key::Path).collect(),
                )?;
            } else {
                build_externals(&scheduler, &rebuilder, &tasks)?;
            }
        }
        break;
    }
    // build log loading later
    if metrics_enabled {
        ninja_metrics::dump();
    }
    Ok(())
}
