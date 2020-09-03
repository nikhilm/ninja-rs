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

use crate::task::{Task, Tasks};
use async_trait::async_trait;
use core::fmt::Debug;

#[async_trait(?Send)]
pub trait BuildTask<V> {
    // Cannot pass state until we have structured concurrency.
    async fn run(&self) -> V;
}

impl<V> Debug for dyn BuildTask<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BuildTask{{}}")
    }
}

pub trait Rebuilder<K, V> {
    type Task: BuildTask<V> + ?Sized;
    type Error: std::error::Error + Send + Sync + 'static;
    fn build(
        &self,
        key: K,
        current_value: Option<V>,
        task: &Task,
    ) -> Result<Option<Box<Self::Task>>, Self::Error>;
}

/*impl<T> BuildTask<V> for Option<T> where T: BuildTask<V> {
    async fn run(&self) -> V {
    }
}*/

pub trait Scheduler<K, V> {
    type Error: std::error::Error + Send + Sync + 'static;
    fn schedule(
        &self,
        rebuilder: &impl Rebuilder<K, V>,
        tasks: &Tasks,
        start: Vec<K>,
    ) -> Result<(), Self::Error>;

    fn schedule_externals(
        &self,
        rebuilder: &impl Rebuilder<K, V>,
        tasks: &Tasks,
    ) -> Result<(), Self::Error>;
}
