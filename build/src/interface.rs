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

use async_trait::async_trait;
use core::fmt::Debug;
use ninja_tasks::{Task, Tasks};

#[async_trait(?Send)]
pub trait BuildTask<State, V> {
    // Cannot pass state until we have structured concurrency.
    async fn run(&self, state: &State) -> V;

    #[cfg(test)]
    fn is_command(&self) -> bool {
        false
    }
}

impl<State, V> Debug for dyn BuildTask<State, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BuildTask{{}}")
    }
}

pub trait Rebuilder<K, V, State, RebuilderError> {
    fn build(
        &self,
        key: K,
        // current_value: V,
        task: &Task,
    ) -> Result<Option<Box<dyn BuildTask<State, V>>>, RebuilderError>;
}

pub trait Scheduler<K, V, State, BuildError, RebuilderError> {
    fn schedule(
        &self,
        rebuilder: &dyn Rebuilder<K, V, State, RebuilderError>,
        state: State,
        tasks: &Tasks,
        start: Vec<K>,
    ) -> Result<(), BuildError>;
    fn schedule_externals(
        &self,
        rebuilder: &dyn Rebuilder<K, V, State, RebuilderError>,
        state: State,
        tasks: &Tasks,
    ) -> Result<(), BuildError>;
}
