use anyhow::{self, Context};
use thiserror::Error;

use ninja_build::{build_externals, default_mtimestate, MTimeRebuilder, ParallelTopoScheduler};
use ninja_metrics::scoped_metric;
use ninja_parse::{build_representation, Loader};
use ninja_tasks::description_to_tasks;
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
}

struct FileLoader {}
impl Loader for FileLoader {
    fn load(&mut self, from: Option<&[u8]>, request: &[u8]) -> std::io::Result<Vec<u8>> {
        // TODO: Handle relative paths with from.
        assert!(from.is_none());
        std::fs::read(Path::new(OsStr::from_bytes(request)))
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
    let mut loader = FileLoader {};
    let repr = build_representation(&mut loader, config.build_file)?;
    // // at this point we should basically have a structure where all commands are fully expanded and
    // // ready to go.
    // Unlike a suspending/restarting + monadic tasks combination, and also because our tasks are
    // specified in a different language, we do need a separate way to get the dependencies for a
    // key.
    // This should also deal with multiple output keys.
    // Since each scheduler has additional execution strategies around async-ness for example, we
    // don't spit out executable tasks, instead just having an enum.
    let tasks = {
        scoped_metric!("to_tasks");
        description_to_tasks(repr)
    };

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
    let rebuilder: MTimeRebuilder<_> = MTimeRebuilder::new(default_mtimestate());
    let scheduler = ParallelTopoScheduler::new(config.parallelism);
    // let start = Start::All; // TODO: filter_keys();
    //build.build(keys_to_tasks, start);
    {
        scoped_metric!("build");
        build_externals(scheduler, rebuilder, &tasks, ())?;
    }
    // build log loading later
    if metrics_enabled {
        ninja_metrics::dump();
    }
    Ok(())
}
