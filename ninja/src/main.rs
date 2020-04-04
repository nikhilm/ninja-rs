extern crate ninja_parse;

use ninja_parse::Parser;

fn main() {
    let start = "build.ninja";
    let description = {
        // TODO: Better error.
        let input = std::fs::read(start).expect("build.ninja");
        let result = Parser::new(&input).parse();
        if let Err(e) = result {
            eprintln!("{}", e);
            return;
        }
        result.unwrap();
    };
    // let state = BuildLog::read();
    // let rebuilder = Rebuilder::new(state);
    // let scheduler = Scheduler::new(description, rebuilder);
    // scheduler.run();
    // scheduler + rebuilder creation
    // build log loading later
    // use the parser
}
