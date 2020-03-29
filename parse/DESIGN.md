# A ninja lexer + parser

A ninja file is made of declarations. Newlines are meaningful. Whitespace is sometimes meaningful.

This means we will need to fiddle a bit with what qualifies as an individual token.

The lexer should be zero-copy, which means it should refer to bytes in the input stream/slice.

In addition, the lexer should preserve enough information about the position of tokens so that the parser and the lexer can give good error messages. This means preserving line numbers and column numbers, and then an error reporter that can refer to bits before and after to give nice error messages.

This means all tokens have a offset + span of some kind, even the single character ones.

we also probably don't want to fail (stop) on the very first error, so instead store/propagate errors to a handler.

should behave like an iterator on the other end.
Now the problem with this iterator model is that certain parts of the lexer depend on parsing context. i.e. right after `build` we want to read a path like string, right after `=` we want to read a string literal. both of these have different character set restrictions than reading an ident, which comes after a `rule` or before `=`. This may require us to switch to a "polling" model on the lexer, where the parser drives the next lex based on what is expected.
The other option is to have the lexer keep track of this state.
