fn main() {
    let start = "build.ninja";
    {
        let input = std::fs::read(start).expect("build.ninja");
        let description = Parser::new(input).parse();
    }
    let state = BuildLog::read();
    let rebuilder = Rebuilder::new(state);
    let scheduler = Scheduler::new(description, rebuilder);
    scheduler.run();
    // scheduler + rebuilder creation
    // build log loading later
    // use the parser
}
