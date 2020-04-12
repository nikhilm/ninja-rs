# Ninja in Rust

An exploration in implementing ninja.

Primary motivations:

* Learn how to write a lexer and parser for a real thing.
* Learn how to write a real build system, w/o worrying about platforms etc.
* Apply ideas from "Build Systems a la carte" to increase modularity and abstraction to allow potential for combining different strategies.
* Be compatible with simple ninja files at the beginning.

## What a basic ninja looks like

A basic ninja should support:

* Rules + Build edges
* mtime based modification (if mtime differs immediately rebuild)
* If we consider the build.ninja file as an implicit input that would be a quick workaround for commands changing.

Should not immediately support, but should support in the future.

* Any of the fancy outputs and inputs styles
* Dynamic dependencies
* a build log/command line hash
* Proper stdout/stderr capturing and streaming

## building

There is some notion of a build plan, a task interface that can get certain keys up to date in some store. a store capable of "storing" mtimes, where the notion is pretty simple because the store only cares about the time right now, which is used to update the store when a key is rebuilt , and then mtimes of dependencies as they are discovered. i.e. a lookup for all input files should always have an mtime and a lookup for non-input files won't have an entry until the files materialize.

for this notion of a rebuilder, the Task implementation is pretty simple. As a build log and hashes are introduced, this gets augmented, but the fundamental "key" in this abstraction is a path, again, abstracted away, but mostly satisfied by some kind of node based cache.

## scheduling

the basic scheduler would need a graph of the build edges, where canonicalization + pathcache would lead to distinct nodes and edges. the build description created by the parser needs canonicalization for that.

Then we need a scheduler interface that is given a set of keys to bring up to date (either from the user ninja call, or derived from the graph).

So we need a query on the graph that can find starting nodes (those that are not dependencies of anything else).
In addition, we need a way to find dependencies of a given key (this is simple in an edges based thing we have now).
In addition, we need a way to run topological sort on the graph to get either a new graph or some "references" to the ordering

If we get the abstractions right, it is possible to express the `dependencies` function from the paper to gather deps without running tasks, but it doesn't give us anything since we already know the static dependencies (required by the ninja file).

Any way, the `reachable` function in the paper can be obtained by going over our graph to obtain a "subgraph". again, not sure we need to necessarily represent it as such.

So this part seems pretty reasonable once we have the path cache stuff implemented. there are some sharing issues and Rc that might affect this.

Remember to add the build.ninja file itself to the graph. It isn't clear what is a more elegant solution. adding build.ninja to the graph, or having it as one of the checks in the task's up-to-date check where more checks (like hashes) will go in the future.
