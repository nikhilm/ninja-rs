extern crate ninja_build;
extern crate ninja_parse;

use ninja_build::{BuildLog, Rebuilder, Scheduler};
use ninja_parse::Parser;

fn main() {
    let start = "build.ninja";
    let builder = {
        // TODO: Better error.
        let input = std::fs::read(start).expect("build.ninja");
        let result = Parser::new(&input, Some(start.to_owned())).parse();
        if let Err(e) = result {
            eprintln!("ninjars: {}", e);
            return;
        }
        result.unwrap()
    };
    let state = BuildLog::read();
    // Tasks yields a ninja specific set of traits
    // If we had an intermediate AST, we could break the parser's dependency on this.
    let (graph, tasks) = builder.consume();
    let mut store = DiskStore::new();
    let rebuilder = Rebuilder::new(&graph, tasks, store);
    let scheduler = NinjaTopoScheduler::new(&graph, rebuilder);
    scheduler.run();
    // build log loading later
}
