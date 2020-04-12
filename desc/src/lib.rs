extern crate ninja_paths;
use ninja_paths::{PathCache, PathRef};

pub type Outputs = Vec<Vec<u8>>;
pub type Inputs = Vec<Vec<u8>>;
pub type Command = Vec<u8>;

// Graph bits that shouldn't be exposed.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct NodeIndex(usize);
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct EdgeIndex(usize);

impl NodeIndex {
    fn index(&self) -> usize {
        self.0
    }
}

impl EdgeIndex {
    fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
struct NodeData {
    path: PathRef,
    // TODO: share this among nodes
    pub command: Option<Command>,

    // Needed to efficiently find dependencies, reachability and topo-sort.
    first_outgoing_edge: Option<EdgeIndex>,
    // Needed to allow efficiently finding targets with no incoming edges, so we know where to
    // start building from on a default invocation.
    first_incoming_edge: Option<EdgeIndex>,
}

#[derive(Debug)]
pub struct EdgeData {
    // The build edge is going to refer to paths in some shared patchcache, or at least a pathcache
    // owned by BuildDescription.
    // other things that may be specific to the edge.
    // we may want to share certain things like commands, which after evaluation may end up being
    // the same.

    // Graph bits.
    // The source of the edge is implicit in the graph.
    target: NodeIndex,
    // Next outgoing edge from implicit source.
    next_outgoing_edge: Option<EdgeIndex>,
    // Next incoming edge to target.
    next_incoming_edge: Option<EdgeIndex>,
}

#[derive(Debug)]
pub struct BuildDescription {
    // we will actually need a few alternative views on this to do things like reachability and
    // topological sorting, so not sure if this should be augmented with more details like having
    // nodes maintain incoming/outgoing edge counts and references to edges.
    paths: PathCache,
    // defaults: Vec<...>, // TODO
    // pools:

    // Graph bits:
    nodes: Vec<NodeData>,
    edges: Vec<EdgeData>,
}

// shenanigans to convince the compiler the regions are non-overlapping.
fn index_twice(
    slice: &mut [NodeData],
    a: NodeIndex,
    b: NodeIndex,
) -> (&mut NodeData, &mut NodeData) {
    assert!(
        a.index() != b.index(),
        "Self-edges are not allowed in a ninja graph!"
    );
    let smaller = std::cmp::min(a.index(), b.index());
    let split_at = smaller + 1;
    let (head, tail) = slice.split_at_mut(split_at);
    let smaller_ref = &mut head[smaller];

    // Return in the order expected by the caller.
    if smaller == a.index() {
        (smaller_ref, &mut tail[b.index() - split_at])
    } else {
        assert_eq!(smaller, b.index());
        (&mut tail[a.index() - split_at], smaller_ref)
    }
}

impl BuildDescription {
    pub fn new(path_cache: PathCache) -> BuildDescription {
        let node_cap = path_cache.iter_refs().end;
        BuildDescription {
            // edges: build_edges,
            paths: path_cache,

            nodes: Vec::with_capacity(node_cap),
            edges: Vec::new(),
        }
    }

    // The invariant to preserve is that the PathRef and NodeIndex have to be the same!
    pub fn add_node(&mut self, path: PathRef) -> NodeIndex {
        assert_eq!(
            self.nodes.len(),
            path,
            "Path and node index must be the same!"
        );
        let node_index = NodeIndex(self.nodes.len());
        let node = NodeData {
            path: path,
            command: None,
            first_outgoing_edge: None,
            first_incoming_edge: None,
        };
        self.nodes.push(node);
        node_index
    }

    pub fn add_edge(&mut self, source: NodeIndex, dest: NodeIndex) -> EdgeIndex {
        assert!(
            source.index() != dest.index(),
            "Self-edges are not allowed in a ninja graph!"
        );
        let edge_index = EdgeIndex(self.edges.len());
        let (mut source_data, mut dest_data) = index_twice(&mut self.nodes, source, dest);
        let edge = EdgeData {
            target: dest,
            next_outgoing_edge: source_data.first_outgoing_edge,
            next_incoming_edge: dest_data.first_incoming_edge,
        };
        source_data.first_outgoing_edge = Some(edge_index);
        dest_data.first_incoming_edge = Some(edge_index);

        self.edges.push(edge);
        edge_index
    }

    pub fn add_command(&mut self, source: NodeIndex, cmd: Command) {
        // better error handling.
        let mut node_data = &mut self.nodes[source.index()];
        assert!(node_data.command.is_none());
        node_data.command.replace(cmd);
    }

    pub fn command(&self, source: NodeIndex) -> Option<&Command> {
        self.nodes[source.index()].command.as_ref()
    }

    pub fn nodes_with_no_incoming_edges(&self) -> impl std::iter::Iterator<Item = NodeIndex> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(idx, node)| node.first_incoming_edge.is_none())
            .map(|(idx, node)| NodeIndex(idx))
    }

    // May finally be time to use the petgraph crate.
    // TODO: Return some wrapper instead of EdgeIndex
    pub fn direct_dependencies(
        &self,
        source: NodeIndex,
    ) -> impl std::iter::Iterator<Item = NodeIndex> + '_ {
        let first = self.nodes[source.index()].first_outgoing_edge;
        Successors {
            graph: self,
            current_edge_index: first,
        }
    }

    pub fn target(&self, edge: EdgeIndex) -> NodeIndex {
        self.edges[edge.index()].target
    }
}

pub struct Successors<'graph> {
    graph: &'graph BuildDescription,
    current_edge_index: Option<EdgeIndex>,
}

impl<'graph> Iterator for Successors<'graph> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<NodeIndex> {
        match self.current_edge_index {
            None => None,
            Some(edge_num) => {
                let edge = &self.graph.edges[edge_num.index()];
                self.current_edge_index = edge.next_outgoing_edge;
                Some(edge.target)
            }
        }
    }
}
