extern crate ninja_build;
extern crate ninja_parse;

use ninja_build::{BuildLog, Rebuilder, Scheduler};
use ninja_parse::Parser;

fn main() {
    let start = "build.ninja";
    let description = {
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
    let rebuilder = Rebuilder::new(/*state*/);
    let scheduler = Scheduler::new(description, state, rebuilder);
    scheduler.run();
    // scheduler + rebuilder creation
    // build log loading later
    // use the parser
}
