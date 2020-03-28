/*
extern crate ninja_parse;
use ninja_parse::Rule;
fn main() {
    println!("Hello, world!");
    let results = ninja_parse::Parser::parse(
        Rule::program,
        r#"
rule foo
    command = gcc
  depfile = bar bie
"#
        .trim(),
    );
    eprintln!("{:?}", results);
}*/

use ninja_parse::Pos;
fn main() {
    // Pos(5); Will fail which is what we want.
}
