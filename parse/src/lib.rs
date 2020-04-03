#![feature(is_sorted)]
#![feature(todo_macro)]

pub mod lexer;

use lexer::Lexer;
use lexer::Token;

#[derive(Debug)]
struct Rule<'a> {
    name: &'a [u8],
    command: &'a [u8],
}

// TODO: Canonicalization pass
// var evaluation
// lifetimes and graphs in Rust

#[derive(Debug)]
struct Pool<'a> {
    name: &'a [u8],
    depth: u32,
}

#[derive(Debug)]
struct BuildEdge {
    // outputs: Vec<
// inputs:
// rule: ownership story
}

#[derive(Debug)]
struct BuildDescription<'a> {
    // environment: Env, // TODO
    rules: Vec<Rule<'a>>,
    build_edges: Vec<BuildEdge>,
    // defaults: Vec<...>, // TODO
    pools: Vec<Pool<'a>>, // TODO
}

pub struct Parser<'a, 'b> {
    lexer: Lexer<'a, 'b>,
    build_description: BuildDescription<'a>,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(input: &[u8]) -> Parser {
        Parser {
            lexer: Lexer::new(input, None, None),
        }
    }

    fn expect_identifier(&mut self) -> Token<'a> {
        if let Some(token) = self.lexer.next() {
            match token {
                Token::Identifier(_) => token,
                _ => todo!("Error handling"),
            }
        } else {
            todo!("Error handling");
        }
    }

    fn consume_indent(&mut self) -> bool {
        if let Some(token) = self.lexer.next() {
            match token {
                Token::Indent => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn expect_and_discard_newline(&mut self) {
        if let Some(token) = self.lexer.next() {
            match token {
                Token::Newline => {}
                _ => todo!("Error handling"),
            }
        }
    }

    fn read_assignment(&mut self) -> (&'a [u8], &'a [u8]) {
        let var = self.expect_identifier();
        if let Some(token) = self.lexer.next() {
            match token {
                Token::Equals => {}
                _ => todo!("Error handling"),
            }
        } else {
            todo!("Error handling");
        }

        let mut value: Option<Token<'a>> = None;
        if let Some(token) = self.lexer.next() {
            match token {
                Token::Literal(_) => {
                    value = Some(token);
                }
                _ => todo!("Error handling"),
            }
        } else {
            todo!("Error handling");
        }
        (var.value(), value.expect("value").value())
    }

    fn parse_rule(&mut self) -> Rule<'a> {
        let identifier = self.expect_identifier();
        self.expect_and_discard_newline();
        // TODO: Do all the scoping and env stuff.
        loop {
            if !self.consume_indent() {
                break;
            }
            let (var, value) = self.read_assignment();
            if var != "command".as_bytes() {
                todo!("Don't know how to handle anything except command");
            }
            return Rule {
                name: identifier.value(),
                command: value,
            };
        }

        todo!("OOPS!");
    }

    pub fn parse(&mut self) -> Result<BuildDescription<'a>> {
        while let Some(token) = self.lexer.next() {
            match token {
                Token::Rule => {
                    let rule = self.parse_rule();
                    eprintln!("Got rule {:?}", rule);
                }
                Token::Newline => {}
                _ => {
                    eprintln!("Unhandled token {:?}", token);
                }
            }
        }
    }
}

#[cfg(test)]
mod parser_test {
    use super::Parser;

    #[test]
    fn test_simple() {
        let input = r#"
rule cc
    command = gcc -c foo.c

build foo.o: cc foo.c"#;
        // TODO: The parser needs some mechanism to load other "files" when includes or subninjas
        // are encountered.
        let mut parser = Parser::new(input.as_bytes());
        parser.parse();
    }
}
