/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use super::env::Env;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Term {
    Literal(Vec<u8>),
    Reference(Vec<u8>),
}

#[derive(Debug)]
pub struct Expr(pub Vec<Term>);

impl Expr {
    pub fn eval(&self, env: &Env) -> Vec<u8> {
        let mut result = Vec::new();
        for term in &self.0 {
            match term {
                Term::Literal(bytes) => result.extend_from_slice(bytes),
                Term::Reference(name) => {
                    result.extend(env.lookup(name.as_slice()).unwrap_or_default());
                }
            }
        }
        result
    }

    pub fn eval_for_build(&self, env: &Env, rule: &Rule) -> Vec<u8> {
        let mut result = Vec::new();
        for term in &self.0 {
            match term {
                Term::Literal(bytes) => result.extend_from_slice(bytes),
                Term::Reference(name) => {
                    result.extend(
                        env.lookup_for_build(rule, name.as_slice())
                            .unwrap_or_default(),
                    );
                }
            }
        }
        result
    }
}

#[derive(Debug)]
pub struct Rule {
    pub name: Vec<u8>,
    pub bindings: HashMap<Vec<u8>, Expr>,
}

#[derive(Debug, Default)]
pub struct Build {
    pub rule: Vec<u8>,
    // These will become structs once we discriminate inputs and outputs.
    pub inputs: Vec<Expr>,
    pub implicit_inputs: Vec<Expr>,
    pub order_inputs: Vec<Expr>,
    pub outputs: Vec<Expr>,
    pub bindings: Env,
    // ...
}
