use ninja_paths::PathCache;

pub type Outputs = Vec<Vec<u8>>;
pub type Inputs = Vec<Vec<u8>>;
pub type Command = Vec<u8>;

#[derive(Debug)]
pub struct BuildEdge {
    // The build edge is going to refer to paths in some shared patchcache, or at least a pathcache
    // owned by BuildDescription.
    outputs: Outputs,
    inputs: Inputs,
    command: Command,
    // rule: ownership story
}

impl BuildEdge {
    pub fn new(outputs: Outputs, inputs: Inputs, command: Command) -> BuildEdge {
        BuildEdge {
            outputs: outputs,
            inputs: inputs,
            command: command,
        }
    }
}

#[derive(Debug)]
pub struct BuildDescription {
    // environment: Env, // TODO
    // TODO: If you think about it, the rules aren't really relevant beyond the parser. a build
    // description is just a graph of BuildEdges + Path nodes because once the graph is ready, all
    // rules simply give their properties to the edges.
    //
    // In addition, build descriptions have pools that do matter at execution time.
    // rules: Vec<Rule>, // hashtable?
    build_edges: Vec<BuildEdge>,
    paths: PathCache,
    // defaults: Vec<...>, // TODO
    // pools:
}

impl BuildDescription {
    pub fn new(build_edges: Vec<BuildEdge>) -> BuildDescription {
        BuildDescription {
            build_edges: build_edges,
            paths: ninja_paths::PathCache::new(),
        }
    }
}
