use structopt::StructOpt;

use ninja::{run, Config};

fn main() {
    let config = Config::from_args();
    if let Err(e) = run(config) {
        eprintln!("ninjars: {}", e);
        std::process::exit(1);
    }
}
