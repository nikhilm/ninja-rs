/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use ninja::{run, Config, DebugMode};

fn read_debug_modes(args: &mut pico_args::Arguments) -> anyhow::Result<Vec<DebugMode>> {
    let mut debug_modes: Vec<DebugMode> = Vec::new();
    while let Some(debug_mode) = args.opt_value_from_str("-d")? {
        if debug_mode == DebugMode::List {
            eprintln!(
                r#" debugging modes:
  stats        print operation counts/timing info
  explain      explain what caused a command to execute
  keepdepfile  don't delete depfiles after they're read by ninja
  keeprsp      don't delete @response files on success
multiple modes can be enabled via -d FOO -d BAR"#
            );
            std::process::exit(1);
        }
        debug_modes.push(debug_mode);
    }
    Ok(debug_modes)
}

fn print_usage() {
    let called_as = std::env::args().next();
    eprintln!(
        r#"usage: {} [options] [targets...]

if targets are unspecified, builds the 'default' target (see manual).

options:
  --version  print ninjars version ("{}")

  -C DIR   change to DIR before doing anything else
  -f FILE  specify input build file [default=build.ninja]

  -j N     run N jobs in parallel [default={}, derived from CPUs available]

  -d MODE  enable debugging (use -d list to list modes)
    "#,
        called_as.as_deref().unwrap_or("ninjars"),
        env!("CARGO_PKG_VERSION"),
        num_cpus::get() + 1,
    );
}

fn main() -> anyhow::Result<()> {
    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        print_usage();
        std::process::exit(1);
    }
    if args.contains("--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let config = Config {
        execution_dir: args.opt_value_from_str("-C")?,
        parallelism: args
            .opt_value_from_str("-j")?
            .unwrap_or_else(|| num_cpus::get() + 1),
        build_file: args
            .opt_value_from_str("-f")?
            .unwrap_or("build.ninja".to_owned()),
        debug_modes: read_debug_modes(&mut args)?,
        targets: args.free()?,
    };

    run(config)
}
