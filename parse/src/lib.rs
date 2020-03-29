#![feature(is_sorted)]
#![feature(todo_macro)]

pub mod lexer;

use lexer::Lexer;

#[derive(Debug)]
pub enum AstNode {
    Description(Vec<AstNode>),
    Build,
}

pub struct Parser<'a, 'b> {
    lexer: Lexer<'a, 'b>,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(input: &[u8]) -> Parser {
        Parser {
            lexer: Lexer::new(input, None, None),
        }
    }

    pub fn parse(&mut self) -> AstNode {
        let children = Vec::new();
        for token in &mut self.lexer {}
        AstNode::Description(children)
    }
}

#[cfg(test)]
mod parser_test {
    use super::Parser;

    #[test]
    fn test_simple() {
        let input = "build foo.o: cc foo.c";
        // TODO: The parser needs some mechanism to load other "files" when includes or subninjas
        // are encountered.
        let mut parser = Parser::new(input.as_bytes());
        let ast = parser.parse();
        eprintln!("{:?}", ast);
    }
}
