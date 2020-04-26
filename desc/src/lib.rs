extern crate ninja_interface;
extern crate ninja_paths;
extern crate petgraph;

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::Hasher,
};

use ninja_interface::Task;
use ninja_paths::PathRef;

mod tasks;
use tasks::{CommandTask, PhonyTask, TaskResult};

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Key {
    // Path(PathRef),
    // Multi(Vec<PathRef>),
    Path(Vec<u8>),
    Multi(Vec<Vec<u8>>),
}

pub type BuildGraph = petgraph::Graph<Key, ()>;
pub type TasksMap = HashMap<Key, Box<dyn Task<TaskResult>>>;

// TODO: Also add a path cache shared by the graph and everything else.
#[derive(Default, Debug)]
pub struct Builder {
    graph: BuildGraph,
    tasks: TasksMap,
    outputs_seen: HashSet<u64>,
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

        let output_node = self.graph.add_node(Builder::make_key(&outputs));
        // TODO: Look up existing nodes!
        for input in inputs {
            let input_node = self.graph.add_node(Builder::make_key_single(input));
            self.graph.add_edge(output_node, input_node, ());
        }

        // If outputs is of multiple type, add phonies.
        if outputs.len() > 1 {
            for output in outputs {
                let phony_node = self.graph.add_node(Builder::make_key_single(output));
                self.graph.add_edge(phony_node, output_node, ());
            }
        }

        Ok(())
    }

    fn make_key_single(path: &[u8]) -> Key {
        Key::Path(path.clone().to_owned())
    }

    fn make_key(paths: &[&[u8]]) -> Key {
        if paths.len() == 1 {
            Key::Path(paths[0].clone().to_owned())
        } else {
            let owned: Vec<Vec<u8>> = paths.iter().map(|v| v.clone().to_owned()).collect();
            Key::Multi(owned)
        }
    }

    pub fn consume(self) -> (BuildGraph, TasksMap) {
        (self.graph, self.tasks)
    }
}
