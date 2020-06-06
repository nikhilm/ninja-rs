use std::fmt;

use anyhow::{self, Context};
use structopt::StructOpt;

use ninja_build::{build_externals, default_mtimestate, MTimeRebuilder, ParallelTopoScheduler};
use ninja_desc::to_description;
use ninja_metrics::scoped_metric;
use ninja_parse::Parser;
use ninja_tasks::description_to_tasks;

#[derive(Debug)]
pub struct NumCpus(usize);

/*
 * This type is required to work around some shortcomings in structopt.
 *
 * 1. We want a default value for this that is dynamic (number of CPUs).
 * 2. We want the actual text in [default...] to have some extra text besides just the number.
 * 3. We want the default to be inferred from FromStr correctly when the user specifies nothing.
 *
 * To satisfy 1, we wrap usize in a newtype and impl a Default on it.
 * To satisfy 2, realize that structopt uses ToString+Default, and ToString is implied by Display.
 * Tack on the extra description in Display.
 *
 * 3 is tricky.
 * To actually get the value, structopt will call FromStr on whatever value exists. When the user
 * passes something, it is whatever the user passed and we can use usize::from_str. But when the
 * user didn't pass anything, FromStr is called on the value of ToString. This has our suffix,
 * which we need to remove. Otherwise the parse will fail and the command fails as the "user" did
 * not specify a valid default.
 */
impl NumCpus {
    const SUFFIX: &'static str = ", derived from CPUs available";
}

impl Default for NumCpus {
    fn default() -> NumCpus {
        NumCpus(num_cpus::get())
    }
}

impl fmt::Display for NumCpus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, derived from CPUs available", self.0)
    }
}

impl std::str::FromStr for NumCpus {
    type Err = <usize as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with(NumCpus::SUFFIX) {
            Ok(NumCpus(usize::from_str(
                &s[..(s.len() - NumCpus::SUFFIX.len())],
            )?))
        } else {
            Ok(NumCpus(usize::from_str(s)?))
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "ninjars",
    usage = "ninjars [options] [targets...]\n\nIf targets are unspecified, builds the 'default' target (see the manual).",
    version = "0.1.0"
)]
pub struct Config {
    /// change to DIR before doing anything else
    #[structopt(short = "-C", name = "DIR")]
    pub execution_dir: Option<String>,

    /// run N jobs in parallel
    #[structopt(short = "-j", default_value, name = "N")]
    pub parallelism: NumCpus,
}

pub fn run(config: Config) -> anyhow::Result<()> {
    if let Some(dir) = config.execution_dir {
        std::env::set_current_dir(&dir).with_context(|| format!("changing to {} for -C", &dir))?;
    }

    ninja_metrics::enable();
    let start = "build.ninja";
    let input = std::fs::read(start).expect("build.ninja");
    let ast = {
        scoped_metric!("parse");
        // TODO: Better error.
        // 0. pulling in subninja and includes with correct scoping.
        // TODO
        Parser::new(&input, Some(start.to_owned())).parse()?
    };
    let ast = {
        scoped_metric!("analyze");
        // seems like each metric costs 3ms to set up :(
        to_description(ast)?
    };

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
        description_to_tasks(ast)
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
    let scheduler = ParallelTopoScheduler::new(config.parallelism.0);
    // let start = Start::All; // TODO: filter_keys();
    //build.build(keys_to_tasks, start);
    {
        scoped_metric!("build");
        build_externals(scheduler, rebuilder, &tasks, ())?;
    }
    // build log loading later
    ninja_metrics::dump();
    Ok(())
}
