use std::collections::{HashMap, HashSet};

use super::{Env, ParseError, Rule};
use ninja_desc::{BuildDescription, Command, Inputs, NodeIndex, Outputs};
use ninja_paths::{InsertResult, PathCache, PathRef};

pub struct DescriptionBuilder {
    path_cache: PathCache,
    build_edges: HashMap<(PathRef, PathRef), Command>,
}

impl DescriptionBuilder {
    pub fn new() -> DescriptionBuilder {
        DescriptionBuilder {
            path_cache: PathCache::new(),
            build_edges: HashMap::new(),
        }
    }

    pub fn new_edge(&mut self) -> EdgeBuilder {
        EdgeBuilder::new(self)
    }

    pub fn finish(self) -> BuildDescription {
        // TODO: Convert to description.
        // This involves creating a structure that allows easy lookup from targets to their
        // edges and inputs, so a node indexed graph.
        // nmatsakis' representation seems reasonable enough.
        // In addition we need to associate data with
        // nodes - the path they represent
        // edges - the command (and later more)
        //
        // edges in some sense represent Tasks, where Tasks can actually be shared across edges,
        // but that is an optimization to do later.
        // Well, tasks can only be shared if you don't expand `command` at parse time and then do
        // interpolations at runtime. which probably isn't smart to start of with.
        //
        // BuildDescription::new(self.build_edges)

        let path_range = self.path_cache.iter_refs();
        let mut desc = BuildDescription::new(self.path_cache);

        let node_indices: Vec<NodeIndex> = path_range
            .map(|path_index| desc.add_node(path_index))
            .collect();

        // Due to the PathRef to NodeIndex invariant, this is valid.
        for ((out, in_), command) in self.build_edges.into_iter() {
            desc.add_command(node_indices[out], command);
            desc.add_edge(node_indices[out], node_indices[in_]);
        }

        desc
    }

    fn add_edge(&mut self, output: PathRef, input: PathRef, command: Command) {
        // Since we do duplicate output checks, we are guaranteed to always have unique
        // combinations.
        self.build_edges.insert((output, input), command);
    }
}

pub struct OutputConflict(pub(crate) Vec<u8>);

pub struct EdgeBuilder<'d> {
    desc: &'d mut DescriptionBuilder,
    outputs: Option<Outputs>,
    inputs: Option<Inputs>,
}

impl<'d> EdgeBuilder<'d> {
    pub fn new(desc: &mut DescriptionBuilder) -> EdgeBuilder {
        EdgeBuilder {
            desc: desc,
            outputs: None,
            inputs: None,
        }
    }

    pub fn add_outputs<V: Into<Vec<Vec<u8>>>>(mut self, outputs: V) -> EdgeBuilder<'d> {
        self.outputs.replace(outputs.into());
        self
    }

    pub fn add_inputs<V: Into<Vec<Vec<u8>>>>(mut self, inputs: V) -> EdgeBuilder<'d> {
        self.inputs.replace(inputs.into());
        self
    }

    pub(crate) fn finish(mut self, _: &Env, rule: &Rule) -> Result<(), OutputConflict> {
        // TODO: Other evaluations.

        let output_references: Result<Vec<PathRef>, OutputConflict> = self
            .outputs
            .take()
            .expect("add_outputs called")
            .into_iter()
            // Need the clone because of error reporting.
            .map(|output| match self.desc.path_cache.insert(output.clone()) {
                InsertResult::AlreadyExists(_) => Err(OutputConflict(output)),
                InsertResult::Inserted(node_ref) => Ok(node_ref),
            })
            .collect();

        let output_references = output_references?;

        let input_references: HashSet<PathRef> = self
            .inputs
            .take()
            .expect("add_inputs called")
            .into_iter()
            .map(|input| match self.desc.path_cache.insert(input) {
                InsertResult::AlreadyExists(node_ref) => node_ref,
                InsertResult::Inserted(node_ref) => node_ref,
            })
            .collect();

        // for all combinations, add edges.
        for output_ref in output_references {
            for input_ref in &input_references {
                self.desc
                    .add_edge(output_ref, *input_ref, rule.command.clone().into());
            }
        }
        Ok(())
    }
}
