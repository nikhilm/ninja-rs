use super::{Env, Rule};
use ninja_paths::PathCache;

#[derive(Debug)]
struct BuildEdge {
    // The build edge is going to refer to paths in some shared patchcache, or at least a pathcache
    // owned by BuildDescription.
    outputs: Vec<Vec<u8>>,
    inputs: Vec<Vec<u8>>,
    command: Vec<u8>,
    // rule: ownership story
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
    pub fn new() -> BuildDescription {
        BuildDescription {
            build_edges: Vec::new(),
            paths: ninja_paths::PathCache::new(),
        }
    }

    pub fn edge_builder(&mut self) -> EdgeBuilder {
        EdgeBuilder {
            desc: self,
            outputs: None,
            inputs: None,
        }
    }
}

pub struct EdgeBuilder<'d> {
    desc: &'d mut BuildDescription,
    outputs: Option<Vec<Vec<u8>>>,
    inputs: Option<Vec<Vec<u8>>>,
}

impl<'d> EdgeBuilder<'d> {
    pub fn add_outputs<V: Into<Vec<Vec<u8>>>>(mut self, outputs: V) -> EdgeBuilder<'d> {
        self.outputs.replace(outputs.into());
        self
    }

    pub fn add_inputs<V: Into<Vec<Vec<u8>>>>(mut self, inputs: V) -> EdgeBuilder<'d> {
        self.inputs.replace(inputs.into());
        self
    }

    pub(crate) fn finish(mut self, _: &Env, rule: &Rule) {
        // TODO: Other evaluations.
        self.desc.build_edges.push(BuildEdge {
            outputs: self.outputs.unwrap(),
            inputs: self.inputs.unwrap(),
            command: rule.command.into(),
        });
    }
}
