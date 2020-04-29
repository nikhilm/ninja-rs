extern crate ninja_build;
extern crate ninja_parse;
extern crate petgraph;

use ninja_build::{BuildLog, MTimeRebuilder, TopoScheduler};
use ninja_interface::Scheduler;
use ninja_parse::Parser;
use petgraph::{graph::NodeIndex, Direction};

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
    // What we really want a pull pipeline, where a builder can take a parser, and may be a task
    // creator and create the graph.
    let (graph, tasks, path_cache) = builder.consume();
    //let mut store = DiskStore::new();
    let rebuilder = MTimeRebuilder::new(&graph, path_cache);
    let scheduler = TopoScheduler::new(&graph, tasks);
    // TODO: Find starting nodes based on user input.
    // TODO: Ideally this crate also would not depend on petgraph directly.
    let start: Vec<NodeIndex> = graph.externals(Direction::Incoming).collect();
    scheduler.schedule(&rebuilder, start);
    // build log loading later
}
