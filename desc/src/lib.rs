extern crate ninja_paths;
extern crate petgraph;

pub use petgraph::graph::{EdgeIndex, EdgeReference, NodeIndex};
use petgraph::{Direction, Graph};
use std::collections::{hash_map::Entry, HashMap};

use ninja_paths::{PathCache, PathRef};

pub type Command = Vec<u8>;
pub type Outputs = Vec<Vec<u8>>;
pub type Inputs = Vec<Vec<u8>>;
pub type Path = Vec<u8>;

#[derive(Debug)]
struct NodeData {
    path: PathRef,
}

#[derive(Debug)]
pub struct EdgeData {
    // The build edge is going to refer to paths in some shared patchcache, or at least a pathcache
    // owned by BuildDescription.
    // other things that may be specific to the edge.
    // we may want to share certain things like commands, which after evaluation may end up being
    // the same.

    // TODO: share this among nodes
    pub command: Command,
}

#[derive(Debug)]
pub struct BuildDescription {
    // we will actually need a few alternative views on this to do things like reachability and
    // topological sorting, so not sure if this should be augmented with more details like having
    // nodes maintain incoming/outgoing edge counts and references to edges.
    // defaults: Vec<...>, // TODO
    // pools:
    path_cache: PathCache,
    path_to_node: HashMap<PathRef, NodeIndex>,
    // The graph is directed with edges going in the same direction as build edges:
    // <input> ---> <output>
    //               ^
    // <input2> -----|
    //               v
    //              <output2>
    // <input3> -----^
    graph: Graph<NodeData, EdgeData>,
}

impl BuildDescription {
    pub fn new() -> BuildDescription {
        BuildDescription {
            path_cache: PathCache::new(),
            path_to_node: HashMap::new(),
            graph: Graph::new(),
        }
    }

    pub fn add_edge(&mut self, inputs: Inputs, outputs: Outputs, command: Command) {
        // TODO: Some kind of command sharing and synchronization.

        let input_path_refs: Vec<PathRef> = inputs
            .into_iter()
            .map(|input| self.path_cache.insert_and_get(input))
            .collect();
        let output_path_refs: Vec<PathRef> = outputs
            .into_iter()
            .map(|output| self.path_cache.insert_and_get(output))
            .collect();

        let input_node_indices: Vec<NodeIndex> = input_path_refs
            .into_iter()
            .map(
                |input_path_ref| match self.path_to_node.entry(input_path_ref) {
                    Entry::Occupied(e) => *e.get(),
                    Entry::Vacant(e) => {
                        let node_index = self.graph.add_node(NodeData {
                            path: input_path_ref,
                        });
                        e.insert(node_index);
                        node_index
                    }
                },
            )
            .collect();
        let output_node_indices: Vec<NodeIndex> = output_path_refs
            .into_iter()
            .map(
                |output_path_ref| match self.path_to_node.entry(output_path_ref) {
                    Entry::Occupied(e) => *e.get(),
                    Entry::Vacant(e) => {
                        let node_index = self.graph.add_node(NodeData {
                            path: output_path_ref,
                        });
                        e.insert(node_index);
                        node_index
                    }
                },
            )
            .collect();

        for input_index in input_node_indices {
            for output_index in &output_node_indices {
                self.graph.add_edge(
                    input_index,
                    *output_index,
                    EdgeData {
                        command: command.clone(),
                    },
                );
            }
        }
    }

    pub fn roots(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.externals(Direction::Outgoing)
    }

    pub fn dependencies(&self, of: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.neighbors_directed(of, Direction::Incoming)
    }

    pub fn command(&self, target: NodeIndex) -> Option<&Command> {
        self.input_edge(target).map(|e| &e.weight().command)
    }

    fn input_edge(&self, target: NodeIndex) -> Option<EdgeReference<'_, EdgeData>> {
        self.graph
            .edges_directed(target, Direction::Incoming)
            .next()
    }
}
