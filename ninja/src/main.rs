use ninja_build::{
    build_externals, default_mtimestate, MTimeRebuilder, MTimeState, ParallelTopoScheduler,
};
use ninja_metrics::scoped_metric;
// use ninja_interface::Scheduler;
use ninja_desc::to_description;
use ninja_parse::Parser;
use ninja_tasks::description_to_tasks;
use std::time::{Duration, Instant};

fn main() {
    ninja_metrics::enable();
    let start = "build.ninja";
    let input = std::fs::read(start).expect("build.ninja");
    let ast = {
        scoped_metric!("parse");
        // TODO: Better error.
        // 0. pulling in subninja and includes with correct scoping.
        // TODO
        let result = Parser::new(&input, Some(start.to_owned())).parse();
        if let Err(e) = result {
            eprintln!("ninjars: {}", e);
            std::process::exit(1);
        }
        result.unwrap()
    };
    let ast = {
        scoped_metric!("analyze");
        let result = to_description(ast);
        if let Err(e) = result {
            eprintln!("ninjars: {}", e);
            std::process::exit(1);
        }
        result.unwrap()
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
    let scheduler = ParallelTopoScheduler::new();
    // let start = Start::All; // TODO: filter_keys();
    //build.build(keys_to_tasks, start);
    {
        scoped_metric!("build");
        if let Err(e) = build_externals(scheduler, rebuilder, &tasks, ()) {
            eprintln!("ninjars: {}", e);
            std::process::exit(1);
        }
    }
    // build log loading later
    ninja_metrics::dump();
}
