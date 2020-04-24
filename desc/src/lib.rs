extern crate ninja_paths;
extern crate petgraph;

use std::{ffi::OsStr, fs::metadata, os::unix::ffi::OsStrExt, time::SystemTime};

pub use petgraph::graph::{EdgeIndex, EdgeReference, NodeIndex};
use petgraph::{Direction, Graph};
use std::collections::{hash_map::Entry, HashMap};

use ninja_paths::{PathCache, PathRef};

pub type Command = Vec<u8>;
pub type Outputs = Vec<Vec<u8>>;
pub type Inputs = Vec<Vec<u8>>;
pub type Path = Vec<u8>;

#[derive(Debug, Clone, Copy)]
enum MTime {
    Unknown,
    DoesNotExist,
    Is(SystemTime),
}

#[derive(Debug)]
struct NodeData {
    path: Vec<u8>,
    mtime: MTime,
}

impl NodeData {
    pub fn restat(&mut self) -> MTime {
        // TODO: Disk interface.
        let path: &OsStr = OsStrExt::from_bytes(&self.path);
        match metadata(path) {
            Ok(metadata) => {
                self.mtime = MTime::Is(
                    metadata
                        .modified()
                        .expect("mtime available on platforms we support"),
                );
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    self.mtime = MTime::DoesNotExist;
                }
                _ => {
                    todo!("handle other errors");
                }
            },
        }
        self.last_modified()
    }

    pub fn last_modified(&self) -> MTime {
        self.mtime
    }
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

#[derive(PartialEq, Eq)]
enum Mark {
    Temporary,
    Permanent,
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
                            path: self.path_cache.get(input_path_ref).clone().to_vec(),
                            mtime: MTime::Unknown,
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
                            path: self.path_cache.get(output_path_ref).clone().to_vec(),
                            mtime: MTime::Unknown,
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

    pub fn command(&self, target: NodeIndex) -> Option<&Command> {
        self.input_edge(target).map(|e| &e.weight().command)
    }

    fn input_edge(&self, target: NodeIndex) -> Option<EdgeReference<'_, EdgeData>> {
        // TODO: This is kinda incorrect but works.
        // Technically we have one input edge per input in a multi-input edge, but they all share
        // the same command.
        self.graph
            .edges_directed(target, Direction::Incoming)
            .next()
    }

    pub fn build_order(&mut self) -> Vec<NodeIndex> {
        // TODO: Use command line arguments from config if passed.
        let start_nodes: Vec<NodeIndex> = self.roots().collect();

        // We could use toposort, but that would do it over the entire graph, so we use a
        // post-order traversal and yield items instead.
        // TODO: Cycle handling.

        // We reverse the normal toposort ordering since our directed graph flows in the
        // opposite direction of the sort we want.

        // Possibly Maintain our own stack coz Rust doesn't like mutability + recursion.
        //

        let mut order: Vec<NodeIndex> = Vec::new();
        let mut visited: HashMap<NodeIndex, Mark> = HashMap::new();
        for node in start_nodes {
            self.post_order_flipped_toposort(node, &mut order, &mut visited);
        }
        for node in &order {
            self.graph.node_weight_mut(*node).unwrap().restat();
        }
        order
    }

    fn post_order_flipped_toposort(
        &self,
        node: NodeIndex,
        order: &mut Vec<NodeIndex>,
        visited: &mut HashMap<NodeIndex, Mark>,
    ) {
        if let Some(mark) = visited.get(&node) {
            if *mark == Mark::Permanent {
                return;
            } else if *mark == Mark::Temporary {
                panic!("CYCLE");
            }
        }

        visited.insert(node, Mark::Temporary);

        for dep in self.graph.neighbors_directed(node, Direction::Incoming) {
            self.post_order_flipped_toposort(dep, order, visited);
        }

        visited.insert(node, Mark::Permanent);
        order.push(node);
    }

    pub fn dump_node(&self, node: NodeIndex) {
        let path = &self.graph[node].path;
        eprintln!("node {}", std::str::from_utf8(path).unwrap());
    }

    pub fn dirty(&self, node: NodeIndex) -> bool {
        let node_data = &self.graph[node];
        let node_mtime = match node_data.last_modified() {
            MTime::Is(mt) => mt,
            MTime::DoesNotExist => return true,
            _ => panic!("Unexpected"),
        };

        let mut older_than_deps = false;

        let mut it = self
            .graph
            .neighbors_directed(node, Direction::Incoming)
            .detach();
        while let Some(dep) = it.next_node(&self.graph) {
            let dep = &self.graph[dep];
            match dep.last_modified() {
                MTime::Is(mt) => {
                    if node_mtime < mt {
                        older_than_deps = true;
                        break;
                    }
                }
                _ => {
                    let path = std::str::from_utf8(&dep.path).unwrap();
                    panic!("Expected dep {} to already be built or exist on disk", path);
                }
            }
        }

        older_than_deps
    }

    pub fn mark_done(&mut self, node: NodeIndex) {
        // We can't just restat this node. Consider a multi-output node where both nodes are about
        // to be built. When the first node causes the command to be run, if we don't restat all
        // the outputs, the second one will still be considered old, leading to running the command
        // again.
        // How does one find all the outputs of the same command in a graph where there is no such
        // link?
        // OOPS! No such association can be inferred simply by going to the input side and asking
        // for its outputs, because one input can be shared by multiple build edges.
        // We would need to iterate over all outgoing edges of inputs and only find the ones that
        // have the same command as this. this is probably where a comparable (Eq), shared edge
        // data would be useful. The edge data seems like it is immutable after creation.
        todo!("Do this properly");
    }
}
