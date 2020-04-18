use std::collections::{HashMap, HashSet};

use super::{Env, ParseError, Rule};
use ninja_desc::{BuildDescription, Command, Inputs, Outputs, Path};
use ninja_paths::{InsertResult, PathCache, PathRef};

// TODO: We actually need a separate outputs cache (has to be a pathcache for
// canonicalization)
// and an inputs cache and need to pass both to the build description somehow.
// unless we just keep searching for outputs
pub struct DescriptionBuilder {
    outputs_seen: PathCache,
    desc: BuildDescription,
}

impl DescriptionBuilder {
    pub fn new() -> DescriptionBuilder {
        DescriptionBuilder {
            outputs_seen: PathCache::new(),
            desc: BuildDescription::new(),
        }
    }

    pub fn new_edge(&mut self) -> EdgeBuilder {
        EdgeBuilder::new(self)
    }

    pub fn finish(self) -> BuildDescription {
        self.desc
    }

    fn add_edge(&mut self, inputs: Inputs, outputs: Outputs, command: Command) {
        // Since we do duplicate output checks, we are guaranteed to always have unique
        // combinations.
        self.desc.add_edge(inputs, outputs, command);
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

    pub fn add_outputs<V: Into<Outputs>>(mut self, outputs: V) -> EdgeBuilder<'d> {
        self.outputs.replace(outputs.into());
        self
    }

    pub fn add_inputs<V: Into<Inputs>>(mut self, inputs: V) -> EdgeBuilder<'d> {
        self.inputs.replace(inputs.into());
        self
    }

    pub(crate) fn finish(mut self, _: &Env, rule: &Rule) -> Result<(), OutputConflict> {
        // TODO: Other evaluations.

        let outputs = self.outputs.take().expect("add_outputs called");

        // Check conflict.
        let conflict: Result<Vec<PathRef>, OutputConflict> = outputs
            .iter()
            // Need the clone because of error reporting.
            .map(
                |output| match self.desc.outputs_seen.insert(output.clone()) {
                    InsertResult::AlreadyExists(_) => Err(OutputConflict(output.to_vec())),
                    InsertResult::Inserted(path_ref) => Ok(path_ref),
                },
            )
            .collect();
        let _ = conflict?;

        let inputs = self.inputs.take().expect("add_inputs called");

        self.desc
            .add_edge(inputs, outputs, rule.command.clone().into());
        Ok(())
    }
}
