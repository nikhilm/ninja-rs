extern crate ninja_parse;
extern crate pest;
use ninja_parse::Rule;
use pest::Parser;
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
}
