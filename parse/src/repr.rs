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

use std::collections::HashSet;

// Paths are canonicalized and mapped to a cache
// Rules are interned into indices.
// This actually needs to come after the variable evaluation pass.
#[derive(Debug, Default)]
pub struct Description {
    // will have things like pools and minimum ninja version and defaults and so on.
    pub builds: Vec<Build>,
    pub defaults: Option<HashSet<Vec<u8>>>,
}

#[derive(Debug)]
pub enum Action {
    Phony,
    Command(String),
}

#[derive(Debug)]
pub struct Build {
    pub action: Action,
    pub inputs: Vec<Vec<u8>>,
    pub implicit_inputs: Vec<Vec<u8>>,
    pub order_inputs: Vec<Vec<u8>>,
    pub outputs: Vec<Vec<u8>>,
}
