extern crate ninja_interface;
extern crate ninja_paths;
extern crate petgraph;

use std::{
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap, HashSet,
    },
    hash::Hasher,
};

use ninja_interface::Task;
use ninja_paths::{PathCache, PathRef};

mod tasks;
use tasks::{CommandTask, PhonyTask};

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Key {
    Path(PathRef),
    Multi(Vec<PathRef>),
}

pub use petgraph::graph::NodeIndex;
pub use tasks::TaskResult;
pub type BuildGraph = petgraph::Graph<Key, ()>;
pub type TasksMap = HashMap<NodeIndex, Box<dyn Task<TaskResult>>>;

// Having a pipeline with the builder consuming the AST would allow the builder to also take
// ownership of the underlying buffer and thus potentially share that, although the path cache may
// want to create new strings when it canonicalizes.
// TODO: Also add a path cache shared by the graph and everything else.
#[derive(Default, Debug)]
pub struct Builder {
    graph: BuildGraph,
    tasks: TasksMap,
    // The path cache becomes read-only once we cede ownership, so both the build graph and tasks
    // can maintain references to this as required.
    path_cache: PathCache,
    // Deals only with outputs as inputs are allowed to duplicate.
    outputs_seen: HashSet<u64>,
    // Key to NodeIndex map so we never recreate a node for a path we have seen.
    node_map: HashMap<Key, NodeIndex>,
}

pub struct OutputConflict<'a>(pub &'a [u8]);

impl Builder {
    pub fn add_edge<'a>(
        &mut self,
        inputs: Vec<&[u8]>,
        outputs: Vec<&'a [u8]>,
        command: &[u8],
    ) -> Result<(), OutputConflict<'a>> {
        // Check conflict.
        let conflict: Result<(), OutputConflict> = outputs
            .iter()
            // Need the clone because of error reporting.
            .map(|output| {
                let mut hasher = DefaultHasher::new();
                hasher.write(output);
                let hash = hasher.finish();
                if self.outputs_seen.contains(&hash) {
                    Err(OutputConflict(output))
                } else {
                    self.outputs_seen.insert(hash);
                    Ok(())
                }
            })
            .collect();
        let _ = conflict?;

        let output_refs: Vec<_> = outputs
            .into_iter()
            .map(|output| self.path_cache.insert_and_get(output))
            .collect();

        let input_refs: Vec<_> = inputs
            .into_iter()
            .map(|input| self.path_cache.insert_and_get(input))
            .collect();

        let output_node = self.node_for(Builder::make_key(&output_refs));

        let task: CommandTask = CommandTask::new(command.clone());
        self.tasks.insert(output_node, Box::new(task));
        for input in input_refs {
            let input_node = self.node_for(Builder::make_key_single(input));
            self.graph.add_edge(output_node, input_node, ());
        }

        // If outputs is of multiple type, add phonies.
        if output_refs.len() > 1 {
            for output in output_refs {
                let phony_node = self.node_for(Builder::make_key_single(output));
                let task: PhonyTask = Default::default();
                self.tasks.insert(phony_node, Box::new(task));
                self.graph.add_edge(phony_node, output_node, ());
            }
        }

        Ok(())
    }

    fn make_key_single(path: PathRef) -> Key {
        Key::Path(path)
    }

    fn make_key(paths: &[PathRef]) -> Key {
        if paths.len() == 1 {
            Key::Path(paths[0])
        } else {
            let owned: Vec<PathRef> = paths.clone().to_owned();
            Key::Multi(owned)
        }
    }

    pub fn consume(self) -> (BuildGraph, TasksMap, PathCache) {
        (self.graph, self.tasks, self.path_cache)
    }

    fn node_for(&mut self, key: Key) -> NodeIndex {
        // Don't want to clone every time, only when we need to insert.
        match self.node_map.entry(key.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let node = self.graph.add_node(key);
                e.insert(node);
                node
            }
        }
    }
}
