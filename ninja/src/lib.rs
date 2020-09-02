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
    task::{description_to_tasks, description_to_tasks_with_start, Key, KeyPath},
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
    if let Some(dir) = &config.execution_dir {
        std::env::set_current_dir(&dir).with_context(|| format!("changing to {} for -C", &dir))?;
    }

    let metrics_enabled = config.debug_modes.iter().any(|v| v == &DebugMode::Stats);
    if metrics_enabled {
        ninja_metrics::enable();
    }

    // OK let's try this out once.
    let scheduler = ParallelTopoScheduler::new(config.parallelism);

    let mut loader = FileLoader {};
    let repr = build_representation(&mut loader, config.build_file.into_bytes())?;
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
        if config.targets.is_empty() {
            description_to_tasks(repr)
        } else {
            description_to_tasks_with_start(
                repr,
                Some(config.targets.into_iter().map(|v| v.into_bytes()).collect()),
            )
        }
    };

    // TODO: Better names.
    let special_rebuilder =
        SpecialRebuilder::new(scheduler, Key::Path(config.build_file.into_bytes().into()));

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
    let rebuilder = caching_mtime_rebuilder();

    // To handle rebuilding the ninja file when any regeneration dependencies change:
    // 1. Create a paralleltoposcheduler and run a build with the start set as just that key.
    // 2. If that ended up changing the manifest, we need to go and re-parse everything again, so
    //    loop (and add a way to find if the manifest changed).
    // 3. If that failed, fail.
    // 4. If that succeeded but the manifest did not change, we are ready to start the build
    //    itself.
    // For that we may need to get our mtime state out of the rebuilder and then pass it back
    // in to a new rebuilder.
    // unless there is a way to propagate this information from the rebuilder to the scheduler
    // all the way up.
    // if rebuilder.into_state().dirtiness(build_key) == Dirtiness::Dirty {
    //     continue;
    // }
    // Hmm... we could directly call the rebuilder with a request to build and see if that
    // gives us a command task or a no-op task?
    // or could we hook into the scheduler with some kind of observer similar to the printer
    // and see if anything changed?
    // that would also help simplify some of the printer interface.
    // Really wish we had a "tell, don't ask" way to do this. For that, certainly having a
    // special task seems nice but it runs into trouble with things like recreating the
    // scheduler, sharing the console printer etc.
    //
    // Alternatively, could we inject a key + task into tasks which is a special task (not a
    // command task), that, when executed, effectively checks build.ninja and then starts the
    // whole "parse build.ninja and go" bit again, recursively?
    // That is, calling the rebuilder with that would return a special task that, if a rebuild
    // was required, would re-parse and start with a new tasks, and running that would be like
    // running this original rebuilder + scheduler? If build.ninja did not change, it would
    // again, just continue with this original rebuilder + scheduler?
    // Then, this function would degenerate to just calling a scheduler with that special
    // rebuilder?
    //
    // Ok, how about this. We have a custom rebuilder.
    // This rebuilder uses the same logic as an MTimeRebuilder, but, instead of returning a
    // command task/noop task, returns some other task.
    // We also have a separate scheduler.
    // If a build.ninja key does not exist, we want to re-use this rebuilder and scheduler so
    // we don't waste the effort spent building the graph. Then we pretty much want to proceed
    // as if we called build() or build_externals().
    // Now, the scheduler has no state across `schedule()` calls, so it can be re-used by
    // reference as many times as required.
    // So the rebuilder can:
    // - If build.ninja is dirty, return this special task, that re-parses etc.
    // - If build.ninja is clean, return a task that uses the scheduler with the tasks
    // description.
    // So this rebuilder is given a scheduler and a tasks map in the constructor to hold on to?
    // And how do we handle the "build.ninja" key does not exist?
    // We create one, set it to no dependencies/possibly phony, then do the same thing.
    //

    // Actual build.
    let scheduler = ParallelTopoScheduler::new(config.parallelism);
    {
        scoped_metric!("build");
        if let Some(requested) = requested {
            build(
                scheduler,
                &rebuilder,
                &tasks,
                requested.into_iter().map(Key::Path).collect(),
            )?;
        } else {
            build_externals(scheduler, &rebuilder, &tasks)?;
        }
    }
    // build log loading later
    if metrics_enabled {
        ninja_metrics::dump();
    }
    Ok(())
}
