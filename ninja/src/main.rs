extern crate ninja_parse;
extern crate petgraph;

// use ninja_build::{MTimeRebuilder, ParallelTopoScheduler};
// use ninja_interface::Scheduler;
use ninja_desc::to_description;
use ninja_parse::Parser;

fn main() {
    let start = "build.ninja";
    let input = std::fs::read(start).expect("build.ninja");
    let ast = {
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
        let result = to_description(ast);
        if let Err(e) = result {
            eprintln!("ninjars: {}", e);
            std::process::exit(1);
        }
        result.unwrap()
    };
    eprintln!("ast {:?}", ast);

    // // at this point we should basically have a structure where all commands are fully expanded and
    // // ready to go.
    // let keys_to_tasks = ninja_task_creator(ast);

    // Ready to build.
    // let _state = BuildLog::read();
    //let mut store = DiskStore::new();
    // TODO: This can all hide behind the build constructor right?
    // So this could be just a function according to the paper, as long as it followed a certain
    // signature. Fn(k, v, task) -> Task
    // let rebuilder = MTimeRebuilder::new(mod_times_oracle);
    // let scheduler = ParallelTopoScheduler::new();
    // // Made up, we likely don't want to go as Fn()y as haskell.
    // let build = scheduler.to_build(rebuilder);
    // let build = NinjaBuild::new(mod_times_oracle);
    // let start = Start::All; // TODO: filter_keys();
    //build.build(keys_to_tasks, start);
    // build log loading later
}
