# A ninja lexer + parser

A ninja file is made of declarations. Newlines are meaningful. Whitespace is sometimes meaningful.

This means we will need to fiddle a bit with what qualifies as an individual token.

The lexer should be zero-copy, which means it should refer to bytes in the input stream/slice.

In addition, the lexer should preserve enough information about the position of tokens so that the parser and the lexer can give good error messages. This means preserving line numbers and column numbers, and then an error reporter that can refer to bits before and after to give nice error messages.

This means all tokens have a offset + span of some kind, even the single character ones.

we also probably don't want to fail (stop) on the very first error, so instead store/propagate errors to a handler.

should behave like an iterator on the other end.
