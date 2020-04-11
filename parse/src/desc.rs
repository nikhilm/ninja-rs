use super::{Env, Rule};
use ninja_desc::{BuildDescription, BuildEdge, Inputs, Outputs};

pub struct DescriptionBuilder {
    build_edges: Vec<BuildEdge>,
}

impl DescriptionBuilder {
    pub fn new() -> DescriptionBuilder {
        DescriptionBuilder {
            build_edges: Vec::new(),
        }
    }

    pub fn new_edge(&mut self) -> EdgeBuilder {
        EdgeBuilder::new(self)
    }

    pub fn finish(self) -> BuildDescription {
        BuildDescription::new(self.build_edges)
    }
}

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

    pub(crate) fn finish(self, _: &Env, rule: &Rule) {
        // TODO: Other evaluations.
        self.desc.build_edges.push(BuildEdge::new(
            self.outputs.expect("outputs populated"),
            self.inputs.expect("inputs populated"),
            rule.command.into(),
        ));
    }
}
