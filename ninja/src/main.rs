use structopt::StructOpt;

use ninja::{run, Config};

fn main() -> anyhow::Result<()> {
    let config = Config::from_args();
    run(config)
}
