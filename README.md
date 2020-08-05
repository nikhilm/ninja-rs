# ninja-rs

ninja-rs (Like ninjas, but pronounced with an intrusive "r" ;-)) is a clone of
the [ninja](https://ninja-build.org) build system.

## Motivation

ninja-rs was created for several different reasons:

1. Education - [Build Systems a la Carte][bsp] was a hugely influential paper
   for me as it formalized and distilled build systems. This was an attempt to
   implement a real build system using the ideas in that paper. Read the notes
   on design below for more about this. Evan Martin, the original author of
   ninja, also [wishes][reflection] they had this paper around.

2. An exercise in using Rust outside of my day job for a real project.

3. I had never written a proper parser before.

ninja is a fairly simple build system (it was explicitly designed to be the
"assembly language" of build systems), so I figured it would be a
good place to start. It is relatively small, and the original code is quite
readable. It doesn't concern itself with any networking or packaging. As Evan
Martin [says][reflection]:

> Ninja is pretty easy to implement for the fun 20% of it and the remaining 80%
> is "just" some fiddly details.

It is a work in progress. As my understanding of Ninja and Rust evolved,
various things changed and continue to change.

[bsp]: https://www.microsoft.com/en-us/research/publication/build-systems-a-la-carte/

## Feature complete-ness

At this point the parser is fairly feature complete and ninja-rs is capable of
building the simple CMake hello-world in this repository. It has a long way to
go to fully support everything ninja supports.

- [X] Working parser and topological sort based builder
- [X] mtime based rebuilding
- [X] Basic command-line compatibility with Ninja
- [X] Implicit and ordered dependencies
- [X] Variables and scoping
- [ ] Handling failed commands correctly
- [ ] Pools
- [ ] Path canonicalization
- [ ] Windows support (Nothing intentionally stopping it, but not tested either.)
- [ ] Ninja log
- [ ] build file regeneration
- [ ] C compiler include parsing (`-M` for GCC/clang, `/showIncludes` for MSVC) and dependency log
- [ ] Dynamic dependencies
- [ ] Better pretty-printing
- [ ] [Extra tools](https://ninja-build.org/manual.html#_extra_tools)

## Design

The [DESIGN.md](DESIGN.md) and [parse/DESIGN.md](parse/DESIGN.md) documents
were written while I was thinking of various things and are not meant to make
sense to anyone else. This section summarizes the actual design.

This design falls out of the following goals:

### Try to stick to the primitives of the Build Systems a la Carte paper

Most production build systems have been around before this paper, and most of
them tend not to have strict boundaries between the various stages. I've tried
to make the primitives shine through when possible. For example, the rebuilder
and scheduler very explicitly use graphs, talk in terms of Keys and Tasks and
try to avoid direct I/O (or thinking in terms of real-world things like unix
times) as much as possible. There are of course limitations on this, since Rust
doesn't have the full generic/higher-kinded types support that Haskell does.
Writing excessively generic code in Rust can get old quickly if one has to
start putting trait bounds or generics everywhere. There is a first stage that
parses ninja files and translates these to a build description. This is then
transformed to a set of Keys and Tasks as in the paper. Then, there is an
implementation of the Scheduler (_topological_) and Rebuilder (_mtime based_)
that implements ninja semantics.

In some sense, this is almost a compiler pipeline for the ninja language, but
the final stage is an interpreter for the action graph instead of a code
generator.

### Try to have usable stand-alone crates

The implementation is a collection of crates. The parser can be used by itself.
The lexer preserves relatively complete information about tokens (unlike
ninja), which could allow things like a `ninja-fmt`. I originally envisioned
the parser yielding per-file ASTs, but the way ninja handles scoping across
`include` and `subninja` rules, makes this intractable. Specifically,
variables in the included file are evaluated immediately within the current
environment, so this cannot be parallelized and a per-file AST is meaningless
as soon as inclusions happen.

The main `ninja` crate simply assembles all these pieces together.
Theoretically, one could use the `tasks` and `build` crates to create another
build system. There are a bunch of unclean API boundaries right now, but those
are amenable to cleaning up.

### Enable optimizing for performance

I started off really focusing on trying to avoid allocations and copies, but
this quickly got intractable with Rust's strong ownership requirements. So I've
currently gone to the other extreme, with liberal uses of `clone()`. The lexer
is still "zero-copy", dishing out references to a single `&[u8]`.
Canonicalization and string-interning can take care of the paths and it should
be possible to go from the lexer to the task description without copying, as
long as one is willing to propagate more lifetimes around. Of course, variable
evaluation will always require allocation of new bytes.

It isn't clear yet whether this can achieve ninja's levels of performance while
maintaining readability.

### Be readable/documented

This generally falls out of sticking to the primitives, but I've also called
out bits and pieces more obviously, when it isn't clear how to translate
something from "how does ninja do it" to "how do I express this in the
primitives from the paper". A particular instance is the rebuilder's handling
of phony rules and dirtiness. Similarly, the scheduler does a very intentional
depth-first topological sort, unlike ninja, where something similar is done but
the graph building is coupled with the parsing and handling dynamic
dependencies.

### Be testable

Ninja doesn't really have a spec. It has a manual that explains behavior
broadly, but the original implementation is the only source of truth. This
means a lot of the canonical behavior can only be determined by running
specific build files through ninja. I've tried to add tests for as much of the
code base as possible. Particularly, the parser has a bunch of acceptance tests
to ensure compliance with ninja and also to act as regression tests. These
acceptance tests are just `.ninja` files, which can be quickly run with `ninja`
to determine if they are following "the spec".

[reflection]: http://neugierig.org/software/blog/2020/05/ninja.html
