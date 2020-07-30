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

### Reconciling the Task abstraction with ninja's build edges

As discussed in section 6.7 of the paper, supporting multiple outputs requires that the "key" is a multi-value pair and materializing (or getting the value of a key) requires building all of them. Since ninja build edges do declare multiple outputs, we need something like this. that also brings the question of where to store edge things like the command.

In the paper, a task has the implementation details (which describes how to bring the key up to date), a key, which describes what thing it brings up to date and a value which is a value produced. We don't actually need to bring the "value" (file contents) into the build system. what is important is the value on disk is updated when a task is executed. This means our Task's (i.e a disk-backed ninja implementation) would have a type of Unit.

The DAG between inputs and outputs doesn't strictly need to carry build information (Tasks) itself, as long as the rebuilder/scheduler can go from a required output to the task that will produce it. If we use petgraph we would stick it in the Node and Edge info as structs.

So, the parser would maintain a list of inputs and outputs, particularly so it can do duplicate output detection, but after that the conversion to a BuildDescription would add a multitude of outputs to a single DAG node. That does bring up the problem of finding zero-incoming-edges and so on, as, strictly speaking, a ninja user can request only one target out of the many for a build edge, and then one would have to go search through the nodes. similarly, if we want to mark edges as dirty as nodes become dirty, we want to be able to query outgoing edges on a per-file basis. So we have some hybrid structure where "the same" edges go between multiple inputs and outputs, and for that we may want to do some interesting sharing over PetGraph or need our own graph.

Also, I'm not really convinced that ninja is just topological sort any more. Actually, it _is_ topo-sorting but not doing a progressive "ok, find nodes, run a topo-sort, now add the list to the plan and start executing it". Instead, as Plan::AddTarget is called, it ends up running a DFS on the dependencies and add the edges to the ready_ list, which is what topo-sort is (post-order traversal). Then the plan starts executing it. I don't know why they wouldn't just call it topo-sort in the code! So the thing they are doing differently is rather than having a key (Node) to task (Edge) mapping that is accessed every time, they just add tasks to the topo-sort list and the task knows its inputs and outputs as expected.

It is possible that our insistence on path caching so early in the design is complicating it.
In addition, since commands are unlikely to be shared between edges, there isn't really a command pool/sharing needed.

In addition, we may only want to create an actual topo-sorted thing only for nodes we deem necessary as being reachable from the targets we want. This can be done by constructing the full graph and then simply only adding things to the scheduler that are needed. So the scheduler sees a sequence of nodes or edges that don't necessarily reflect the entire graph.

**Update 2020-04-26** ninja/src/bin/model.rs models abstractions that line up well with the paper (minus deriving dependencies by calling fetch, as task is never applicative) while preserving the behavior we want. The take-aways compared to the C++ ninja implementation are:
1. Modeling the nodes and edges as true nodes and edges, i.e., if a target has multiple inputs, each input has an edge from the target to the input, allows us to leverage an existing graph library like petgraph, instead of the C++ impl where the "graph" isn't really a graph, because multiple inputs actually share the same Edge object. This allows us to leverage petgraph and its algorithms and data structures, which is partly nicer, partly necessary in Rust due to the multitude of iterators etc. we would otherwise need to write to maintain borrow checker requirements.
2. Modeling the scheduler as a true topo-sort will probably not allow dyndep kind of things right now, but it is an acceptable trade-off. We are leveraging DfsPostOrder to do the sorting so we do not iterate the entire graph. This means we may be able to simply extend that by "pausing" the post order, or extending it or something when a dependency is discovered. That can be looked at later.
3. In our graph representation, the directed edges have the source as the target and the sink as the inputs so we can work with DfsPostOrder.
4. Not keeping "mutable across a session" and "immutable across a session" fields together, as encouraged by the paper abstractions, but not by the C++ impl (where things like mtime are on the Node), may be advantageous in Rust's more restrictive rules, in terms of easily sharing things like the `Graph` across scheduler and rebuilder with simple shared references, since we will never change the graph, while keeping something like the disk interface and mtime separate and using synchronization mechanisms as necessary on it.

We really need an ergonomic Node/Key sharing mechanism that allows using a shared reference/index across the graph, the mtime check and the tasks lookup. Key will likely also refer to Paths internally, so there are multiple levels of indirection to make it nice.

In such a setup, the "builder" comes back into play as the parser should simply feed it edges and have the builder keep track of and produce a:
1. build graph, where node data is simply keys and edge data is unit.
2. a tasks lookup function.
3. a store/state abstraction that is used for mutable bits (disk access and mtimes primarily)

These 3 things can then be transferred to the scheduler and rebuilder. The scheduler needs only the graph, the rebuilder needs all 3.
If we were to have the parser emit an AST instead of using the builder directly, that gap can be bridged in main.

## Questions of concurrency

Since we eventually want concurrent job execution, we need to make sure our designs don't lock us into a space where supporting something like that is difficult.

## On include and subninja

The ninja design (by empirical observation) scopes rules but not build edges. What I mean is, a parse beginning at the main (typically build.ninja) file eventually resolves into one canonical set of build edges. This main file can include other ninja files with `include` or `subninja`. The former operates like a C include and "pulls" the other file into this scope. Since rules "have scope"/"are scoped to a file", redefining a rule with the same name in this case will error. With a `subninja`, this will not error as they are 2 different rules. On the other hand, build edges are identified purely by inputs and outputs, and those inputs/outputs are resolved relative to the ninja file. If those end up evaluating to the same outputs, ninja will warn (and we fail, but probably shouldn't later). A parsing model needs to account for this. We may want the parser to just produce an AST with a Include node that then has a full AST of the included file, and similarly for subninja. As long as this does not break variable evaluation at the top level, and rule inference. Then we can have the canonicalization pass handle rule-duplication correctly based on the type.

~It is mandatory that an include is processed when found (and not in a future pass) because the values of variables "at that instant" matter.~
That is not true. Running an example through ninja shows that build edges are evaluated after everything is parsed, so that top-level bindings use their last assigned value even in includes.

i.e.

```
  # trial.ninja
rule echo
    command = echo $a

a = 2
include trial_include.ninja
build bar: echo
a = 3
```

```
  # trial_include.ninja
build foo: echo
```

Running `ninja -f trial.ninja` will print 3 for both build edges. So a multi-pass architecture can still be preserved.

OOPS! NOT SO FAST! What I said above is wrong. Because top-level bindings are expanded immediately in all ninja files, we DO need to immediately parse and expand a file as soon as an include is encountered. Consider

```
  # trial.ninja
rule echo
    command = echo $buildvar

a = 2
include trial_include.ninja
a = 3
build bar: echo
    buildvar = $a
```

```
  # trial_include.ninja
b = $a
build foo: echo
    buildvar = $b
```

Here, `buildvar = 2` for foo and `buildvar = 3` for bar.

This implies we have to move to a state based parse model, where the parser is aware of the state and keeps updating it, including performing evaluation. Thus a parser cannot really produce an AST since certain lexemes cause side effects at parse time. Parsing each file independently is also not possible.

Given all this, the parser is the one that should present some kind of Session API to start an entire "N .ninja files" parse. There is still a bunch of "parse" vs "canonicalize" kind of stuff, such that it may make sense to have a distinction between parsing and processing internally, but the two will need to inter-op. In particular, since top-level and build-edge level variables are immediately expanded, but rule variables are expanded "later" in the scope of the build-edge, reading bindings in a build-edge leads to expansion immediately (and so build edges have an Env from Vec<u8> to Vec<u8>) while rules still have an Env from Vec<u8> to Expr that will be evaluated "later". This also means we may not want rule bindings to keep around the entire parsed bytes, since we may want to deallocate those by the time we start running commands.

We should also add a test for re-binding variables that previously were expanded. Something like:

```
a = 1
b = number_${a}  # Should be number_1
a = 2
c = number_${a}  # Should be number_2
```

### Defaults are not exactly defaults

Before ninja starts interpreting the user requests, it specifically does a lookup in the build description for an edge whose output is the main build file itself. If this is found, it is run first, allowing it to be updated. Then, ninja will run while respecting `default`, which explains why CMakeLists.txt changes are incorporated even though the `default` list in the generated build.ninja has no dependency on the build.ninja itself.
